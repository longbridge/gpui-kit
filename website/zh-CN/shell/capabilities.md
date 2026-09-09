---
title: 能力授权
description: 默认全部拒绝的模型，fs / storage / clipboard / process 接口，存储位置，以及沙箱裁掉了什么。
order: 8
---

# Capabilities

脚本默认**什么都拿不到**。没有文件访问、没有剪贴板、没有进程执行、没有网络。`Capabilities::default()` 就是空集，并有一条断言把它钉在那里。

唯一的例外是存储，而且只在 manifest 这一层：没有写 `storage` 的应用会拿到属于它自己的 `localStorage`，就像浏览器不问自答地给每个 origin 一个那样。这是关于作者**要写什么**的约定，不是模型上的口子——Rust 侧的 `Capabilities` 在 Host 开口之前照样拒绝，manifest 也照样可以写 `"storage": false`。见 [Storage](#storage)。

授权由 Host 决定，因为只有 Host 知道它对即将运行的这段代码信任到什么程度。至于它主动**递出去**的东西——它自己的、有意暴露的那部分 Rust——见 [HostModule](./host-module.md)。 View 在加载时冻结 capabilities；修改默认值只影响之后加载的应用，不会悄悄改变已经按某项授权运行的代码。

```rust
gpui_kit::shell::set_capabilities(
    Capabilities::new()
        .read_roots([application_root.clone()])
        .write_roots([data_directory.clone()])
        .storage(true)
        .exit(true),
);
```

## 本地运行的应用被授予什么

从命令行运行一个目录，是一次明确的信任行为——与 `node app.js` 一样——所以 `gpui-shell <directory>` 授予的是一组具体且很窄的能力：

|          |                                |
| -------- | ------------------------------ |
| 读       | 应用目录，以及它自己的存储目录 |
| 写       | 它自己的存储目录               |
| 存储     | 授予                           |
| 剪贴板   | **不**授予                     |
| 进程执行 | **不**授予                     |
| 退出请求 | 授予                           |
| 网络     | **不**授予                     |

因此应用可以读自己的源码与资源、使用自己的存储，除此之外没有别的。它刻意比“全部放开”要窄，因为将来安装的插件会走同一条代码路径、由 manifest 来决定授权——而一个对本地运行足够宽松的默认，继承过去就是错的默认。

## 拒绝信息会写明怎么修

每一条拒绝都以“要声明什么”结尾，而不只是说了句拒绝：

```text
filesystem read is not granted; declare capabilities.fs.read in the manifest
```

```text
`/etc/passwd` is outside every granted read root;
add its directory to capabilities.fs.read in the manifest
```

```text
storage is not granted; set capabilities.storage to true
```

```text
running `git` is not granted; add it to capabilities.fs.execute in the manifest
```

```text
process.exit() is not granted; set capabilities.process.exit to true in the manifest
```

## Manifest

目录通过 **`gpui-shell.json`** 被识别。Manifest 是惰性数据——发现阶段只读取身份、可选版本元数据、Git 依赖与请求的权限，不执行 entry module。它识别 `id`、`name`、`version`、`shell-version`、`entry`、`dependencies` 与 `capabilities`；只有 `id`、`name` 和 `entry` 必填：

```json
{
  "id": "com.example.quotes",
  "name": "Quotes",
  "version": "1.0.0",
  "shell-version": "0.6.0",
  "entry": "main.js",
  "dependencies": {
    "omarchy-ui": "huacnlee/omarchy-ui"
  },
  "capabilities": {
    "fs": { "read": ["${pluginDir}"], "write": ["${dataDir}"] },
    "network": {
      "hosts": ["stream.example.com"],
      "http": [
        {
          "scheme": "https",
          "host": "api.example.com",
          "methods": ["GET"],
          "path_prefixes": ["/v1/"]
        }
      ]
    },
    "storage": true,
    "clipboard": { "read": false, "write": true },
    "process": { "exit": false }
  }
}
```

`dependencies` 把裸模块名映射到一个 JavaScript package，gpui-shell 会在 entry
module 运行之前从 Git 抓取它——`import { Title } from "omarchy-ui"`。字符串形式
接受严格的 GitHub 简写或完整 Git URL，可带可选的 `#ref`；显式指定 `branch` 或
`tag` 的 object 形式同样保持支持。每次加载还会把 package 链接到编辑器能找到的
位置，于是这条 import 会带上 package 自己的类型与文档。版本选择、package entry、
缓存，以及编辑器看见的东西，详见[依赖](./dependencies.md)。

这个块里的每项授权省略时都默认**拒绝**，只有 `storage` 默认给予——要拒绝它就写 `"storage": false`。

未知字段、非法 reverse-DNS id、显式填写但不合法的 SemVer、不兼容的 `shell-version`、逃出目录的 entry，以及未知 `${...}` placeholder 都会在代码执行前令 manifest 失效。省略 `version` 时显示为 `unknown`。省略 `shell-version` 时接受当前 runtime；显式填写时，它表示应用所需的最早兼容 gpui-shell 版本。版本不低于该要求的 runtime 都会被接受。独立 CLI 会拒绝非法 manifest，不会在假设已经不一致时继续执行 entry。

每条 scoped `network.http` 规则除了 host、method 与 path 外，还会绑定请求的 scheme 与有效端口。`scheme` 默认为 `https`；`port` 默认为该 scheme 的标准端口，仅非默认 endpoint 需要显式填写。

## `fs`

```js
import * as fs from "fs/promises";
```

每个调用都返回 promise。`await` 它们，或者接 `.then`——另见下面关于 `render` 的提示。

| 调用                                        | resolve 结果                     |
| ------------------------------------------- | -------------------------------- |
| `fs.readFile(path)`                         | `Uint8Array`                     |
| `fs.readFile(path, "utf8")`                 | UTF-8 文本                       |
| `fs.writeFile(path, contents)`              | —                                |
| `fs.readdir(path)`                          | 按名字排序的名称数组             |
| `fs.readdir(path, { withFileTypes: true })` | 带 `isDirectory()` 的 `Dirent[]` |
| `fs.exists(path)`                           | `true` / `false`                 |
| `fs.unlink(path)`                           | —                                |
| `fs.rmdir(path)`                            | —                                |
| `fs.mkdir(path, options?)`                  | —                                |

```js
const source = await fs.readFile("notes.md", "utf8");
await fs.writeFile("notes.md", source + "\n");
```

相对路径相对某个已授权的根解析；绝对路径必须本来就在某个根之内。这套接口里的每一条路径都经过**同一个解析器**，所以不存在第二处让穿越漏洞藏身的地方。它先做归一化——`../../etc/passwd` 在到达文件系统之前就被拒绝——然后把「是否在根之内」这件事交给文件系统判定，而不是判定字符串：授权承诺的是一个**目录**，而 `data/escape/passwd` 在字面上位于根之内，一旦 `escape` 是符号链接就读到了 `/etc/passwd`。路径中已经存在的最深一段会被连同链接一起解析，其结果必须仍在根之下；解析不到任何目标的符号链接会被直接拒绝，而不是猜它指向哪里。

**授权是一个句柄，不是一个字符串。** 解析器交回一个打开的目录，它无法被诱导指向自身之外的任何东西；读、写、列目录、删除、建目录全部对着**它**执行——于是一条路径永远不会被解析两次，「判定允许」与「实际使用」之间也就没有窗口。

这一点要紧，是因为显而易见的写法行不通。先检查路径再调 `std::fs`，路径被解析了两次：检查时就在的链接会被抓住，而在两次解析**之间**替换掉某个目录组件的，会被第二次解析跟出根目录。这里用的是 [`cap-std`](https://docs.rs/cap-std)——在 Linux 上是 `openat2(RESOLVE_BENEATH)`，其他平台是逐级 `openat` 遍历。

其中三项的行为值得说明，理由都是同一个：

**被拒绝的路径抛异常，而不是返回 `false`。** “你不能看”和“它不存在”是两个不同的事实，把它们合并会让脚本能一次一个布尔值地探测自己根目录之外的文件系统。

**删文件和删目录是两个调用**，和 Rust 一样——单独一个 "remove" 说不清目录算不算在内。`remove_dir` 只收空目录：写权限是按根授予的，递归删除会把一次路径笔误变成整个应用数据目录的丢失。真要这么做的脚本可以自己遍历。

**`mkdir` 就是别处那个 `mkdir`。** 不带参数时只建一层，父目录不存在就报错；`{ recursive: true }` 才把父目录一起建出来。它原来叫 `create_dir_all`——那个名字确实说清了它做什么，代价是它不是每个脚本作者已经认识的那个名字。

**`read_dir` 已排序。** 渲染列表的脚本不该自己再排一遍，也不该继承文件系统的任意顺序。

**每个调用都返回 promise。** 系统调用在主线程之外执行——磁盘要花多久没有上界，而在这里阻塞会同时卡住帧和 VM，而且卡在中断预算看不见的地方，因为那段时间花在内核里。

**拒绝仍然在调用点抛出**，而不是变成 rejected promise。能力检查几乎不花时间，留在调用线程上；而没人 await 的 rejected promise，等于没人看得见的拒绝。

`readFile` 会拒绝超过 64 MiB 的文件，并指出文件名和上限。没有这个上限的话，替代方案是一个必须塞进 JavaScript 堆的字符串——而那个堆本身也有上限——于是失败会表现为 VM 内部的内存耗尽，而不是一句你能据以行动的话。

`writeFile` 每次最多接受 8 MiB。`readdir` 最多返回 10,000 个 entry 或累计 1 MiB 的 UTF-8 文件名（先触及哪个就按哪个停止），避免恶意目录让一个 promise 造成无界分配。

::: tip 仍然不要在 `render` 里读文件
`render` 描述界面，它没法 await。在 `init` 或事件回调里读，把结果留在 View 上，拿到后 `cx.notify()`。
:::

## Storage

[Web Storage API](https://developer.mozilla.org/zh-CN/docs/Web/API/Web_Storage_API)，和浏览器里的是同一个。不需要 import：`localStorage` 与 `sessionStorage` 是全局变量，同时也挂在 `window` 上。

```js
localStorage.setItem("todolist.items", JSON.stringify(items));
const saved = localStorage.getItem("todolist.items"); // 键不存在时为 null
localStorage.removeItem("todolist.items");
localStorage.length;
localStorage.key(0);
localStorage.clear();
```

| 成员                | 说明                        |
| ------------------- | --------------------------- |
| `length`            | 已存的键数量                |
| `key(index)`        | 该位置上的键，越界为 `null` |
| `getItem(key)`      | 值，键不存在时为 `null`     |
| `setItem(key, val)` | 存入，值会被转成字符串      |
| `removeItem(key)`   | 忘掉一个键                  |
| `clear()`           | 全部忘掉                    |
| `flush()`           | 写入落盘后 resolve          |

**两者只差在活多久。** `localStorage` 是 Host 放好的一个文件，跨重启存活；`sessionStorage` 是内存，随进程一起消失。这也是只有前者是一项 capability 的原因：`sessionStorage` 里的东西从不离开进程，没有什么可授权的，因此在一个什么都没授权的 Host 上它照样能用。

**值是字符串**，和 web 上完全一样——`setItem` 会把拿到的东西转成字符串。有结构的东西进出各走一趟 `JSON.stringify` 和 `JSON.parse`，这跟你在浏览器里会写的代码是同一段：

```js
localStorage.setItem(
  "window",
  JSON.stringify({ title: "Notes", size: [640, 480] }),
);
const window = JSON.parse(localStorage.getItem("window") ?? "{}");
```

每个成员都是同步的，这是刻意的：`getItem` 在 `render` 里也可达，所以值缓存在内存里，读取从缓存回答。每次渲染读一次文件是荒唐的。

**一次修改安排一次写入，而不是执行一次写入。** 文件在后台线程写出——先写临时文件再改名覆盖目标，所以写到一半崩溃留下的是之前完整的配置，而不是一个被截断的文件——并且同时只有一次写入在途，于是一连串 `setItem` 汇成一个文件，而不是一次一个文件。写入在途期间发生的改动，由下一次写入带上。

需要确认落盘时 `await localStorage.flush()`。这是相对浏览器接口唯一多出来的一个成员，它存在是因为浏览器根本不必回答这个问题——它的存储从头到尾都是同步的。它是**屏障，不是第二个写入者**：等待此前所有修改抵达磁盘，写入失败时用写入自己的错误 reject。若让它自己再写一次，就会与自动写入抢同一个临时文件，两者之间没有任何顺序保证——旧版本可能最后落盘，把新版本抹掉。

cache 与等待队列都有上限：单个存储文件序列化后最多 8 MiB，最多 4,096 个 key，单个 value 最多 1 MiB。同时最多允许 1,024 个尚未完成的 `flush()` barrier；更多调用会 reject，而不是无限增长 waiter 列表。

### 存储在哪里

存储按应用划分，位置由 Host 选择——应用不能指定自己的存储位置，否则两个应用可以故意撞在一起。

**Host 给应用起名字，数据跟着这个名字走：**

```rust
let data = gpui_kit::shell::set_bundle_id("com.example.notes")?;
gpui_kit::shell::set_capabilities(Capabilities::new().write_roots([data]));
```

| 平台              | 位置                                                                    |
| ----------------- | ----------------------------------------------------------------------- |
| Linux 与其他 Unix | `$XDG_DATA_HOME/gpui-shell/apps/<id>/store.json`，默认 `~/.local/share` |
| macOS             | `~/Library/Application Support/gpui-shell/apps/<id>/store.json`         |
| Windows           | `%APPDATA%\gpui-shell\apps\<id>\store.json`                             |

id 就是身份，所以目录被改名、被移动、被一次升级整个替换掉，数据都还在——这正是用户说"我的设置"时指的东西。改用路径作 key，一次升级就等于悄悄让用户从头开始。

**运行时不会去某个文件里找这个 id。** 只有安装了这个应用的那一层知道它叫什么；运行时自己挑一个 manifest 去读，等于对一件不属于它的事情宣称权威。

被"指向"某个目录的 Host——这个命令行、一个 dev server——没有这样一个名字，而在那种情况下路径确实就是身份。`gpui_kit::shell::bundle_id_for_path(root)` 用目录名加完整路径的摘要造一个，于是同一个目录总是访问到同一份数据，同一份源码的两个 checkout 也互不干扰。这在你正在编辑它时是对的，在它已经被安装之后是错的——而这正是声明一个真名字带来的区别。

id 允许 `a-z`、`0-9`、`.`、`-`、`_`，不允许 `..`。这不是整洁问题：它会被拼到用户数据目录后面，没检查的 id 能够到目录里的其他东西。数据放在那里而不是应用内部，因为应用目录可能只读、往往是一个 git checkout，也不是用户预期自己数据所在的地方。

### 未被授权时的退化

未被授权的 `localStorage` 会抛异常，而写得好的应用会把它当作关于 Host 的一个事实，而不是一个错误：

```js
// storage.js —— 取自示例应用
export function load() {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === null) return [];
    const items = JSON.parse(saved);
    return Array.isArray(items) ? items : [];
  } catch (error) {
    console.warn(
      `todolist: storage unavailable, starting empty (${error.message})`,
    );
    return [];
  }
}
```

示例的页脚随后会在界面上说明这一点——“Not saved — this host did not grant storage, so the list lasts for this run only”——这才是对的形态：在边界处吸收拒绝，并对用户说实话。

## 剪贴板

```js
cx.write_to_clipboard("copied");
const text = cx.read_from_clipboard(); // 剪贴板中没有文本时为 undefined
```

名字取自 `App::write_to_clipboard` 与 `App::read_from_clipboard`，挂在 `cx` 上是因为 GPUI 就放在那里。不需要 import。

读与写是**两项独立授权**，拒绝信息会指出缺的是哪一半：

```text
writing the clipboard is not granted; declare capabilities.clipboard.write in the manifest
```

剪贴板需要一次实时的 Host 调用——GPUI 的 `App` 只在一次调用期间存在——所以一个没有活调用的 `cx` 会直说，而不是 panic：

```text
cx.read_from_clipboard() needs a live host call; call it from render, an event handler or a task
```

## `console`

```js
console.info("loaded", count, { source: "disk" });
console.warn("could not save");
```

`debug`、`log`、`info`、`warn` 与 `error`。它是全局的——与其他 JavaScript 运行时一样——不需要 import；shell 原本把同一个对象再以 `gpui.log` 导出了一遍，那只多了一个名字，别的什么也没多。

**不需要任何能力**：能跑起来的脚本本来就能说话，禁掉它只会让作者失去自己的诊断信息，别的什么都拦不住。

多余的参数会以空格分隔追加在后面，与 `console.log` 的行为一致。结构化的值以 JSON 打印，因为那是读日志的人想看到的形式。

输出通过 `tracing` 走，target 是 `gpui_kit::shell::script`，所以在日志过滤里脚本输出与 Host 输出是可分开的。**没有安装 `tracing` subscriber 的 Host 会把这些全部丢弃**——连同运行时自己报告的抛异常的处理函数、未处理的 rejection 与 phase 非法的调用。`gpui-shell` 二进制安装的是一个 `INFO` 级别的 stderr sink，`--dev` 下是 `DEBUG`。

## `process`

```js
import process from "process"; // 同时也是一个全局

const { code, stdout, stderr } = await process.run("git", ["status"]);
process.exit(0);
```

`process.run` 返回 promise，理由是 `fs` 那条的加强版。文件读取没有时间上界；子进程连上界的影子都没有——它可以算上几分钟、等一个永远不来的输入，甚至活得比窗口还久。在这个线程上等它，会把帧和 VM 一起卡住，而且卡在内核里，interrupt budget 看不见。

输出是**捕获的，不是继承的**：跑一条命令的脚本几乎总是想要它说了什么，而在一个窗口程序里，子进程往 Host 的 stdout 写，是写到没人会看的地方。`code` 成功时是 `0`，被信号杀死时是 `-1`——那种情况本来就没有退出码。

执行是有界的：30 秒、stdout 8 MiB、stderr 8 MiB。触及任一上限都会终止并回收子进程，同时 reject promise。取消所属任务或销毁 runtime 也会终止子进程。子进程从清空的环境开始，不会继承 Host secret；shell 也不提供添加环境变量的选项。

它受执行授权约束，授权有三种形态：拒绝（默认）、命令名白名单，或不受限。被拒绝的命令**在调用处抛出**而不是 reject，和被拒绝的 `fs` 路径一样——没人 await 的 rejected promise，等于没人看见的拒绝。

`process.exit` 在运行时内部是**一个请求，绝不是 `exit(2)`**。它把退出码交给 Host 安装的处理函数，由后者决定怎么做——关闭插件的面板、关闭窗口、结束进程。一个插件不能把 Host 进程带走，而 Host 可能还有未保存的状态。

处理函数不是可选的：授予了这项能力却没有安装处理函数的 Host，会让这次调用**直接失败**并指明是 Host 漏了什么。没人应答的请求比拒绝更糟，因为脚本分辨不出这两者。`gpui-shell` 这个二进制安装的是「 Host 本身就是进程」时该有的策略——按脚本要求的退出码结束进程。

这个名字上的撞车是刻意的。`process` 正是 JavaScript 作者——或者生成 JavaScript 的模型——会去伸手拿的名字，所以运行时把自己受能力约束的接口放在那里，而不是把这个名字空着、任其看起来像 Node 的却行为不同。

`process.exit` 使用独立的 `capabilities.process.exit` 授权。文件系统访问权不会隐式获得关闭面板、窗口或进程的权限。

## 沙箱

除了能力授权之外，运行时还会裁剪语言本身。以下全部在**未开启开发模式**时生效。

**没有动态代码。** `globalThis.eval` 被直接删除——`ReferenceError` 不会被特性探测误认为是一个可用的 `eval`，而一个抛异常的桩会。四个函数编译器全部被替换：`Function`，以及通过 `(async function(){}).constructor`、`(function*(){}).constructor` 和异步生成器等价物可达的那三个。`Function` 是被*替换*而不是删除，并保留了真正的 `Function.prototype`，所以 `x instanceof Function` 与 `.call` / `.apply` / `.bind` 继续可用，只有构造会抛异常。

**冻结内建原型。** `Object`、`Array`、`Function`、`String` 与 `Number` 的原型被冻结。一个 VM 将来会承载多个插件，这使得这些原型成为共享可变状态：一个插件给 `Object.prototype` 加一个可枚举属性，就改变了其他所有插件以及运行时自身 prelude 的 `for...in`。代价是真实的——一个给 `Array.prototype` 打补丁的库会在 import 时就停止工作——所以明知要运行这类库的 Host 可以关掉冻结，并保留沙箱的其余部分。

**模块解析被限制在应用根目录内。** `import "./ui.js"` 相对发起 import 的文件解析；任何解析到应用目录之外的结果都会被拒绝。动态 `import()` 刻意保持可用——延迟加载将来靠它——并且由同一个解析器约束。

**资源上限**，让失控的脚本报错而不是把窗口一起带走：

| 上限                                       | 值                                                                           |
| ------------------------------------------ | ---------------------------------------------------------------------------- |
| 堆                                         | 256 MiB——泄漏表现为一个可捕获的 JavaScript 异常，而不是整个 Host 被 OOM kill |
| 解释器栈                                   | 1 MiB——深递归表现为 `RangeError`，而不是原生栈溢出                           |
| 已加载的 JavaScript module                 | 每个源码文件 8 MiB                                                           |
| 尚未完成的 host task                       | 每个 runtime 1,024 个                                                        |
| 单次调用耗时：render 与 layout             | 50 ms                                                                        |
| 单次调用耗时：event 与 task                | 500 ms                                                                       |
| 单次调用耗时：不在任何调用中，例如模块求值 | 5 秒                                                                         |

时钟在每一次 Host 调用时重置，这正是渲染路径能比事件回调有更紧预算的原因。**中断无法被 `catch` 吞掉**——这一点有测试来度量，因为如果能被吞掉，中断就根本不是一道防线。每个 WebSocket 另有一条由 `read`、`write` 与 `close` 共用的 8-command 队列；队列已满时新操作会 reject，并要求调用方等待 outstanding work。

这里没有 quickjs-libc 的 `std`：quickjs-libc 从一开始就没有被编进这个构建。运行时仍提供下文列出的、经过审计的小型 `os` 模块。

::: tip 开发模式
`--dev` 会启用源码监听，并在构造运行时之前调用 `gpui_kit::shell::set_development_mode(true)`。它会恢复动态代码构造器并让内建原型保持可写。

开发模式从不放宽能力约束。它让语言更好摆弄，但不会发出任何人没有声明过的访问权限——因为一项作者从没写下来的授权，就是一项在生产环境里会缺失的授权。
:::

## 网络与安全标准 API

全局 `fetch(url, options?)` 返回 promise，结果提供 `{ status, ok, url, , json() }`。它的授权比原始网络更窄：每次请求与 redirect 都必须匹配声明的 HTTP host、method，以及精确 path 或 path prefix；HTTPS 永不降级到 HTTP，authorization 与调用方 header 也不会跨 origin。

`net.connect(host, port)` 与 `websocket` 模块具名导出的 `WebSocket.connect(url, { headers? })` 使用 `capabilities.network.hosts`。`WebSocket` 不会安装成浏览器全局，也不是构造器。Raw TCP 的 `read()` 返回 `Uint8Array`，到达 EOF 时返回 `null`，因此传输分块不会经过有损文本解码。WebSocket 支持文本与 `Uint8Array` 消息，并通过单一 actor 串行化写入；它不会跟随 redirect。Connect、handshake 与 write 操作都有 30 秒 timeout。每个 socket 同一时间只允许一个 outstanding `read()`；第二个会立即 reject，而不是与第一个争抢下一条消息。凭证 header 与握手控制 header 会被拒绝。Raw TCP 与 WebSocket 权限有意比 HTTP request grant 更宽。

DNS 解析是有界的进程级共享服务：所有应用共用两个 resolver worker 和一个最多 64 个请求的队列。排队沿用每次连接已有的 deadline，所以饱和时会以 timeout 失败，不会无界增长内存或线程。这是资源收敛，不是每应用的服务质量保证；同一进程中运行互不信任应用的 Host，不会获得应用之间的 DNS 公平性。

运行时还提供 `buffer`、`path`、`url`、`crypto`、`zlib`、`console`、`process` 与 `os`。它们是生成的 `gpui-kit.d.ts` 所声明、经过审计的 LLRT/Host 子集；`node:` 别名和任意 Node 内建模块不属于 shell 契约。

## 还没有的东西

- **向用户询问授权。** 授权在应用加载之前就已决定，不会在使用的那一刻弹出询问。
