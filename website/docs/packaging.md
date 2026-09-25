---
title: Packaging
description: Turn a GPUI Kit release build into a macOS app and DMG, a Windows installer, or a Linux tarball and DEB.
order: -10.5
---

# Packaging Desktop Apps

`cargo build --release` produces an executable, not an installer. GPUI Kit supplies the application and UI layers; your application owns its package identity, icons, external files, installer, signing, and release tests. Start with [Installation](./installation.md) and [Getting Started](./getting-started.md), then build and package **on each target platform**. This guide uses the repository's `hello_world` binary to make paths concrete. Replace `hello_world`, `HelloWorld`, the sample version, and `com.example.helloworld` with your application's values before shipping.

## Choose the artifact

| Platform | Portable artifact | Installed artifact | What changes |
| --- | --- | --- | --- |
| macOS | A `.app` inside a ZIP or tarball | A `.app` distributed in a `.dmg` | Finder recognizes the app bundle; direct distribution needs Developer ID signing and notarization. A DMG is a delivery container, not an installer that writes files automatically. |
| Windows | ZIP containing an `.exe` and its files | An Inno Setup `.exe` installer | The installer registers the app, shortcuts, and uninstall behavior. Renaming a ZIP to `.exe` does not provide that behavior. |
| Linux | `.tar.gz` with the binary and resources | Distribution package such as `.deb` | A DEB declares package identity and dependencies, installs a desktop entry and icon, and supports package manager upgrade/removal. A tarball needs an explicit install process. |

Choose only the formats your users need. A `.deb` does not cover RPM distributions, and a Windows ZIP is useful for portable use even when you also provide an installer. [Apple's distribution guide](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution), [Microsoft's packaging overview](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/packaging/), and the [Debian binary package guide](https://www.debian.org/doc/debian-policy/ap-pkg-binarypkg.html) describe the platform rules.

## Build and inventory the application

From this repository's root, build the existing example on the platform being packaged:

```sh
cargo build --release --locked -p hello_world
```

The output is `target/release/hello_world` on macOS/Linux or `target\release\hello_world.exe` on Windows. In your own application, run `cargo build --release --locked` from its project directory and use its binary name. Keep the `Cargo.lock` used to build the release, and record the target OS, CPU architecture, app version, and build revision with the artifact. Build separately for each architecture you publish; a successful x86-64 build does not establish an Arm build.

Before packaging, list everything the program opens **at runtime**:

- GPUI Kit's default native icon assets are embedded in the binary. A custom `AssetSource` may instead read files at runtime. Assets addressed through a filesystem `Path` need to be copied into the package and resolved relative to a stable installed location; see [Icons & Assets](./assets.md).
- Fonts inserted with `include_bytes!` are compiled into the executable. Fonts opened from disk must travel with it, with their licenses; see [Fonts](./fonts.md).
- Include other required files such as configuration templates, translations, helper executables, and license notices. Keep mutable user data outside the installed application directory.
- Review native dynamic libraries and system services used by *your* feature set. A compile-time development package is not necessarily a runtime dependency. For example, a Linux GPUI window needs a working graphical session and Vulkan driver; a machine with only the Vulkan loader cannot render it. See [Installation](./installation.md).

Give the app a stable identity before publishing it. The visible name, executable name, platform package identifier, update channel, and data directory policy should agree across releases. If the app uses GPUI's application identity for notifications or related platform integration, set it at startup with `cx.set_app_identity("com.example.helloworld", "HelloWorld")`; see [System Notifications](./system-notification.md). Package identity and GPUI runtime identity serve different APIs, so check both. A new identifier can create a second installation or separate settings/notification identity rather than upgrading the old one.

## macOS

A macOS application is a directory with a defined bundle layout: `Contents/MacOS` holds the executable, `Contents/Resources` holds resources, and `Contents/Info.plist` declares the executable and bundle identifier. [Apple documents the bundle structure](https://developer.apple.com/documentation/bundleresources/placing-content-in-a-bundle). On a Mac, assemble a minimal local test bundle from this repository's example:

```sh
mkdir -p dist/HelloWorld.app/Contents/MacOS dist/HelloWorld.app/Contents/Resources
cp target/release/hello_world dist/HelloWorld.app/Contents/MacOS/hello_world
cat > dist/HelloWorld.app/Contents/Info.plist <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>HelloWorld</string>
  <key>CFBundleDisplayName</key><string>HelloWorld</string>
  <key>CFBundleIdentifier</key><string>com.example.helloworld</string>
  <key>CFBundleExecutable</key><string>hello_world</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.6.5</string>
  <key>CFBundleVersion</key><string>1</string>
</dict></plist>
PLIST
plutil -lint dist/HelloWorld.app/Contents/Info.plist
open dist/HelloWorld.app
```

That bundle has no custom icon. For your app, add an `.icns` in `Contents/Resources`, declare `CFBundleIconFile`, and copy external files under `Resources`; resolve them from the bundle location rather than the caller's working directory. Update both version fields for each release. The example above is a **local bundle test**, not a signed public release.

For distribution outside the Mac App Store, sign the final app with a Developer ID Application certificate and hardened runtime, then create and sign a DMG. Sign any nested executable code from the inside out before signing the app; do not treat `codesign --deep` as a substitute for that signing order. Verify the signed app before creating the DMG. Apple's [signing guide](https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac/) explains these requirements. A representative final sequence is:

```sh
codesign --force --timestamp --options runtime --sign "Developer ID Application: Your Organization" dist/HelloWorld.app
codesign --verify --deep --strict --verbose=2 dist/HelloWorld.app
mkdir -p dist/dmg-root
cp -R dist/HelloWorld.app dist/dmg-root/HelloWorld.app
hdiutil create -volname HelloWorld -srcfolder dist/dmg-root -ov -format UDZO dist/HelloWorld.dmg
codesign --force --timestamp --sign "Developer ID Application: Your Organization" dist/HelloWorld.dmg
xcrun notarytool submit dist/HelloWorld.dmg --keychain-profile "notary-profile" --wait
xcrun stapler staple dist/HelloWorld.dmg
xcrun stapler validate dist/HelloWorld.dmg
```

Replace the signing identity and `notary-profile` with credentials you provision in your own release process. Check that `notarytool` reports **Accepted**; `--wait` alone is not a success criterion. Investigate its submission log if rejected. Staple and validate the accepted DMG, then download and launch that exact artifact on a clean Mac. An ad hoc signature (`--sign -`) only seals code for local use; it does not replace Developer ID signing and notarization for direct distribution. See Apple's [notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).

## Windows

Build with the MSVC Rust toolchain and Windows SDK described in [Installation](./installation.md). Stage the executable and any runtime files first. A ZIP can be enough for a portable app:

```powershell
New-Item -ItemType Directory -Force dist\windows | Out-Null
Copy-Item target\release\hello_world.exe dist\windows\hello_world.exe
Compress-Archive -Path dist\windows\* -DestinationPath dist\hello-world-windows-x64.zip -Force
```

Test by extracting the ZIP into a fresh directory and launching the extracted `.exe`; do not test only `target\release`. A ZIP does not add an uninstall entry or Start menu shortcut. If your app needs those, build an `.exe` installer with [Inno Setup](https://jrsoftware.org/isinfo.php). Save this minimal **unsigned local example** as `packaging.iss` in the project root, after staging `dist\windows\hello_world.exe`:

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

Keep `AppId` stable for upgrades. Add your own icon, publisher metadata, license, files, and install scope as needed. For a signed release, sign the application executable **before** placing it in either the ZIP or installer, then sign the finished setup `.exe`. Inno Setup's uninstaller is embedded in that `.exe`, so signing the setup afterward does **not** sign the uninstaller; configure Inno's [signed uninstaller workflow](https://jrsoftware.org/ishelp/topic_setup_signeduninstaller.htm) or an equivalent two-pass build. Check all signatures and install/uninstall behavior on a clean Windows VM. The exact signing mechanism depends on your certificate provider; it is not supplied by GPUI Kit.

## Linux

For a portable tarball, stage the executable, any external files, license notices, and a launcher that resolves files from its own directory. Add your own 256 × 256 PNG at `packaging/hello-world-gpui.png`, then make an archive for the minimal example:

```sh
mkdir -p dist/portable/HelloWorld/bin \
  dist/portable/HelloWorld/share/icons/hicolor/256x256/apps
cp target/release/hello_world dist/portable/HelloWorld/bin/hello_world
install -m 0644 packaging/hello-world-gpui.png \
  dist/portable/HelloWorld/share/icons/hicolor/256x256/apps/hello-world-gpui.png
tar -czf dist/hello-world-linux-x64.tar.gz -C dist/portable HelloWorld
tar -tzf dist/hello-world-linux-x64.tar.gz
```

Extract the archive somewhere else and launch the executable there. For an app with external files, copy them under `HelloWorld` before archiving and locate them from the executable or launcher, not the shell's current directory. A bare Rust binary may run on your build machine while missing shared libraries or a usable GPU driver on another. Inspect direct ELF requirements with `objdump -p target/release/hello_world` and test on the oldest distribution you support. If you use `ldd`, run it only on binaries you built or trust.

For users who want a desktop menu entry without a system package, publish an `install.sh` alongside the tarball. The following small installer accepts a **local** archive and its expected SHA-256 digest as two arguments. Give users the digest through a trusted release channel; a checksum downloaded from the same untrusted location as a replaced archive does not authenticate that archive. The script verifies the entire archive before changing installed files, rejects unsafe paths and links, requires the binary and icon, then installs into the current user's default XDG locations. It needs Python 3 and no root access:

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

Save the code as `install.sh`. Run `sh install.sh dist/hello-world-linux-x64.tar.gz SHA256_FROM_TRUSTED_RELEASE`, replacing the final argument with the archive's real 64-character digest. The example uses `~/.local/bin` for the executable and the default XDG data directory `~/.local/share` for the desktop file and icon; [the XDG Base Directory specification](https://specifications.freedesktop.org/basedir/latest/) defines these defaults. If your desktop uses a custom XDG data home, adapt `data_dir` to that configured **absolute** directory before using the script. The desktop entry uses an absolute executable path, so the menu does not depend on `~/.local/bin` being on the desktop session's `PATH`. See the [Desktop Entry specification](https://specifications.freedesktop.org/desktop-entry/latest-single/) for `Exec` quoting and icon lookup.

Close the app and rerun the installer with a verified newer archive to upgrade. Replacing the binary within one directory is atomic, while updating the binary, icon, and desktop entry together is **not** one transaction. For this minimal example, uninstall with `rm ~/.local/bin/hello_world ~/.local/share/applications/hello-world-gpui.desktop ~/.local/share/icons/hicolor/256x256/apps/hello-world-gpui.png`; leave user settings and documents in their own directories. An application with additional runtime files needs an installer that stages and updates those files too.

For Debian/Ubuntu installation, stage files under a package root. The following tree shows the key locations; copy your actual binary into `usr/lib/hello-world-gpui/`, your icon into the hicolor icon tree, and your desktop entry into `usr/share/applications/`:

```text
dist/deb-root/
├── DEBIAN/control
└── usr/
    ├── lib/hello-world-gpui/hello_world
    └── share/
        ├── applications/hello-world-gpui.desktop
        └── icons/hicolor/256x256/apps/hello-world-gpui.png
```

Create the staging directories and copy the example binary on a Linux build machine; supply your own application icon:

```sh
mkdir -p dist/deb-root/DEBIAN \
  dist/deb-root/usr/lib/hello-world-gpui \
  dist/deb-root/usr/share/applications \
  dist/deb-root/usr/share/icons/hicolor/256x256/apps
install -m 0755 target/release/hello_world \
  dist/deb-root/usr/lib/hello-world-gpui/hello_world
```

Use a stable package name and an architecture/version that match the binary. A minimal `DEBIAN/control` for the sample is:

```text
Package: hello-world-gpui
Version: 0.6.5
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Example Maintainer <maintainer@example.com>
Description: A GPUI Kit desktop example
```

Add the runtime `Depends` derived from your **built application** and distribution baseline; `dpkg-shlibdeps` can help derive shared-library dependencies. Do not copy another application's dependency list. A desktop entry for the staged paths is:

```ini
[Desktop Entry]
Type=Application
Name=HelloWorld
Exec=/usr/lib/hello-world-gpui/hello_world
Icon=hello-world-gpui
Terminal=false
Categories=Utility;
```

When all staged files are present, build and inspect the package. The [documented `--root-owner-group` option](https://manpages.debian.org/unstable/dpkg/dpkg-deb.1.en.html) records root ownership for staged package files even when you build as a regular user; use it when all packaged files should be root-owned.

```sh
dpkg-deb --root-owner-group --build dist/deb-root dist/hello-world-gpui_0.6.5_amd64.deb
dpkg-deb --info dist/hello-world-gpui_0.6.5_amd64.deb
dpkg-deb --contents dist/hello-world-gpui_0.6.5_amd64.deb
```

Install a copy on a clean test machine with `sudo apt install ./dist/hello-world-gpui_0.6.5_amd64.deb`, launch it from the desktop menu, then remove it with `sudo apt remove hello-world-gpui`. [Debian policy](https://www.debian.org/doc/debian-policy/ch-opersys.html) prohibits packages from placing their files in `/usr/local`; the [Desktop Entry specification](https://specifications.freedesktop.org/desktop-entry/latest-single/) defines the launcher keys. Use a separate RPM packaging recipe for RPM distributions.

## Verify the release artifact

Do the final checks against the **downloaded package**, not the original build directory:

1. Confirm filename, embedded app version, architecture, package identity, and checksum match the intended release. Keep one version source of truth where possible.
2. Inspect the archive/package contents for the executable, icon, external assets, required notices, and accidental development files. Extract it to a new location; launch from there without relying on the source tree or working directory.
3. On a clean machine or VM for each supported OS/architecture, check first launch, window creation, text and icons, keyboard and pointer input, notifications or WebView if used, and behavior without a network connection if offline use is promised. Linux tests need a real graphical session and Vulkan driver.
4. Install version N, install N+1 over it, and check both application data and shortcuts. Then uninstall and check which files remain by design. Test portable archives separately because they have no package-managed upgrade or removal.
5. Verify signatures and notarization where applicable, publish SHA-256 checksums, and download the artifact again to compare its checksum before announcing it.

Package format does not guarantee platform support for every optional feature. In particular, check the [WebView](./webview.md) target limitations before promising a Linux WebView build.
For in-app version checks and upgrades after packaging, continue with [Auto Update](./auto-update.md).
