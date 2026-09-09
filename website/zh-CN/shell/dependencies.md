---
title: 依赖
description: shell package——什么样的 Git 仓库算一个，以及 manifest 如何命名、选择版本、抓取与导入它，直到编辑器看见它。
order: 9
---

# 依赖

应用用相对路径 import 自己的文件。除此之外，它写下的每一条 import 只有两个来源：运行时提供的**内建模块**——`gpui-kit`、`gpui-base`、`gpui-shell`、`gpui-fps`，以及标准运行时的 `fs/promises`、`path`、`crypto`、`net`、`websocket`——或者一个**依赖**：manifest 声明、gpui-shell 在 entry module 求值之前从 Git 抓取的 JavaScript package。

这里没有 registry，没有包管理器，也没有安装步骤。一个依赖就是一个 Git remote、一个 ref，加上脚本 import 它时用的名字。

## Shell package

依赖可以是 manifest 指向的任何一个 Git 仓库。`omarchy-ui` 属于其中一类，而这一类有自己的名字：**shell package**——为 gpui-shell 而写、而不是为 Node 或浏览器而写的 JavaScript package，就像 crate 是为 Cargo 而写的一样。五件事决定一个仓库是不是 shell package：

| shell package                                               | 原因                                                    |
| ----------------------------------------------------------- | ------------------------------------------------------- |
| 发布 ES module，且不需要构建步骤                            | 运行时直接求值 checkout 里的文件，而且 `require` 不存在 |
| 根目录 `package.json` 带 `"type": "module"` 与 `main`       | 它让声明只需一行，并同时向运行时和编辑器指明 entry      |
| 只 import 内建模块与自己的文件                              | 其余的都解析不了——它无法反向伸回 import 它的应用        |
| 把 `gpui-kit` 与 `gpui-base` 当作由环境提供，而非自己的依赖 | 它们来自加载它的运行时，版本由 Host 决定                |
| 不声明任何属于自己的 capability                             | 它的 `fs` 与 `fetch` 调用都跑在使用方应用的授权之下     |

这个名字不会被任何代码读取：依赖是靠被声明来识别的，不是靠被贴标签。它的作用是让一个作者写下、另一个作者找到；写给搜索引擎的那一份，是仓库上的 `gpui-shell` topic。

[`omarchy-ui`](https://github.com/huacnlee/omarchy-ui) 就是一个 shell package，本页余下部分都以它为例。

## 声明一个依赖

`omarchy-ui` 是一个提供展示组件与主题工具的 shell package。在 `gpui-shell.json` 里加一行就够了：

```json
{
  "id": "com.example.projects",
  "name": "Projects",
  "entry": "main.js",
  "dependencies": {
    "omarchy-ui": "huacnlee/omarchy-ui"
  }
}
```

map key 就是脚本使用的裸模块名——package 内部并不参与决定它。是 manifest 给 package 起名，就像 import 的 `as` 子句那样：两个应用可以用不同名字引用同一个 remote，而仓库改名也不会改变 import：

```js
import {
  AppShell,
  Button,
  CenteredWorkspace,
  MutedText,
  PageColumn,
  Surface,
  Title,
} from "omarchy-ui";

export function render(cx) {
  const card = new Surface()
    .children([
      new Title("Projects").build(cx),
      new MutedText("Choose a project to continue").build(cx),
      new Button("project-create")
        .label("Create project…")
        .onClick((_event, context) => context.notify())
        .build(cx),
    ])
    .build(cx);

  const page = new PageColumn("projects-page").child(card).build(cx);
  return new AppShell()
    .content(
      new CenteredWorkspace("projects-workspace").content(page).build(cx),
    )
    .build(cx);
}
```

其他什么都不用改。脚本还是[快速开始](./getting-started.md)里的那个脚本，依赖只是拓宽了它能 import 的范围。

## 什么能解析，解析到哪里

| 写法                                  | 解析结果                                             |
| ------------------------------------- | ---------------------------------------------------- |
| `"omarchy-ui"`                        | package entry——见 [package entry](#package-entry)    |
| `"omarchy-ui/src/style"`              | checkout 内的该文件，`.js` 后缀可省略                |
| package 内部的 `"./theme.js"`         | 该 package 自己 checkout 内的文件                    |
| package 内部的 `"gpui-kit"`           | 内建模块，与应用代码中完全一致                       |
| 另一个已声明的依赖名                  | 那个 package 的 entry——已声明的 package 之间互相可见 |
| 从 package 内部用裸名 import 应用文件 | 拒绝：package 不能反向伸回 import 它的应用           |

解析结果一旦离开起点所在的 checkout，就会在模块加载前被拒绝，因此 `../` 无法走出一个 package、进入旁边的缓存。在应用目录内，同一条边界就是应用根目录，也正是[沙箱](./capabilities.md#沙箱)对相对 import 已有的规则。

**依赖不是第二层沙箱。** 它在应用自己的上下文里求值，持有的授权与 manifest 完全相同：package 读文件，用的就是你的 `fs.read` 范围。声明一个依赖，等于像引入一个 Rust crate 那样信任它的代码，而你钉住的 ref 决定了那究竟是哪一份代码。

## 选择版本

字符串形式是严格的 GitHub 简写或完整 Git URL，两者都可带可选的 `#ref`：

```json
{
  "dependencies": {
    "default-main": "huacnlee/omarchy-ui",
    "named-ref": "huacnlee/omarchy-ui#v1.2.0",
    "commit": "https://github.com/huacnlee/omarchy-ui#0123456789abcdef0123456789abcdef01234567",
    "remote-head": "https://github.com/huacnlee/omarchy-ui"
  }
}
```

| 形式                       | 选中                         |
| -------------------------- | ---------------------------- |
| `owner/repository`         | `main`                       |
| `owner/repository#ref`     | 该 branch、tag 或 commit-ish |
| `https://…/repository`     | remote 的 `HEAD`             |
| `https://…/repository#ref` | 该 branch、tag 或 commit-ish |

简写形式故意收得很紧：恰好一组 `owner/repository`，字符限于字母数字与 `.`、`-`、`_`，最多一个 `#`，前后不能有空白，fragment 必须是合法的 Git ref 名。其余一律作为 manifest 错误，而不是从一个笔误里猜出 URL。完整 URL 可以是任意 Git 传输方式，包括 `ssh://` 与 `git@host:owner/repo`。

**branch、tag 或 remote `HEAD` 在每次加载应用时都会重新 fetch 并解析；commit ID 永远选中同一个 commit。** 依赖一个 branch，意味着下次开窗时代码会在你脚下变化——这在你自己开发 package 时很方便，而在你发布之后就是一个供应链决定。凡是不由你掌控的东西，请钉住 tag 或 commit。

抓取需要 Host 的 `PATH` 上有 `git`，并且发生在脚本 capability 存在之前——这是 gpui-shell 代表应用运行 Git，不是脚本在访问网络，因此它不受 `capabilities.network` 管辖，也不需要 `fs.execute`。当没有东西要抓（缓存里已有该 commit）时，这次加载完全不产生网络访问；而当依赖是移动 ref 且当前没有网络时，fetch 失败，应用不会加载。

## Package entry

checkout 就绪后，gpui-shell 读取 package 根目录的 `package.json`，把字符串类型的 `main` 作为 entry。`omarchy-ui` 发布的是：

```json
{
  "type": "module",
  "main": "src/index.js",
  "types": "src/index.d.ts"
}
```

于是 `import { Title } from "omarchy-ui"` 求值的是 `src/index.js`。没有 `package.json`，或者其中没有 `main` 时，entry 是根目录的 `index.js`。

运行时读 `main`，编辑器读 `types`。两者出自同一个文件——这正是为什么一个附带 `.d.ts` 的 package 不需要应用做任何事，就能在调用处拥有完整类型。

JSON 格式错误、非字符串 `main`，以及缺失、不是文件或逃出 checkout 的 entry，都会在应用 JavaScript 执行前令加载失败。

## Object 形式

最初的 object 形式保持完全兼容。它必须显式且只指定一个 `branch` 或 `tag`，其 repository 相对的 `entry` 默认是 `index.js`：

```json
{
  "dependencies": {
    "omarchy-ui": {
      "git": "https://github.com/huacnlee/omarchy-ui",
      "tag": "v1.2.0",
      "entry": "src/index.js"
    }
  }
}
```

现有 manifest 无需迁移。改用字符串形式，意味着由 package 通过 `package.json` 的 `main`（或根目录 `index.js`）自己发布 entry，而不是让每个使用方各写一遍。

## 缓存

```text
~/.gpui-shell/cache/dependencies/
├── locks/<remote>.lock
├── mirrors/<remote>.git
└── checkouts/<remote>/<commit>/
```

`<remote>` 是去掉 fragment 后完整 URL 的 SHA-256，它既是 remote 的身份，也是缓存的身份。每个 remote 一把锁，串行化 mirror 更新。checkout 按 commit 寻址且从不改写，因此并发启动与旧的 hot-reload generation 各自读到自己开始时的那棵树。

每次使用都会拿 mirror 的 configured origin 与 manifest 校验，且比较的是原始配置值——Git 的 `url.*.insteadOf` 仍可以选择另一个实际 fetch URL，镜像站或企业内网替换因此照常可用。Git 以非交互方式运行，禁用凭据提示，每条命令限时 30 秒，于是一个要求输入密码的仓库会给出错误信息，而不是把一个正等着打开的窗口挂在那里。

这份缓存不会被自动清理。它是内容寻址的，所以直接删掉是安全的：下次加载会重新抓取需要的部分。

## 编辑器看见的东西

运行时靠 manifest 回答 `import { Title } from "omarchy-ui"`。编辑器则是从 import 所在文件向上查找 `node_modules`，它从来没听说过 `gpui-shell.json`。放着不管，一条正确的 import 会被标红成找不到的模块，它背后的每个名字也随之失去类型、参数提示和文档。

因此每次加载——以及 `gpui-shell types`——都会把 materialize 出来的 checkout 按 manifest 给的名字链接进应用的 `node_modules`：

```text
projects/
├── gpui-shell.json
├── main.js
├── gpui-kit.d.ts          运行时生成——请忽略
├── jsconfig.json      只生成一次，之后归你
└── node_modules/
    └── omarchy-ui  →  ~/.gpui-shell/cache/dependencies/checkouts/<remote>/<commit>
```

这样编辑器读到的，就是运行时即将执行的同一批文件，它显示的签名与 JSDoc 都来自 package 自身，不会与实际运行的代码脱节。

只有 gpui-shell 自己写下的条目会被替换或删除——指向自身依赖缓存的 symlink，或带有它标记文件的目录。同名的已安装 package 不会被动到；manifest 中已移除的依赖，其链接也会一并清除。若平台拒绝创建 symlink（例如未开启开发者模式、权限不足的 Windows 进程），gpui-shell 改为写入一个转发该 checkout 的小 package：裸 import 的类型效果相同，只有 package subpath import 无法解析。

当目录里既没有 `jsconfig.json` 也没有 `tsconfig.json` 时会生成一份 `jsconfig.json`，且只生成一次——已有的配置永远不会被替换。它不是装饰：靠推断得到的 `moduleResolution` 可能落到那种从不查看 `node_modules` 的解析方式，把运行时明明能解析的依赖标红；而默认的 `lib` 会把浏览器的全局对象塞给脚本，它们的声明与 `gpui-kit.d.ts` 产生冲突，于是描述 API 的那个文件本身反倒被报成错误。

`node_modules` 和 `gpui-kit.d.ts` 一样属于生成物，两者都应加入忽略列表：

```text
gpui-kit.d.ts
node_modules/
```

这个目录之所以叫 `node_modules`，是因为所有编辑器只认这一个位置；这里没有包管理器参与，也没有任何东西来自 registry。这个名字还换来了安静：TypeScript 会把从这里解析到的内容视为 external library，于是依赖自身的 implicit-`any` 之类诊断不会混进你自己的诊断里。

## 抓取与链接何时发生

| 调用                                            | 抓取并链接 | 失败时                               |
| ----------------------------------------------- | ---------- | ------------------------------------ |
| `gpui-shell <directory>`                        | 是         | 加载失败；仅链接这一步是 best-effort |
| `gpui-shell check <directory>`                  | 是         | 作为 check 失败报告                  |
| `gpui-shell types <directory>`                  | 是         | 报告错误，并带非零退出码             |
| 嵌入式 Host 的 `ShellRuntime::load`             | 是         | 加载失败；仅链接这一步是 best-effort |
| `gpui_kit::shell::write_dependency_links(root)` | 是         | 以 error 返回给调用方                |

加载依赖抓取，所以无法 materialize 的依赖会让加载失败。写编辑器链接则不然：只读的应用目录是失去编辑器类型的理由，不是拒绝运行的理由。`gpui-shell types` 存在的意义正是这个差别——它做同样的事，并把没能完成的部分报出来。

hot-reload 对待 package 的方式与对待应用文件一致：每次加载都是一个新的 module generation，所以重启应用就足以让一个 branch 依赖前进到新的 commit。

## 可能的失败

以下每一种都在应用 JavaScript 求值之前报告：

| 信息                                                       | 原因                                               |
| ---------------------------------------------------------- | -------------------------------------------------- |
| `GitHub shorthand must contain exactly owner/repository …` | 简写里带了路径、scheme 或非法字符                  |
| `a string dependency #Git ref must not be empty`           | 末尾多了一个 `#`                                   |
| `could not clone Git dependency …`                         | Git 失败：remote 不存在、没有凭据、没有网络        |
| `git timed out after 30 seconds …`                         | fetch 卡住，通常是弹出了交互式凭据提示             |
| `Git dependency … cache origin is …, expected …`           | 两个 manifest 对同一条缓存记录不一致；删掉它再试   |
| `Git dependency … package.json main must be a string`      | `main` 是对象，或是逃出 checkout 的路径            |
| `Git dependency … has no entry …`                          | `main` 或 object 形式的 `entry` 指向不存在的东西   |
| `cannot resolve module … from …`                           | subpath import 指向不存在的文件，或离开了 checkout |

## 发布一个 shell package

shell package 就是一个普通的 Git 仓库：`omarchy-ui` 没有构建产物、没有 lockfile，也没有发布步骤。在决定它是不是 shell package 的那五件事之外，让它用起来舒服的是这些：

- **根目录 `package.json` 里的 `main`**，使用方只写一行、不用写 `entry`。旁边的 `"type": "module"` 让编辑器和运行时对「源码是 ES module」保持一致。
- **一个统一 re-export 公开面的 entry。** 由 `src/index.js` 决定使用方能叫出哪些名字；checkout 里的其余文件仍可通过 subpath 触达——那是有用的应急出口，却是糟糕的公开 API。
- **类型放在源码旁边**，通过 `package.json` 的 `types` 指出。生成的 `.d.ts` 与 JSDoc 都可以，两者都会顺着链接抵达调用处——使用方的 `jsconfig.json` 不需要任何 `paths` 配置。
- **为发布打 tag**，让使用方可以钉住 `#v1.2.0`，而不是跟着 `main` 走。
- **在仓库上加 `gpui-shell` topic**，让想找 shell package 的人能找到它。

## 继续阅读

| 页面                             | 内容                                                |
| -------------------------------- | --------------------------------------------------- |
| [能力](./capabilities.md)        | manifest 的其余部分：身份、版本，以及脚本能触达什么 |
| [快速开始](./getting-started.md) | `gpui-shell types`、`check`，以及依赖加入的那份声明 |
| [API 参考](./api.md)             | package 与你的代码一同 import 的内建模块            |
