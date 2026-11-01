---
title: Packaging
description: 把脚本应用发布成桌面应用——`gpui-shell bundle` 生成体积很小的 macOS .app 或 Linux 压缩包，使用已安装的 gpui-shell 或自动下载，也可以生成 Windows 便携文件夹；还可以手工组装自带二进制的独立包。
order: 16
maturity: [preview]
---

# Packaging

脚本应用运行在预编译的 `gpui-shell` 二进制上。整个过程不需要编译，所以打包要解决的只是这个二进制从哪里来：

- **`gpui-shell bundle`** 为 macOS、Linux 或 Windows 打包。macOS 和 Linux 的包不带二进制：在用户的机器上，包会使用已安装的 `gpui-shell`，或者下载打包时所用的那个版本；只下载一次，所有打包的应用共用，所以包的大小和应用本身差不多。Windows 的包是一个便携文件夹，二进制就是应用的可执行文件。
- **独立包**需要手工组装，把二进制放在应用旁边，适合自己的安装程序或自定义目录结构。见[独立包](#独立包)。

## 打包之前

应用需要一个带稳定 `id` 的 manifest。打包后的应用按这个 `id` 保存数据，而不是按路径，所以移动 `.app` 或安装更新都不会丢失用户的数据。

```json
{
  "id": "com.example.clock",
  "name": "Clock",
  "version": "1.0.0",
  "shell-version": "0.7.3",
  "entry": "main.js",
  "capabilities": { "storage": true }
}
```

其中的 `capabilities` 会按原样授予，所以只申请应用真正用到的部分，见 [Capabilities](./capabilities.md)。打包前先运行 `gpui-shell check -d clock`。

`gpui-shell new clock` 会按这个结构创建一个起步目录。

## 一条命令打包

```bash
gpui-shell bundle -d clock
```

| 目标      | `dist/` 中的产物                              | 内容                                                                       |
| --------- | --------------------------------------------- | -------------------------------------------------------------------------- |
| `macos`   | `Clock.app`                                   | `Info.plist`、作为 `Contents/MacOS/Clock` 的启动脚本、`Resources/app` 中的应用 |
| `linux`   | `clock-1.0.0-linux.tar.gz` 和 `install.sh`    | 压缩包里是启动脚本 `clock` 和 `app/`；安装脚本在压缩包外，与它并列         |
| `windows` | `clock-1.0.0-windows-x86_64.zip`              | `Clock.exe`（就是 `gpui-shell.exe` 本身）和 `app\`                         |

macOS 和 Linux 的包里没有 `gpui-shell` 二进制，所以在任何平台上都能打，也和 CPU 架构无关。Windows 的包带着 x86-64 的 `gpui-shell.exe`，所以 zip 约 20 MB；`bundle` 从哪里获取它见 [Windows](#windows)。

这些名字都来自 manifest：`name` 用作 `.app` 的名字、Windows 可执行文件的名字和窗口标题，它的小写形式（`clock`）用作命令名和压缩包名；`version` 写进压缩包名和 `Info.plist`；`id` 用作 bundle identifier、Linux 的安装目录和 desktop entry 的文件名，以及窗口的 application id。所以打包必须有 `gpui-shell.json`。

| 选项                    | 含义                                                                  |
| ----------------------- | --------------------------------------------------------------------- |
| `<name>`、`-d <dir>`    | 要打包的应用，写法同 `run`；省略时为当前目录                          |
| `--platform <platform>` | `macos`、`linux` 或 `windows`；省略时为命令所在的平台                 |
| `--icon <png>`          | 正方形 PNG，最好是 1024×1024                                          |
| `--out <dir>`           | 输出目录；省略时为当前目录下的 `dist`                                  |
| `--download-url <url>`  | 仅用于 Linux：压缩包将发布到的地址，让单独运行的 `install.sh` 能下载它 |

复制应用时会去掉只在开发时需要的内容：隐藏文件和目录、`node_modules/`、所有 `gpui-kit.d.ts`，以及根目录下的 `jsconfig.json` 和 `AGENTS.md`。输出目录本身也会排除，所以 `dist/` 可以放在应用目录里，`gpui-shell new` 也已把它写进 `.gitignore`。目录里的其他内容都会打包进去，所以应用不加载的测试和脚本请放在别处。

### 运行时

macOS 和 Linux 的包需要的 `gpui-shell` 版本不低于打包时所用的版本：用 `gpui-shell` 0.8.0 打的包，接受 0.8.0 及更新的版本。安装包时（Linux）或打开时（macOS），包会按这个顺序查找：`~/.local/bin/gpui-shell`、`PATH` 上的 `gpui-shell`、`/opt/homebrew/bin`、`/usr/local/bin`、`~/.cargo/bin`。

如果都不够新，就从 GitHub 下载对应的 release，用 release 附带的 `.sha256` 校验后安装为 `~/.local/bin/gpui-shell`，和[安装脚本](./getting-started.md)使用同一个位置，不需要 sudo。之后装上的更新版本会被直接使用，包不会把运行时降级。

因此，除非已经装有足够新的 `gpui-shell`，第一次安装或启动需要联网。没有网络的机器，请先用拷贝过去的 release 压缩包安装 `gpui-shell`。

启动脚本用 `gpui-shell run --dir <app> --packaged` 启动应用。`--packaged` 会让应用的数据始终保存在 manifest 的 `id` 下，与目录在哪里无关；窗口标题使用 manifest 的 `name`；不往目录里写编辑器文件，所以签过名的包不会因此失效，只读的安装位置也能正常运行。`--packaged` 和 `bundle` 在同一个版本中发布，所以任何满足包版本要求的运行时都支持它。

### macOS

把 `Clock.app` 拖进“应用程序”文件夹就完成了安装。它的可执行文件是一个启动脚本：第一次打开时查找或下载 `gpui-shell`，下载期间显示通知，失败时弹出提示，然后交给它运行应用。`--icon` 会生成 `Clock.icns`，包含不超过原图尺寸的所有标准尺寸。

`.app` 不做签名：签名需要你自己的开发者身份，分发给别人之前请按 [macOS 签名](#macos-签名)一节签名并公证。

### Linux

`bundle` 会写出两个文件：压缩包，以及与它并列的 `install.sh`。只用压缩包，就能在解压的位置直接运行应用：

```bash
tar -xzf clock-1.0.0-linux.tar.gz
./clock-1.0.0-linux/clock
```

`install.sh` 为当前用户安装应用。它写有压缩包的 SHA-256，只安装与之相符的压缩包，按下面的顺序取第一个可用的来源：

1. 作为参数传入的压缩包路径或 URL；
2. 脚本旁边的 `clock-1.0.0-linux.tar.gz`；
3. 打包时指定的 `--download-url` 加上压缩包的文件名。

```bash
sh install.sh                                         # 使用旁边的压缩包
curl -fsSL https://example.com/clock/install.sh | sh  # 打包时用了 --download-url https://example.com/clock
```

要使用第二种方式，把 `install.sh` 和压缩包一起发布到那个地址，比如 release 的附件或任何静态托管。`install.sh` 先确保 `gpui-shell` 可用，这样第一次启动不用等下载；然后只往 `~/.local` 下写文件，不需要 sudo：

| 路径                                                    | 内容                     |
| ------------------------------------------------------- | ------------------------ |
| `~/.local/share/com.example.clock/`                     | 启动脚本和 `app/`        |
| `~/.local/bin/clock`                                    | 指向启动脚本的链接       |
| `~/.local/share/applications/com.example.clock.desktop` | 启动项                   |
| `~/.local/share/icons/hicolor/<size>x<size>/apps/`      | 图标（使用 `--icon` 时） |

再次运行就是更新：先完整复制出新版本，替换失败时恢复旧版本。如果安装目录是一个链接，或者 `~/.local/bin/clock` 不是它创建的，它会拒绝覆盖。`sh install.sh --uninstall` 会删除这些文件；按 `id` 保存的用户数据和共用的 `gpui-shell` 都会保留。

### Windows

`clock-1.0.0-windows-x86_64.zip` 是一个便携文件夹：用户解压到任意位置，打开 `Clock.exe` 即可。不需要安装，也不需要管理员权限。

```text
clock-1.0.0-windows-x86_64\
├── Clock.exe                 # gpui-shell.exe，已标记为 GUI 程序
└── app\
    ├── gpui-shell.json
    └── main.js
```

`Clock.exe` 就是打包所用版本的 release 中的 `gpui-shell.exe`，只改了一处：它被标记为 GUI 程序，所以打开时不会出现控制台窗口。它会在自己旁边找到 `app\`，并按 `id` 保存应用的数据，所以移动文件夹或换成更新的版本都不会丢失数据。`--icon` 不会嵌入：可执行文件仍使用 `gpui-shell` 的图标。如需签名，请在打包之后用你的证书执行 `signtool sign /fd SHA256 /a Clock.exe`。

在 Windows 上，`bundle` 直接复制正在运行的 `gpui-shell.exe`。在 macOS 和 Linux 上，它从 GitHub 下载 release `v<version>` 中的 `gpui-shell-x86_64-pc-windows-msvc.zip`，用它的 `.sha256` 校验后，把可执行文件缓存为 `~/.gpui-shell/cache/runtime/v<version>/x86_64-pc-windows-msvc/gpui-shell.exe`（设置了 `GPUI_SHELL_HOME` 时在它下面）。没有网络时要为 Windows 打包，请先把那个 release 的 `gpui-shell.exe` 放到这个位置。

## macOS 签名

分发给别人的包必须签名并公证，否则 Gatekeeper 会拒绝打开。包里的所有内容都要签名，脚本也是其中一部分：

```bash
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: Example Inc (TEAMID)" Clock.app
ditto -c -k --keepParent Clock.app Clock.zip
xcrun notarytool submit Clock.zip --keychain-profile notary --wait
xcrun stapler staple Clock.app
```

签名也会封存脚本，签名之后对 `Resources/app` 的任何改动都会让签名失效，所以签名要放在最后一步。

## 独立包

`bundle` 只在 Windows 便携文件夹里放二进制。如果要发布自带二进制的 macOS 或 Linux 包，比如给无法下载的机器用，或者交给自己的安装程序，就需要手工组装：把预编译的 `gpui-shell` 二进制和应用目录并排放在一起。二进制在不带参数启动时，会在自己旁边寻找应用；Finder、`.desktop` 启动项、开始菜单快捷方式都是这样启动程序的，它们都没法传路径。这样启动的应用，会得到和 `--packaged` 相同的处理。

| 平台    | 可执行文件位置                     | 应用目录位置                         |
| ------- | ---------------------------------- | ------------------------------------ |
| macOS   | `Clock.app/Contents/MacOS/`        | `Clock.app/Contents/Resources/app/`  |
| Linux   | 包目录                             | 可执行文件旁边的 `app/`              |
| Windows | 包目录                             | 可执行文件旁边的 `app\`              |

可执行文件可以改名，比如把 `gpui-shell` 改成 `Clock`：应用是按可执行文件所在的位置找到的，和它叫什么名字无关。像 `bundle` 一样，去掉只在开发时需要的内容。

### 获取二进制

每个 GitHub Release 都为每个平台附带一个压缩包，旁边附有 `.sha256`：

| 目标平台              | 产物                                         |
| --------------------- | -------------------------------------------- |
| macOS，Apple 芯片     | `gpui-shell-aarch64-apple-darwin.tar.gz`     |
| macOS，Intel          | `gpui-shell-x86_64-apple-darwin.tar.gz`      |
| Linux，x86-64         | `gpui-shell-x86_64-unknown-linux-gnu.tar.gz` |
| Windows，x86-64       | `gpui-shell-x86_64-pc-windows-msvc.zip`      |

```bash
curl -fsSLO https://github.com/longbridge/gpui-kit/releases/latest/download/gpui-shell-aarch64-apple-darwin.tar.gz
tar -xzf gpui-shell-aarch64-apple-darwin.tar.gz
```

在构建脚本里固定版本时，把 `latest/download` 换成 `download/v0.7.3`。也可以自己构建：在仓库里运行 `cargo build -p gpui-shell --release`，二进制位于 `target/release/gpui-shell`。

### macOS

```text
Clock.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── Clock              # 改名后的 gpui-shell 二进制
    └── Resources/
        ├── Clock.icns
        └── app/
            ├── gpui-shell.json
            └── main.js
```

`Info.plist` 指定可执行文件和图标。bundle identifier 使用 manifest 的 `id`，让两者保持同一个身份：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>Clock</string>
  <key>CFBundleIdentifier</key>
  <string>com.example.clock</string>
  <key>CFBundleName</key>
  <string>Clock</string>
  <key>CFBundleIconFile</key>
  <string>Clock</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0.0</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
```

组装只需要几次复制：

```bash
set -e
app=Clock.app
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp Info.plist "$app/Contents/"
cp gpui-shell-aarch64-apple-darwin/gpui-shell "$app/Contents/MacOS/Clock"
cp Clock.icns "$app/Contents/Resources/"
rsync -a --exclude node_modules --exclude gpui-kit.d.ts --exclude jsconfig.json \
  --exclude AGENTS.md clock/ "$app/Contents/Resources/app/"
```

`iconutil -c icns Clock.iconset` 可以从一个 PNG 图片目录生成 `Clock.icns`。要让同一个包同时支持 Apple 芯片和 Intel，不要只复制其中一个，而是把两个 release 二进制合并成可执行文件：

```bash
lipo -create -output "$app/Contents/MacOS/Clock" \
  gpui-shell-aarch64-apple-darwin/gpui-shell \
  gpui-shell-x86_64-apple-darwin/gpui-shell
```

然后按 [macOS 签名](#macos-签名)一节签名并公证。

### Linux

```text
clock/
├── clock                     # 改名后的 gpui-shell 二进制
├── clock.png
└── app/
    ├── gpui-shell.json
    └── main.js
```

把这个目录打成压缩包发布，并把 desktop entry 安装为 `~/.local/share/applications/com.example.clock.desktop`（为所有用户安装时放在 `/usr/share/applications`），`Exec` 指向解压后的可执行文件。文件名请使用 manifest 的 `id`：窗口的 application id 就是这个 `id`，Wayland 桌面靠它把窗口和启动项对应起来。

```ini
[Desktop Entry]
Type=Application
Name=Clock
Exec=/opt/clock/clock
Icon=/opt/clock/clock.png
Categories=Utility;
StartupWMClass=com.example.clock
```

同样的目录结构可以原样放进 `.deb`、`.rpm` 或 AppImage：关键是 `app/` 要留在可执行文件旁边。运行时的依赖和所有 GPUI 应用一样：Vulkan 驱动、`libxkbcommon`，以及 Wayland 或 X11 会话。

### Windows

```text
Clock\
├── Clock.exe                 # 改名后的 gpui-shell.exe
└── app\
    ├── gpui-shell.json
    └── main.js
```

`gpui-shell.exe` 是控制台程序，这样它的命令才能输出内容。原样复制后从资源管理器或快捷方式启动，它会在应用窗口旁边多出一个自己的控制台窗口；`bundle` 写出的可执行文件则被标记为 GUI 程序，除此之外与这个目录结构相同。把这个文件夹打成 zip，或者交给 Inno Setup、WiX 等安装程序；快捷方式指向 `Clock.exe`，不带参数。

## 相关

| 页面 | 内容 |
| --- | --- |
| [Getting Started](./getting-started.md) | 安装运行时、`run`、`new`、`check`、`bundle` 和已安装的应用 |
| [Capabilities](./capabilities.md) | manifest 可以申请哪些能力，打包后的应用会被授予什么 |
| [Dependencies](./dependencies.md) | Git 依赖，首次启动时拉取到 `~/.gpui-shell/cache` |
