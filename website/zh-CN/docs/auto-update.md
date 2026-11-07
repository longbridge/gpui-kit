---
title: Auto Update
description: 为 GPUI Kit 应用设计版本检查、可信下载、安装与重启流程，并处理不同平台的分发边界。
order: -10.6
---

# 桌面应用自动更新

GPUI Kit 提供窗口、状态和界面组件，但没有统一的自动更新服务。应用需要决定发布渠道、更新元数据、产物验证、安装权限和重启方式；更新界面可以用 GPUI Kit 构建。先按[打包与分发](./packaging)准备可安装、可验证的产物，再添加更新入口。`gpui` 负责应用生命周期和任务调度，`gpui-kit` 整合框架能力，`gpui-base` 与 `gpui-component` 提供交互和显示组件；更新策略属于应用。使用 `gpui-shell` 扩展的应用还须确定扩展与宿主版本的兼容规则，不能只更新宿主二进制后假设旧扩展仍可工作。

自动更新通常分成两条路径：

| 安装来源 | 推荐的更新归属 | 应用内动作 |
| --- | --- | --- |
| 用户可写目录中的便携版 | 应用自己管理经过验证的文件或完整应用包 | 检查、下载、安装，明确请求重启 |
| macOS `.app` | 应用自己的完整 bundle 更新流程，或引导安装新版应用包 | 验证整个已签名 `.app`，不得只覆盖内部可执行文件 |
| Windows Inno Setup 安装 `.exe` | Inno Setup 安装器 | 检查新版并启动受支持的升级流程；维护原有安装身份与卸载信息 |
| Linux DEB/RPM 等系统包 | 发行版包管理器或软件中心 | 告知可升级，按包管理器规则执行；不要直接覆盖其管理的文件 |

表中的便携版并不表示所有目录都可写。系统安装目录可能需要管理员权限，应用也可能正占用将要替换的文件。更新器必须根据**实际安装来源**选择路径，而不能仅根据操作系统猜测。参见 [`self_update` 的权限与 bundle 说明](https://docs.rs/crate/self_update/1.3.0/source/README.md)。

## 先定义发布契约

在编写更新界面前，给每个 release 明确以下字段和规则：

1. **版本与渠道**：使用可比较的版本号；明确 stable、preview 等渠道如何筛选，以及是否允许跨渠道。只在明确的恢复流程中降级。显示当前版本、目标版本和发布说明。
2. **目标平台**：每个产物标注操作系统、CPU 架构、必要时的 ABI、最低系统版本与安装格式。选择产物时做精确匹配；没有匹配项就报告“此设备暂无更新”，不要下载相近架构。
3. **获取位置**：元数据和下载地址使用 HTTPS；限制允许的主机和重定向目标，不把未经验证的元数据 URL 当作任意文件写入指令。请求设置超时、响应大小限制，并为离线、限流和损坏的元数据提供可重试错误。
4. **可信验证**：对下载的完整产物核对大小与摘要，并验证发布者身份。与产物放在同一发布位置的 `SHA256SUMS` 能发现下载损坏，但若该发布位置被控制，攻击者也可能同时替换摘要。按分发模型采用可信签名、签名元数据或平台代码签名；macOS 更新还要核对 bundle 签名及身份。验证失败必须停在替换之前。
5. **安装身份**：固定 bundle identifier、Windows 安装器身份和 Linux 包名，让新版接续旧版的升级与卸载规则。更新不能悄悄改变用户数据目录或扩展兼容性约定。

发布新版本的顺序应是先上传并验证各平台产物及校验资料，最后让更新元数据指向它们。更新服务不要向用户宣称某个目标版本可用，却没有对应架构的可下载文件。

## 让 UI 与更新任务各司其职

把更新状态保留在拥有界面的 `Entity<T>` 中，网络与磁盘工作放在后台任务里。完成后回到 GPUI context 更新状态并调用 `cx.notify()`；不要在 `render` 中检查网络、下载文件或修改安装目录。可参考[Task](./task)、[Entity](./entity) 和 [Context](./context)。

| 状态 | 界面应说明的内容 | 可用操作 |
| --- | --- | --- |
| `Idle` / `Checking` | 尚未检查，或正在检查；手动检查时显示反馈 | 检查期间避免重复提交 |
| `Current` / `Available` | 当前已是最新版，或显示目标版本、渠道和发布说明 | 对可用版本提供用户可选择的更新动作 |
| `Downloading` / `Verifying` / `Installing` | 下载字节数或可确定进度；校验与安装各有独立提示 | 下载可取消时明确说明边界；安装阶段不要伪装成可随意中止 |
| `ReadyToRestart` | 文件已安装，当前进程仍运行旧代码 | 允许用户完成当前工作后重启 |
| `Failed` | 简洁错误、旧版是否仍可启动、重试或手动下载入口 | 不自动陷入反复下载或重启循环 |

启动时可在后台做一次检查，同时保留“检查更新”入口；重复检查应受间隔或请求合并控制。一次只安装一个目标版本，用户切换渠道或窗口关闭时应避免过期任务回写新状态。下载进度不必每个字节都触发一次 `cx.notify()`，只在可见数值有意义地变化时更新。检查失败不应阻止应用启动。

按钮应有明确的可见文字与无障碍名称，进度同时提供文字或数值，不能只靠旋转图标或颜色。下载与校验完成后用静态状态提示即可；需要动效时尊重 reduced motion。更新前若有未保存内容或正在执行的任务，提示用户完成或保存后再重启。可按[Accessibility](./accessibility)验证键盘、焦点与辅助技术路径。

## 简单路径：便携版使用 `self_update`

对**用户可写位置的单文件便携版**，可用 [`self_update` 1.3 文档](https://docs.rs/self_update/1.3.0/self_update/)中的 GitHub Releases backend 处理查询、下载及替换。下面展示应用自己的 `Cargo.toml` 依赖；归档格式和摘要支持需要显式打开对应 feature：

```toml
[dependencies]
self_update = { version = "1.3", default-features = false, features = ["ureq", "rustls", "github", "archive-tar", "compression-tar-gz", "archive-zip", "compression-zip-deflate", "checksums"] }
```

下面函数只在用户选择安装后，由后台工作线程调用。替换 `example`、`hello-world-releases`、二进制名和发布产物命名；在应用启动时调用它会让更新下载和磁盘操作阻塞启动路径。`SHA256SUMS` 必须随每个 release 提供，并包含所选产物的摘要。该代码是便携版安装核心片段，尚不包含 GPUI 界面状态与平台安装器逻辑。

```rust
fn install_portable_update() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("example")
        .repo_name("hello-world-releases")
        .bin_name("hello_world")
        .current_version(self_update::cargo_crate_version!())
        .checksum_from_asset("SHA256SUMS")
        .no_confirm(true) // The application UI already obtained consent.
        .show_output(false)
        .show_download_progress(false)
        .build()?
        .update()?;

    Ok(status.is_updated().then(|| status.version().to_owned()))
}
```

`Some(version)` 表示可执行文件已替换，`None` 表示当前已是最新版。更新器会依据当前 target 匹配 release asset。发布时需在受支持的每个 target 上验证实际文件名、归档结构和内部二进制路径；不要假定 GitHub 上任意 ZIP 或 tarball 都能被正确识别。应用自己的检查 UI 可以单独调用 `is_update_available()` 获取版本，再在用户同意后调用 `update()`；检查与安装之间若发布了另一版，`latest` 可能已经变化。正式实现应固定用户选中的 release，并在替换前再次核对精确产物。[`self_update` 的检查与安装 API](https://docs.rs/crate/self_update/1.3.0/source/README.md)对此有分别的示例。

`update()` 报告已安装新版时，磁盘上的目标文件已更新，**当前进程仍执行旧代码**。在应用保存状态、关闭窗口和后台资源后，再请求重启；不要在安装回调里立即杀死进程。`self_update` 不会替应用取得管理员权限，也不会替应用设计失败恢复策略。其 `checksum_from_asset` 校验的是同一发布位置给出的摘要，不能单独证明发布者身份。

## 按平台完成安装

### macOS

带 `Contents/Resources`、`Info.plist` 和签名的 `.app` 必须作为一个整体更新。只覆盖 `Contents/MacOS` 里的文件会使签名失效，还可能留下旧资源。若使用 `self_update` 的 bundle 模式，release 归档应包含完整 `.app`，配置 `bundle_path_in_archive`，在替换前对暂存 bundle 验证签名与预期身份；crate 不会替应用签名或公证。首次安装和更新产物都应沿用[打包指南](./packaging)里的 Developer ID、公证及目标机器验证流程。只读或 App Translocation 位置不能原地更新时，应提示用户把应用安装到正常位置，或下载新版安装包。

### Windows

ZIP 便携版与 Inno Setup 安装 `.exe` 需要不同策略。安装版应让 Inno Setup 升级文件、开始菜单入口和卸载记录；直接替换其应用 `.exe` 可能绕开权限、身份和回滚规则。对可写的便携版，也应在 Windows 的文件占用条件下测试替换与重启，必要时退出进程后由受控辅助程序或安装器完成替换。不要假定一个运行中的进程能原地覆盖它正在使用的全部 DLL。安装失败时保留旧版可启动，并给用户手动安装路径。

### Linux

用户自己解压到可写目录的 tarball 可以采用便携版路径。由 DEB/RPM 管理的文件则应通过对应包管理器升级，保持依赖声明、文件清单和卸载记录一致；不要让应用直接写入 `/usr` 下的包文件，也不要自动发起不明来源的提权。不同发行版、图形会话和 CPU 架构要分别验证。更新完成后由应用在合适时机重新启动，或者让用户按包管理器提示重启。

## 失败恢复与发布验证

在发布流程中保留上一版可下载产物，并测试更新失败时的恢复路径。把“下载完成”“摘要通过”“平台身份通过”“替换完成”“新版成功启动”视为不同检查点；任何一步失败都应记录足够的诊断信息，保留可用的旧版或提供明确的重装入口。`self_update` 的具体替换行为不能替代应用级的升级事务、数据迁移和回滚设计。

最小验证矩阵应覆盖：当前版本无更新、更新可用、相同版本与降级、渠道切换、错误架构、缺少产物、损坏摘要、错误签名、离线或超时、磁盘空间不足、目录不可写、下载中关闭窗口、安装中退出、重启后版本确认，以及用户数据与扩展兼容性。每种**实际分发格式**都应从旧版安装后升级到新版，再验证启动与卸载；单独运行 `cargo build --release` 无法验证自动更新。
