---
title: Packaging
description: 将 GPUI Kit 应用制作成 macOS、Windows 和 Linux 上可安装、可验证的发布产物。
order: -10.5
---

# 桌面应用打包与分发

`cargo build --release` 生成可执行文件，不会自动创建应用包或安装程序。GPUI Kit 提供应用框架和组件；应用项目负责选择分发格式、确定应用标识、收集运行时资源、签名并验证安装。本文以仓库现有的 `hello_world` package 为构建示例，示例版本是 `0.6.5`。实际发布时，请用自己的应用名、版本、图标、标识及二进制路径替换示例值。

先完成[安装与本机运行](./installation)，再决定要交付哪一种产物：

| 平台 | 便于内部测试或手动解压 | 面向用户的安装方式 |
| --- | --- | --- |
| macOS | `.app`，或包含 `.app` 的归档 | 包含已签名、已公证应用的 `.dmg`；也可按自己的发布渠道选择其他格式 |
| Windows | 含 `.exe` 和资源的 `.zip` | 使用 Inno Setup 制作带安装与卸载流程的 `.exe` |
| Linux | 含可执行文件和资源的 `.tar.gz` | Debian/Ubuntu 使用 `.deb`；其他发行版需要各自的包格式或安装流程 |

归档与安装程序的区别在于谁创建应用入口、把文件放在哪里，以及如何升级和卸载。不要把 Windows ZIP 称为安装程序，也不要假设 DEB 能安装在所有 Linux 发行版。平台模型可参照 [Apple 的分发说明](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution)、[Microsoft 的打包概览](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/)和 [Debian 二进制包说明](https://www.debian.org/doc/debian-policy/ap-pkg-binarypkg.html)。

## 先构建并列出运行时文件

在仓库根目录、目标操作系统的构建机上运行：

```sh
cargo build --release --locked -p hello_world
```

`--locked` 使用已提交的 `Cargo.lock`；它不会替你处理目标平台工具链、签名或安装器。请先在构建机运行 release 二进制，确认窗口能够打开，再将**相同的二进制**放入打包暂存目录。为每个操作系统和架构分别构建、标记并测试产物；例如 macOS arm64 与 x86_64、Windows x86_64、Linux x86_64 不能仅凭改动文件名互换。

打包前核对以下文件和约定：

- **应用标识**：确定稳定的包名或 bundle identifier、显示名、版本、架构与安装路径。应用升级时维持对应身份；若要让 preview 与 stable 并存，应给它们不同身份。
- **资源**：GPUI Kit 的默认原生 SVG 图标由 `Assets` 嵌入二进制；`include_bytes!` 等编译期嵌入资源也无需复制源目录。应用使用文件系统 `Path` 读取的图片、字体、配置或其他数据，必须随安装包提供，并从可靠的安装位置查找。详见[图标与资源](./assets)及[字体](./fonts)。
- **运行时依赖**：检查动态链接库与平台服务是否存在于目标机器。开发机已有的库不等于用户机器也有。Linux 特别要检查图形会话、Vulkan 驱动以及实际链接的系统库；[安装指南](./installation)里的 `-dev` 包是构建依赖，不应原样列为最终用户依赖。
- **平台功能**：通知、文件关联、URL scheme、WebView 等可能另需应用身份、权限声明或系统组件。只为应用实际使用的功能配置它们；参见[原生扩展](./native-extension)和 [WebView](./webview)。

将二进制、外部资源、许可文本和平台元数据先放入独立的暂存目录，再生成归档或安装程序。这样可以在压缩前检查内容，并避免把 `target/release` 的中间文件、调试符号或开发机路径一起发布。

如果应用使用 GPUI 的应用身份承载通知等平台功能，启动时应通过 `cx.set_app_identity("com.example.helloworld", "HelloWorld")` 设置，并与安装包的身份一起检查；参见[系统通知](./system-notification)。更换安装包标识可能生成另一份安装或数据目录，更换运行时标识也可能影响系统功能。

## macOS

macOS 的应用包有固定结构；至少需要可执行文件、`Info.plist`，通常还需要图标和应用资源。下面是示意结构，`CFBundleExecutable` 必须与 `Contents/MacOS` 中的文件名一致：

```text
HelloWorld.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── hello_world
    └── Resources/
```

下面命令在 macOS 构建机上制作最小 `.app`。示例并未包含图标；正式发布时把自己的 `.icns` 文件放入 `Contents/Resources`，并在 plist 中加上相应图标键：

```sh
mkdir -p dist/HelloWorld.app/Contents/MacOS dist/HelloWorld.app/Contents/Resources
cp target/release/hello_world dist/HelloWorld.app/Contents/MacOS/hello_world
cat > dist/HelloWorld.app/Contents/Info.plist <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>com.example.helloworld</string>
  <key>CFBundleName</key><string>HelloWorld</string>
  <key>CFBundleDisplayName</key><string>HelloWorld</string>
  <key>CFBundleExecutable</key><string>hello_world</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.6.5</string>
  <key>CFBundleVersion</key><string>1</string>
</dict></plist>
PLIST
plutil -lint dist/HelloWorld.app/Contents/Info.plist
open dist/HelloWorld.app
```

`Info.plist` 中的标识、名称及版本要与应用一致。需要文件类型、URL scheme、隐私授权或最低系统版本时，也在对应配置中声明。可手工组装，也可选择适合自己应用的 bundler；GPUI Kit 本身没有要求使用某一个 bundler。[Apple 的 bundle 布局说明](https://developer.apple.com/documentation/bundleresources/placing-content-in-a-bundle)解释了 `MacOS` 与 `Resources` 的用途。

在本机打开暂存的 `.app` 验证图标、资源和功能。若通过网站直接向普通用户分发，使用自己的 **Developer ID Application** 身份签名，并按 Apple 的流程公证。应用内若有额外可执行文件、动态库或 framework，先分别签名嵌套代码，再签名外层 `.app`。将以下示例身份替换成自己的有效签名身份：

```sh
codesign --force --timestamp --options runtime \
  --sign "Developer ID Application: Example (ABCDE12345)" dist/HelloWorld.app
codesign --verify --deep --strict --verbose=2 dist/HelloWorld.app
```

临时 ad-hoc 签名只能用于相应的本地测试，不能替代 Developer ID 签名和公证。[Apple 的代码签名说明](https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac/)给出了顺序和选项。

将验证过的 `.app` 放进 DMG 暂存目录，按需添加指向 `/Applications` 的别名，再创建磁盘映像。一个不含自定义 Finder 布局的最小命令是：

```sh
mkdir -p dist/dmg-root
cp -R dist/HelloWorld.app dist/dmg-root/HelloWorld.app
hdiutil create -volname HelloWorld -srcfolder dist/dmg-root \
  -format UDZO -ov dist/HelloWorld.dmg
```

通过网站直接分发时，先签名 DMG，再提交公证；公证完成后检查结果并把票据附加到 DMG。以下命令要求你已在本机安全地配置好 `notarytool` 的 keychain profile，并已换成自己的签名身份：

```sh
codesign --force --timestamp \
  --sign "Developer ID Application: Example (ABCDE12345)" dist/HelloWorld.dmg
xcrun notarytool submit dist/HelloWorld.dmg \
  --keychain-profile HelloWorld --wait
xcrun stapler staple dist/HelloWorld.dmg
xcrun stapler validate dist/HelloWorld.dmg
```

`notarytool` 返回提交结果后，应确认状态为 `Accepted`；失败时查看日志并修复签名或包内容。最后在干净的 macOS 机器上挂载 DMG、拖入 Applications、首次启动，并检查 Gatekeeper。详见 [Apple 的公证工作流](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)。

## Windows

仅分发单个 `hello_world.exe` 适合最小示例；真实应用若从相对路径读取资源，应在 ZIP 和安装器中保持相同的文件布局。Windows 上可先暂存文件，制作便于验证的 ZIP：

```powershell
cargo build --release --locked -p hello_world
New-Item -ItemType Directory -Force dist\windows | Out-Null
Copy-Item target\release\hello_world.exe dist\windows\hello_world.exe
Compress-Archive -Path dist\windows\* `
  -DestinationPath dist\hello-world-windows-x64.zip -Force
```

ZIP 不会自动创建开始菜单项、卸载记录或升级规则。需要安装程序时，使用 Inno Setup。以下是可用于**这个单文件示例**的最小脚本；把它保存为仓库根目录的 `packaging.iss`，在 Windows 构建机使用 Inno Setup 编译。多文件应用应把资源一起放进 `[Files]`，并测试资源路径。

```ini
[Setup]
AppId=Example.HelloWorld
AppName=HelloWorld
AppVersion=0.6.5
DefaultDirName={autopf}\HelloWorld
DefaultGroupName=HelloWorld
OutputDir=dist
OutputBaseFilename=HelloWorld-Setup
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "dist\windows\hello_world.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\HelloWorld"; Filename: "{app}\hello_world.exe"

[Run]
Filename: "{app}\hello_world.exe"; Description: "Launch HelloWorld"; Flags: nowait postinstall skipifsilent
```

```powershell
ISCC.exe packaging.iss
```

发布真实产品时，用产品唯一且长期稳定的 `AppId`，补充发布者、图标、许可证、必要的运行时依赖和数字签名。应在制作 ZIP 或安装器前先签名应用 `.exe`，再签名最终安装器和卸载器；Inno Setup 会把卸载器嵌入安装 `.exe`，因此事后仅签名安装器不等于签名卸载器。按 [Inno Setup 的签名卸载器流程](https://jrsoftware.org/ishelp/topic_setup_signeduninstaller.htm)配置，具体签名方式取决于证书提供方。上面的脚本只生成用于本地验证的未签名安装器。若应用会自行更新，检查安装器与更新器是否使用同一应用身份和文件位置；脚本字段见 [Inno Setup 官方帮助](https://jrsoftware.org/ishelp/topic_setupsection.htm)。

## Linux

tar.gz 适合明确说明“解压后运行”或配套安装脚本的分发；桌面菜单、图标、权限及升级规则需要由安装脚本处理。DEB 则由包管理器安装到系统路径。为下面的普通用户安装示例，先准备自己的 256 × 256 PNG 图标 `packaging/hello-world-gpui.png`，再创建归档：

```sh
mkdir -p dist/portable/HelloWorld/bin \
  dist/portable/HelloWorld/share/icons/hicolor/256x256/apps
cp target/release/hello_world dist/portable/HelloWorld/bin/hello_world
install -m 0644 packaging/hello-world-gpui.png \
  dist/portable/HelloWorld/share/icons/hicolor/256x256/apps/hello-world-gpui.png
tar -czf dist/hello-world-linux-x64.tar.gz -C dist/portable HelloWorld
tar -tzf dist/hello-world-linux-x64.tar.gz
```

该归档包含示例二进制与应用提供的图标；如果应用还依赖外部资源，应先复制到 `dist/portable/HelloWorld` 中，再创建归档。解压到其他目录运行，以排除对源码目录与当前工作目录的偶然依赖。

**普通用户目录安装。** 如果希望 tar.gz 也能在无需 sudo 的情况下进入应用菜单，可随发布产物提供 `install.sh`：先从可信发布渠道取得压缩包和对应的 SHA-256 校验值，再校验完整压缩包，检查归档路径及必需文件，最后把程序、`.desktop` 入口和图标安装到当前用户目录。下面的脚本需要 Python 3，不需要 root 权限。它使用 XDG 用户数据目录的默认位置 `~/.local/share`，把入口放在 `~/.local/share/applications`，图标放在 `~/.local/share/icons/hicolor/256x256/apps`，程序放在 `~/.local/bin`；这些默认位置见 [XDG Base Directory 规范](https://specifications.freedesktop.org/basedir/latest/)。若用户另行设置 `XDG_DATA_HOME` 或自定义安装前缀，应把脚本中的 `data_dir` 改为配置的绝对路径，并确保桌面环境能够发现该目录。

发布用归档应包含 `HelloWorld/bin/hello_world` 和应用提供的 `HelloWorld/share/icons/hicolor/256x256/apps/hello-world-gpui.png`。现有仓库的 `hello_world` 仅提供二进制；请先给自己的应用准备图标。校验值应来自可信的发布清单或独立发布渠道；从刚下载的同一份压缩包现场计算一个值再传给安装脚本，不能验证它是否是发布者预期的文件。

将下面内容保存为 `install.sh`。它接受本地 tar.gz 和发布方给出的**确切 SHA-256 值**作为参数，只提取所需的普通文件；遇到绝对路径、`..`、链接或重复条目会在安装前拒绝归档。示例安装三个文件，不包含应用额外资源；实际应用若要加载外部文件，需要扩展归档布局和安装步骤。

```sh
#!/bin/sh
set -eu
[ "$#" -eq 2 ] || { echo 'Usage: sh install.sh ARCHIVE EXPECTED_SHA256' >&2; exit 2; }
python3 - "$1" "$2" <<'PY'
from pathlib import Path, PurePosixPath
import hashlib
import io
import os
import re
import shutil
import sys
import tarfile
import tempfile

archive = Path(sys.argv[1])
expected = sys.argv[2].lower()
if not re.fullmatch(r"[0-9a-f]{64}", expected):
    raise SystemExit("Expected a 64-character SHA-256 digest")
digest = hashlib.sha256()
verified = tempfile.TemporaryFile()
with archive.open("rb") as archive_stream:
    for chunk in iter(lambda: archive_stream.read(1024 * 1024), b""):
        digest.update(chunk)
        verified.write(chunk)
if digest.hexdigest() != expected:
    raise SystemExit("Checksum mismatch; nothing installed")
verified.seek(0)

binary_name = "HelloWorld/bin/hello_world"
icon_name = "HelloWorld/share/icons/hicolor/256x256/apps/hello-world-gpui.png"
with tarfile.open(fileobj=verified, mode="r:gz") as bundle:
    members = {}
    for member in bundle:
        path = PurePosixPath(member.name)
        if (not path.parts or path.is_absolute() or ".." in path.parts
                or path.parts[0] != "HelloWorld"
                or not (member.isfile() or member.isdir())
                or str(path) in members):
            raise SystemExit("Unsafe or duplicate archive entry")
        members[str(path)] = member
    if not all(name in members and members[name].isfile()
               for name in (binary_name, icon_name)):
        raise SystemExit("Archive is missing its executable or icon")

    bin_dir = Path.home() / ".local/bin"
    data_dir = Path.home() / ".local/share"
    app_dir = data_dir / "applications"
    icon_dir = data_dir / "icons/hicolor/256x256/apps"
    for directory in (bin_dir, app_dir, icon_dir):
        directory.mkdir(parents=True, exist_ok=True)

    binary = bin_dir / "hello_world"
    icon = icon_dir / "hello-world-gpui.png"
    desktop = app_dir / "hello-world-gpui.desktop"
    # Desktop Entry quoting has two escape layers; this handles a home path with spaces.
    exec_path = str(binary).replace("\\", "\\\\\\\\")
    for character in ('"', '`', '$'):
        exec_path = exec_path.replace(character, "\\\\" + character)
    exec_path = exec_path.replace("%", "%%")
    desktop_content = (
        "[Desktop Entry]\nType=Application\nName=HelloWorld\n"
        + f'Exec="{exec_path}"\n'
        + "Icon=hello-world-gpui\nTerminal=false\nCategories=Utility;\n"
    )

    staged_files = []

    def stage_file(destination, input_stream, mode):
        with tempfile.NamedTemporaryFile(
            dir=destination.parent, prefix=".hello-world-install-", delete=False
        ) as staged:
            staged_path = Path(staged.name)
            staged_files.append(staged_path)
            shutil.copyfileobj(input_stream, staged)
        staged_path.chmod(mode)
        return staged_path

    try:
        with bundle.extractfile(members[binary_name]) as entry_stream:
            staged_binary = stage_file(binary, entry_stream, 0o755)
        with bundle.extractfile(members[icon_name]) as entry_stream:
            staged_icon = stage_file(icon, entry_stream, 0o644)
        staged_desktop = stage_file(
            desktop, io.BytesIO(desktop_content.encode("utf-8")), 0o644
        )
        for staged, destination in (
            (staged_binary, binary),
            (staged_icon, icon),
            (staged_desktop, desktop),
        ):
            os.replace(staged, destination)
    finally:
        for staged in staged_files:
            staged.unlink(missing_ok=True)
verified.close()
print("Installed HelloWorld for the current user")
PY
```

在发布流水线中为最终 tar.gz 生成 SHA-256 校验值并随产物发布；安装时从可信发布清单取得对应值。第二个参数须替换为实际的 64 位十六进制值：

```sh
sh install.sh dist/hello-world-linux-x64.tar.gz SHA256_FROM_TRUSTED_RELEASE
```

脚本生成的桌面入口使用安装后二进制的绝对路径，因此不依赖图形会话的 `PATH` 是否包含 `~/.local/bin`。路径中有空格等特殊字符时，应遵循 [Desktop Entry 规范](https://specifications.freedesktop.org/desktop-entry/latest-single/)的 `Exec` 引号与转义规则。安装后从应用菜单启动，确认图标和入口都已出现；部分桌面环境还需要重新登录或刷新菜单缓存。升级时先退出应用，再对新版本重复执行脚本；脚本先在各目标目录暂存三个文件，再分别重命名替换。每个文件的替换都是原子操作，但三个文件合起来并不是一次事务。这个最小示例可使用下列命令卸载三个安装文件，用户配置与数据默认保留：

```sh
rm ~/.local/bin/hello_world \
  ~/.local/share/applications/hello-world-gpui.desktop \
  ~/.local/share/icons/hicolor/256x256/apps/hello-world-gpui.png
```

若改用 DEB 等系统包管理器安装，不要让两套安装方式同时管理同一条入口或同一份文件。

**Debian/Ubuntu 系统包。** DEB 可按以下结构暂存：

```text
dist/deb-root/
├── DEBIAN/
│   └── control
└── usr/
    ├── lib/hello-world-gpui/
    │   └── hello_world
    └── share/
        ├── applications/hello-world-gpui.desktop
        └── icons/hicolor/256x256/apps/hello-world-gpui.png
```

先在 Linux 构建机建立暂存目录并复制示例二进制；图标由应用自行提供：

```sh
mkdir -p dist/deb-root/DEBIAN \
  dist/deb-root/usr/lib/hello-world-gpui \
  dist/deb-root/usr/share/applications \
  dist/deb-root/usr/share/icons/hicolor/256x256/apps
install -m 0755 target/release/hello_world \
  dist/deb-root/usr/lib/hello-world-gpui/hello_world
```

不要让 DEB 写入 `/usr/local`；该目录留给本机管理员。`.desktop` 文件中的 `Exec` 应指向已安装的命令，`Icon` 应与已安装的图标名匹配。示例控制文件内容如下；发布前依据实际链接库填写 `Depends`，不要复制开发机的构建包列表：

```text
Package: hello-world-gpui
Version: 0.6.5
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Example <maintainer@example.com>
Description: HelloWorld desktop application
```

```ini
[Desktop Entry]
Type=Application
Name=HelloWorld
Exec=/usr/lib/hello-world-gpui/hello_world
Icon=hello-world-gpui
Categories=Utility;
Terminal=false
```

把两个文本分别保存到上面的 `control` 与 `.desktop` 路径，复制二进制和图标，设置可执行权限，然后在 Linux 构建机创建并查看包：

```sh
dpkg-deb --root-owner-group --build dist/deb-root dist/hello-world-gpui_0.6.5_amd64.deb
dpkg-deb --info dist/hello-world-gpui_0.6.5_amd64.deb
dpkg-deb --contents dist/hello-world-gpui_0.6.5_amd64.deb
```

用 `readelf -d` 或 `objdump -p` 查看自己的二进制直接依赖的共享库，再用 Debian 的 [`dpkg-shlibdeps`](https://manpages.debian.org/unstable/dpkg-dev/dpkg-shlibdeps.1.en.html) 和目标发行版包信息推导运行时 `Depends`；如果运行时才加载库，也要检查对应路径。桌面入口遵循 [Desktop Entry 规范](https://specifications.freedesktop.org/desktop-entry/latest-single/)，图标放置及系统目录遵循 [Debian Policy](https://www.debian.org/doc/debian-policy/ch-opersys.html)。若还要发布 RPM，应另行制作并测试 RPM，不要直接改 DEB 的扩展名。

`--root-owner-group` 让无 root 权限的暂存构建将包内文件归属设为 root，避免把构建用户的 UID/GID 写进 DEB；参见 [`dpkg-deb` 手册](https://manpages.debian.org/bullseye/dpkg/dpkg-deb.1.en.html)。在干净的 Debian/Ubuntu 测试机使用 `sudo apt install ./dist/hello-world-gpui_0.6.5_amd64.deb` 安装，从桌面菜单启动，再使用 `sudo apt remove hello-world-gpui` 检查卸载。

## 发布前验证

在与开发机分开的干净虚拟机或设备上，分别验证安装包和归档：

1. **检查内容**：核对版本、架构、入口文件、资源、许可文本、签名和依赖；生成 SHA-256 校验值并随产物发布。不要只依据文件存在或构建成功判断可运行。
2. **安装并启动**：从最终用户会采用的路径安装；检查窗口、文字与图标、键盘输入、系统菜单，以及应用实际使用的通知、WebView、URL scheme 或文件打开功能。
3. **升级与卸载**：用上一版升级到当前版，再卸载；确认应用身份保持稳定、用户数据处理符合产品设计，并且快捷方式、桌面入口与安装文件正确更新或清理。
4. **架构与兼容性**：每个目标架构、受支持的最低系统版本和所需图形环境至少测一次。Linux 应覆盖所声明的发行版和图形会话；Windows 应覆盖安装权限模式；macOS 应验证首次启动和公证状态。

这份指南展示发布流程和最小示例。安装器配置、签名凭据、依赖清单及平台集成仍应由具体应用维护；如需自动化，把打包与验证放在应用自己的发布流水线中，避免把仅用于编译的检查当成安装包测试。
安装包验证完成后，可继续阅读[自动更新](./auto-update)，为不同安装来源设计版本检查与升级流程。
