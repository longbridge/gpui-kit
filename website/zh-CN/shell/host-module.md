---
title: HostModule
description: Host 如何把自己的 Rust 借给脚本——注册、脚本侧的 import、纯数据边界，以及 Host function 运行时受到的约束。
order: 12
---

# HostModule

[Capabilities](./capabilities.md) 管的是脚本**不能**碰什么。这一篇讲的是另一半：Host 主动递出去的东西。

脚本无法加载 native 扩展。`dlopen` 进来的 Rust 没有稳定 ABI，而且一旦进了进程，它就持有进程的全部权限——允许这种事的沙箱等于没有沙箱。所以方向是反的：**Host 在编译期注册它愿意暴露的那部分 Rust**，脚本能够到的就只有这些，一点不多。

```rust
use gpui_kit::shell::{HostModule, HostValue};

gpui_kit::shell::export_module(
    HostModule::new("workspace")
        .function("project_name", |_| Ok(HostValue::from("gpui-component")))
        .function("version", |_| Ok(HostValue::from("0.1.0"))),
)?;
```

```js
import { project_name } from "workspace";

project_name(); // "gpui-component"
```

注册好的模块就是一个普通的 ES module，由解析 `gpui-kit` 和 `path` 的同一个 loader 负责。一次调用注册一个模块，重名会替换掉先前那个而不是合并进去——有三个模块的 Host 就调三次 `export_module`。本页余下的部分讲它的代价和它拒绝的东西。

## 为什么是 import 而不是查表

显而易见的另一种做法是 runtime registry lookup，返回一包函数：

```js
// 这里没有采用的形态。
const workspace = native("workspace");
workspace.projectName(); // 拼错了：迟早会抛
```

```js
// 实际采用的形态。
import { projectName } from "workspace"; // 拼错了：链接阶段就失败
```

它输两次，而且都输在**你什么时候才发现**上：

- **导出名拼错了要到运行时才炸。** `workspace.projectName()` 能通过类型检查、能加载、能渲染，然后在第一次真正走到它的那一帧抛出——对于只有某个分支才会碰到的名字，那一帧可能离出错的那次编辑很远。import 在模块图链接时解析，所以同样的拼写错误会在应用跑第一行之前就把它拦住，并指名模块和导出。
- **类型声明什么都说不了。** 只有 Host 知道自己注册了什么，所以 lookup 最多只能给出 `Record<string, (...args: any[]) => any>`；想要真类型的应用只能手写一份 `.d.ts`，而没有任何东西拿它跟 registry 对过账。而 module specifier 是一个类型声明**可以**写在上面的名字，所以声明[直接从 registry 生成](#给它们写类型)，拼错在编辑器里就是红的。

import **没有**冻结的是名字背后的那个函数。每个导出都是一个转发桩，每次调用都重新经过注册表，所以撤销一个模块仍然立即生效：脚本手里那个已经 import 进来的函数会得到一次拒绝，而不是那个已被收回的闭包。被固定下来的只有**名字的集合**，固定在 import 它的那个模块被链接的时刻——这也是 Host 必须**先**调用 `export_module`、再加载应用的原因。

## 注册表本身就是授权

默认注册表是**空的**，和 `Capabilities::default()` 同一个形状。什么都没注册的 Host 就是没有授予任何扩展面，脚本 import 一个模块时会被指名告知：

```text
HostModule `market` is not available: this Host registered none.
HostModule access is granted by the embedding application, with
gpui_kit::shell::export_module(...).
```

注册了东西之后，消息就变成告诉你有什么：

```text
unknown HostModule `marker`; this Host registered: market, theme
```

```text
HostModule `market` has no function `quote`; it provides: quotes, ticks, watch, watch_all
```

这上面刻意没有再叠一层"每个模块单独授权"。名单是 Host 定的，所以**名单就是授权**——撤销某一项的办法是导出一个同名模块、或者清空整个集合，下一次调用即生效，不必重启。

对于要跑多个应用的 Host，每个公开的 `Policy` 各自带着自己冻结的 capabilities 和自己的模块注册表——用 `Policy::with_host_module` 一个一个加进去，形状和上面一样。这就是同一个 runtime 里的两个插件如何拿到不同权限、而不需要在 `await` 边界上来回换 thread-local 状态。身份和申请的系统权限写在 `gpui-shell.json` 里；HostModule 不在其中，因为它是 Host 注册的可执行行为。

## runtime 自己留用的名字

HostModule 和内置模块、[Standard Runtime](./engine.md) 共用同一个 specifier 命名空间，而 resolver 先走到后两者。所以注册一个 `path` 并不会遮蔽真正的 `path`——它只会注册一个永远没人能 import 到的模块，而且悄无声息。

`export_module` 直接拒绝这样的名字，并说清楚它归谁：

```text
`path` is one of the runtime's own module names and cannot be registered: a
script importing it reaches the runtime, never this module. The reserved names
are: gpui, gpui-base, gpui-fps, buffer, console, crypto, fs/promises, net, os,
path, process, url, websocket, zlib
```

完整名单是 `gpui_kit::shell::RESERVED_SPECIFIERS`。除此之外的名字都归你——也不会被应用目录里的同名文件遮蔽，因为 HostModule 的解析顺序在应用自己的文件之前。

## 边界上只有纯数据

Host function 收到的是 `HostArguments`，返回的是 `HostValue`：null、布尔、数字、字符串、数组、对象。这六种是脚本引擎和 JSON 都能承载的交集，也正是同一份注册表能服务[引擎接缝](./engine.md)后面任意引擎的原因。

它永远不会收到脚本句柄。句柄会让 Host 把一个脚本值的引用留到产生它的那次调用之后——也留到那个让周围上下文有效的 call scope 之后。

参数按位置取出，类型检查和错误消息都是现成的：

| 调用                   | 得到                                         |
| ---------------------- | -------------------------------------------- |
| `arguments.string(0)`  | `&str`，或一个说明实际来的是什么的错误       |
| `arguments.number(0)`  | `f64`                                        |
| `arguments.integer(0)` | `i64`，拒绝带小数的数字                      |
| `arguments.boolean(0)` | `bool`                                       |
| `arguments.value(0)`   | 原始的 `HostValue`，给那些接受多种形状的函数 |
| `arguments.get(0)`     | `Option<&HostValue>`，给可选参数             |

返回一条记录用的是 builder 而不是 map，因为对象往往**就是**脚本要渲染的那一行，字段顺序应该由 Host 说了算：

```rust
use gpui_kit::shell::HostObject;

HostObject::new()
    .field("symbol", "AAPL.US")
    .field("last", 224.22)
    .field("watched", true)
```

错误是一句话，不是一个类型：`HostError::new("no such symbol")` 到了脚本那边就是一个可以 catch 的 `Error`。

## Host function 的三条规矩

**不许回调进脚本引擎。** 一次 host 调用发生在一次脚本调用里面，而后者又在一次 Host 调用里面；从这里重新进入 VM，就是在引擎栈帧还在、渲染过程还没结束的时候去跑脚本代码。不持有任何脚本句柄让这件事很难被误写出来，而 dispatcher 干脆直接拒绝嵌套调用，这样即使 Host 找到了别的路径，得到的也是一个可诊断的错误而不是未定义行为。

**读写 Host 状态才是重点。** 函数通过 `gpui_kit::shell::with_current_app` 拿到环境里的 `App`，不在一次活跃调用中时它是 `None`：

```rust
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, HostError> {
    gpui_kit::shell::with_current_app(read)
        .ok_or_else(|| HostError::new("only reachable while a script call is in progress"))
}
```

**从里面发出的 `cx.notify()` 在调用退栈之后才送达。** 所以 Host function 可以改一个 entity 并请求所有观察它的 View 重渲染，而这次重渲染不会发生在调用它的那段脚本的下面。

## 不该占住线程的活

`function` 是同步的：它返回一个值，脚本拿到那个值。慢的那种会占住渲染线程。

`async_function` 返回的是一个 future，脚本拿到的是 promise：

```rust
HostModule::new("db")
    .declarations("export function query(sql: string): Promise<Row[]>;")
    .async_function("query", |arguments| {
        // 同步的一半：在主线程上，在调用方的 scope 里。可以读 Host 状态，
        // 在这里拒绝就是在调用点抛出。
        let sql = arguments.string(0)?.to_owned();
        let pool = with_app(|cx| cx.global::<Pool>().handle())?;

        // 异步的一半：在 GPUI 的后台执行器上。
        Ok(async move { Ok(pool.query(&sql).await?.into_host_value()) })
    })
```

```js
import { query } from "db";

const rows = await query("select 1");
```

### 切成两半就是这个设计本身

闭包在主线程上跑，返回 future。所以参数检查、以及把工作需要的东西复制出来，都发生在 `with_current_app` 还答得上话的时候。之后 future 是 `Send + 'static`，在别处被驱动，那里既没有 `App` 也没有脚本引擎可碰。

这跟[上面那三条](#host-function-的三条规矩)是同一条规矩，只不过从"强制执行"变成了"物理上做不到"。同步的函数体靠一个运行时守卫被摁住"不许重进引擎"；异步的那一半根本没法把这件事表达出来，因为后台线程上没有引擎可进。

### 脚本看到什么

- **同步那一半的拒绝，在调用点抛出。** `arguments.string(0)?` 失败是写下这次调用的地方抛 `TypeError`，而不是一个要 await 才听得到的 rejected promise。
- **future 的失败会 reject 这个 promise**，消息里带着 `module.function`，所以 `await` 外面包 `try`/`catch` 就是正常写法。
- **被取消的调用会永远 pending。** View 消失、或者它的应用被重载，那么续体不会执行，也不会给一段被要求停下来的代码编造一个错误——跟 `cx.sleep` 的答案一致。

返回类型里的 `Promise` 要你自己写。注册表只核对两边的名字，不读签名，所以声明里漏掉 `Promise` 不会被任何东西抓到。

## 给它们写类型

模块在 Rust 里、紧挨着注册代码，描述自己的 TypeScript 面貌：

```rust
HostModule::new("market")
    .declarations(r#"
        /** One row of the board, as it crosses the boundary. */
        export interface Quote { symbol: string; last: string; watched: boolean }

        /** Every row on the board. */
        export function quotes(): Quote[];
        /** Flips one row's watched flag and answers the new value. */
        export function watch(symbol: string): boolean;
    "#)
    .function("quotes", /* … */)
    .function("watch", /* … */)
```

生成的 `gpui-kit.d.ts` 会把这段原样放进 `declare module "market"`，于是 `import { quotes } from "market"` 得到的检查和 `import { div } from "gpui-kit"` 完全一样。

把它写在这里、而不是脚本旁边的 `.d.ts` 里，是让两半保持为一件事的关键。`.d.ts` 会是第二个文件、第二种语言，而且没有任何东西把它绑在注册表上。`export_module` 会拿声明的导出和实际注册的对账，不一致就拒绝：

```text
HostModule `market` declares a different set of functions than it registers;
registered but not declared: quotes; declared but not registered: prices
```

现在改了一边的函数名，得到的是启动时的一句话，而不是一个还在不断补全某个 Host 早就删掉的函数的编辑器。

不写声明也可以，代价只是精度。没有声明的模块会以宽松签名生成：

```ts
declare module "audit" {
  import { HostValue } from "gpui-kit";

  export function observe(...args: HostValue[]): HostValue;
}
```

模块名和每一个导出名仍然是被检查的——而且这个形状是诚实的，因为跨越边界的东西正好就是 `HostValue`（脚本这边这个类型，就是 Rust 那边的同名类型）。写成 `any` 会比运行时更宽：脚本传一个函数过来能通过类型检查，然后在调用时被拒绝。

## 一个真实的例子

Gallery 的 Shell story 注册了一个 market 模块，这就是它那段脚本拥有的全部扩展面。主题值走的是 `cx.theme()`。 Host 侧长这样：

```rust
fn market_module(market: &Entity<Market>) -> HostModule {
    let read = market.clone();
    let flip = market.clone();

    HostModule::new("market")
        .declarations(MARKET_TYPES)
        .function("quotes", move |_| with_app(|cx| read.read(cx).to_host_value()))
        .function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // 在这次调用退栈之后才送达，所以它不会重新进入引擎：
                    // story 和脚本 View 会一起重渲染。
                    cx.notify();
                    Ok(HostValue::from(watched))
                })
            })?
        })
}

gpui_kit::shell::export_module(market_module(&market))?;
```

用它的脚本是这样——读的是旁边那个 Rust 面板正在渲染的同一个 `Market` entity：

```js
import { quotes, watch } from "market";

const rows = quotes();
const watched = rows.filter((quote) => quote.watched).length;
```

`cargo run -- shell` 跑起来。两个面板通过两条路径读同一个 entity，一旦对不上就会立刻看出来。

## 还没有的东西

- **类和对象身份。** 模块导出的是函数。导出一个类意味着把一个活的 Host 对象交给脚本，这被上面那条纯数据边界排除了；今天用一个返回记录的工厂函数就能做同样的事。
- **同一注册表内的按函数授权。** policy 授予的是 Host 组装好的那个注册表，不会再为每个函数加一个开关。
- **向 Host 流式传输或回调。** 脚本不能把函数交给 HostModule；模块只能被调用。
