---
title: Hosting
description: Rust 这一侧的全貌——运行时的生命周期、挂载脚本 View、从 Host 状态刷新它、指标、退出请求与 hot-reload。
order: 11
---

# Hosting

[Getting Started](./getting-started.md) 给的是把脚本 View 放上屏幕的那四行。这一页是 Rust 接口的其余部分：该调什么、什么时候调，以及那两三处“看起来该调的那个其实是错的”。

## 运行时

一个 `ShellRuntime` 拥有一个 VM。它是一个带内部可变性的 `Rc`——既不是 `Send` 也不是 `Sync`——所以它待在拥有 `App` 的那个线程上。

```rust
gpui_kit::shell::init(cx);                     // gpui-base、默认 token 调色板、样式表

let runtime = ShellRuntime::new(cx)?;     // 一个 VM，并注册为当前 App 的默认 runtime
```

`new(cx)` 让回调、 HostModule 与 hot reload 不必由 Host 层层传递句柄，也能找到默认 runtime。明确管理多个 VM 的 Host 可以用 `new_isolated()` 创建其他 runtime，并自行保留这些句柄。

`gpui-shell` 通过 GPUI 的 inspector reflection table 暴露 fluent style
方法，release 构建也不例外。因此，依赖这个 crate 会为 Cargo 统一后的依赖图启用
`gpui-base/inspector` feature。这是 JavaScript 样式接口正常工作的必要条件；嵌入方
应把 release 构建中新增的检测代码与依赖计入构建成本。

## 加载与实例化

普通应用窗口只需一次加载，并直接获得它的 `ShellRoot`：

```rust
cx.open_window(options, move |window, cx| {
    let root = runtime.load(&app_root, window, cx);
    #[cfg(debug_assertions)]
    if let Ok(watch) = runtime.watch(&root, window, cx) {
        watch.forget();
    }
    root
})?;
```

存在 `gpui-shell.json` 时，`load` 会验证其中的身份信息，并采用其 entry。capabilities 是能力请求，不等于 Host 已经批准；两条路径都按 Host 当前的默认 policy 运行，没有 manifest 时入口为 `main.js`。两条路径都会刷新 `gpui-kit.d.ts`；加载失败会渲染可选择文字的错误界面，而不是让 Host panic。需要自行处理结构化错误的 Host 使用 `try_load`。失败状态的 root 没有可供监听的应用，因此 `watch` 会返回 `Err`；这里忽略这个错误，才能保留可选择的失败界面。

下面的低层方法只供需要把脚本 View 装进既有 Rust 组合的 Host 使用。

加载把源码变成一个**View 类型**——脚本 default 导出的那个类。实例化把这个类型变成一个**View 对象**，也就是一个活的实例：

```rust
let view_type = runtime.load_app(&root, "main.js")?;    // 一个目录
let view_type = runtime.load_source("inline", source)?; // 一个字符串，测试用

let object = runtime.instantiate(&view_type, window, cx)?;
```

`load_app` 会解析目录、读取入口文件、求值该模块。这里的每一种失败都是一个带着脚本自身调用栈的 `ShellError`——语法错误、解析到应用根目录之外的 import、缺失或形态不对的 default 导出。

实例化会执行脚本的 `init`，因此它需要一个活的 `Window`：`init` 里可能会创建 `InputState` 这类留存状态。

## 挂载

脚本 View 和别的 GPUI View 没有两样，它挂在**一个 `ShellRoot` 之下**：

```rust
cx.open_window(options, move |window, cx| {
    let object = runtime.instantiate(&view_type, window, cx).expect("view");
    let content = cx.new(|_| ScriptView::new(runtime.clone(), object));
    cx.new(|cx| ShellRoot::new(content.into(), window, cx))
})
```

`ShellRoot` 持有 dialog 栈、sheet、toast 栈、焦点恢复与 Tab 导航——正是 `Root` 对一个 `gpui-component` 窗口所起的作用。`window.open_dialog` 这一类调用要经由它找到根 View，所以挂在别的根 View 之下的脚本会拿到一条讲清原因的拒绝，而不是悄无声息地没反应。

Host 也可以直接驱动同样这几个界面，插件面板与 Host 自己的 UI 因此落在同一个栈里：

```rust
root.update(cx, |root, cx| {
    root.open_dialog(view.into(), window, cx);
    root.push_toast(ToastRequest::new("Saved").with_level(ToastLevel::Success), window, cx);
    root.close_all_dialogs(window, cx);
});
```

## Host 状态变了，怎么刷新 View

这是最容易调错的一个，而且调错了不会报错。

```text
cx.notify()        ── 把这个 View 再画一遍       （不跑脚本）
view.refresh(cx)   ── 而且它的描述已经过期了   （脚本会跑）
```

因为脚本的一次 `render` [不等于一帧渲染](./state.md#render-什么时候执行)，光调 `cx.notify()` 重绘的是已经存在的那份 Snapshot。如果 Host 改动的是脚本**会读到**的东西——某个 HostModule 背后的实体、一项设置、一份文档——就必须告诉 View：描述本身已经过期了。

```rust
runtime.refresh(&root, cx)?;
```

runtime 会先确认 `root` 装载的是它自己的应用，再让脚本 View 失效并安排重绘。 Host 不需要拿到具体的 `ScriptView`，也不会因为手工 downcast 或混用另一个 runtime 的 View 而刷新错误对象。

反过来调错则立刻看得见——界面就是不更新——这与 GPUI 里忘了调 `cx.notify()` 是同一种失败方式。

## 脚本能碰到什么

三项 Host 设置的生命周期不同。Capabilities 会在每个新 View 加载时冻结；store handle 与 HostModule registry 则是该 View 共享的实时 Host 配置，替换后会在下一次调用生效：

```rust
gpui_kit::shell::set_capabilities(
    Capabilities::new()
        .read_roots([app_root.clone()])
        .write_roots([data_dir.clone()])
        .store(true),
);
gpui_kit::shell::set_store_path(data_dir.join("store.json"));
gpui_kit::shell::export_module(market_module(&market))?;
```

三项的默认都是“什么都没有”：没有文件访问、没有存储位置、没有 HostModule 。见 [Capabilities](./capabilities.md) 与 [HostModule](./host-module.md)。

独立二进制还会检查 `<root>/gpui-shell.json`。其中已识别的字段提供应用身份、可选的应用/Shell 版本元数据、entry 与 capability 请求；只有 `id`、`name` 和 `entry` 必填。Embedder 若要让每个加载的应用拥有不同 grant 与 HostModule registry，也可以直接构造 `Policy`。

## 观察它花了多少

运行时把两件事分开计数，而这两个数之间的差就是重点：

```rust
let reading = runtime.read_metrics();
reading.script_renders();      // 跟着 cx.notify()、重载、主题变化走
reading.materializations();    // 跟着帧走
reading.script_render_time();  // 脚本 render 里的总耗时
reading.native_time();         // 其中花在 HostModule 里的部分
reading.slowest_script_render();
reading.structure_repeat_rate();  // 一次重建产出的结构，与它替换掉的那份是否相同
```

`RuntimeMetrics::since(&earlier)` 给出两次读数之间的差值，每秒速率就是这么算的。这里没有重置：计数器属于运行时，把它们清零会把正在读它们的其他人一起挪动。要量某一段，就自己留一个基线再相减——Shell story 每次切换 feed 都会取一次基线，所以它的读数回答的是“这个 feed 要花多少”，而不是“这个窗口从打开到现在干了多少”。

回归测试可以直接对 `script_renders` 做断言；[基准测试里的第三个数](./engine.md#那次实测)靠的正是这一点。

`structure_repeats()` 与 `structure_changes()` 回答的是另一个问题：在那些有上一份描述可比的重建里，有多少次产出的**结构**完全相同——相同的组件、相同的 builder 方法、相同的树，只有其中的取值变了。运行时不会因为这个答案而少做任何事；它存在，是为了给[Snapshot 缓存止步于哪里](./performance.md#snapshot-缓存止步于哪里)量个尺寸。 View 的第一次构建没有前一份可比，两个计数都不计它。

## 开发构建的配置

Host 的 debug 构建，**单次脚本渲染大约比 release 慢三倍**，而差距全部来自两个依赖。
在一个实时应用上实测——一个每笔行情都重渲染的行情终端，用运行时自带的
[`RuntimeMetrics`](#观察它花了多少)：

| `[profile.dev.package]`          | 平均脚本渲染 | 平均物化   |
| -------------------------------- | ------------ | ---------- |
| 不配，或只写 `rquickjs`          | 31.5 ms      | 3.9 ms     |
| `rquickjs-sys` + `rquickjs-core` | **11.3 ms**  | **1.2 ms** |
| release（对照）                  | 11.0 ms      | 1.2 ms     |

所以：

```toml
[profile.dev.package]
rquickjs-sys = { opt-level = 3 }
rquickjs-core = { opt-level = 3 }
```

**只写 `rquickjs` 没有任何作用**，这正是坑所在：它是一个薄门面，把 `rquickjs-core`
重新导出而已，写它既没优化到解释器，也没优化到绑定。`rquickjs-sys` 编译的是 QuickJS
本体——C 源码，经 `cc` 构建，而 `cc` 读的是**那个包**在 profile 里的优化级别；
`rquickjs-core` 则是每一个跨界值的转换所在。没优化的解释器，正是让 debug 构建
用起来像另一个产品的原因。

`llrt_*` 那批**不需要**这么做。同一个应用上实测，它们带来的差异在噪声范围内：
`fs`、`net`、`crypto` 之类根本不在渲染路径上，优化它们换不来脚本作者能感知的东西。

这些设置只在**构建出二进制的那个 workspace 根**生效。库无法替依赖它的应用设定 profile，
所以 `gpui-shell` 没办法替你配好——每个 Host 都得自己写一遍。

## 退出请求

脚本里的 `process.exit(code)` 是**一个请求，绝不是 `exit(2)`**。一个插件不能把 Host 进程带走，而 Host 可能还有未保存的状态。运行时把这个请求交给 Host，由 Host 决定怎么办：

```rust
gpui_kit::shell::on_exit_request(|request, window, cx| {
    match request.view() {
        Some(view) => close_the_panel_showing(view, window, cx),
        None => cx.quit(),
    }
});
```

`request.code()` 是脚本要求的退出码，`request.view()` 在有的情况下会指出请求来自哪个 View——插件 Host 关掉的应该是**那个**插件的面板，若换成关窗口，就等于让一个插件终结了别人的工作。

**授权了 exit 却没装处理器的 Host，会在调用现场被告知**，而不是永远不知道：`process.exit()` 会抛出异常，并点名 `on_exit_request`。一个没人回应的请求，是朝着讨好方向说的谎——脚本拿到了成功，而什么都没发生。

## Hot-reload

一个调用就能开起来，`--watch` 用的也是这一个：

```rust
runtime.watch(&root, window, cx)?.forget();
```

`runtime.watch` 从已加载的 root 读取解析后的目录与 manifest entry，不再让 Host 维护第二份可能漂移的元数据。它不暗藏构建模式策略：CLI 在解析到 `--watch` 后启用监听，嵌入式 Host 则可以把调用放进 `#[cfg(debug_assertions)]`。返回的 `Watcher` 持有这次监听：把它 drop 掉，循环就停；`.forget()` 则让它跟随 View 继续运行。 View、运行时或窗口任意一个消失时，循环也会自己结束。

一次重载会重新读取**每一个**模块，入口也在内——一个悄悄用了旧 import 的 hot-reload 比没有更糟，因为它看起来是成功的。它会先把所有可能失败的活干完，再去碰活着的那个 View：新代码加载失败时，上一个 View 继续运行，错误进 `tracing`，窗口里由一条固定 id 的 toast 报出来；下一次成功的重载会撤掉这条 toast。

View 本身能挺过重载。`ScriptView::replace_object` 只换掉脚本产出的那部分，实体保留下来，随之保留的还有窗口、焦点与元素身份。

插件 unload 是比移除单个 view 更强的生命周期边界：manager 会在丢弃插件前取消所有携带该插件 `Policy` 的 outstanding task，包括没有 owner 的工作。任何 continuation 都不能继续保留或使用已卸载插件的权限。

## 脚本出错的时候

抛异常的脚本不会把界面一起带走。最后一份可用的 Snapshot 仍然挂在那里，失败信息报在它上面，读者的滚动位置、焦点、正在读的内容都还在。在有什么让 View 失效之前，运行时不会重跑那个失败的 `render`。

记得装一个 `tracing` subscriber。运行时通过 `tracing` 报告脚本错误、未处理的 promise rejection 与非法 phase 调用，target 是 `gpui_kit::shell::script`；没有 subscriber 的话这些全部被丢弃，症状就是一个悄悄不再响应的 View。

## 还没有的东西

- **给卡住的脚本做监管。** 解释器自己的中断会切断一次调用，但没有东西会去重启一个反复撞上中断的运行时。
