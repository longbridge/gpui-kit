---
title: Packaging
description: Ship a script application as a desktop application — `gpui-shell bundle` makes a small macOS .app or Linux archive that uses an installed gpui-shell or downloads it, or a portable Windows folder, and a standalone package can carry the binary itself.
order: 16
maturity: [preview]
---

# Packaging

A script application runs on the prebuilt `gpui-shell` binary. Nothing is compiled, so packaging is a matter of where that binary comes from:

- **`gpui-shell bundle`** makes a bundle for macOS, Linux or Windows. The macOS and Linux bundles leave the binary out: on the user's machine they use an installed `gpui-shell`, or download the release they were made with, once, for every bundled application to share, so they are about the size of the application itself. The Windows bundle is a portable folder that carries the binary as the application's executable.
- **A standalone package**, assembled by hand, carries the binary beside the application, for an installer or a layout of your own. See [Standalone packages](#standalone-packages).

## Before packaging

The application needs a manifest with a stable `id`, because a packaged application keeps its storage under that `id` rather than under its path: moving the `.app` or installing an update does not lose the user's data.

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

Its `capabilities` are granted as written, so ask only for what the application uses — see [Capabilities](./capabilities.md). Run `gpui-shell check -d clock` before packaging.

`gpui-shell new clock` creates a directory in this shape to start from.

## Bundling with one command

```bash
gpui-shell bundle -d clock
```

| Target    | Output in `dist/`                                   | What it holds                                                                           |
| --------- | --------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `macos`   | `Clock.app`                                         | `Info.plist`, a launcher as `Contents/MacOS/Clock`, the application in `Resources/app`   |
| `linux`   | `clock-1.0.0-linux.tar.gz` and `install.sh`         | the archive: the launcher `clock` and `app/`; the installer beside it, outside the archive |
| `windows` | `clock-1.0.0-windows-x86_64.zip`                    | `Clock.exe` — `gpui-shell.exe` itself — and `app\`                                      |

The macOS and Linux bundles hold no `gpui-shell` binary, so they can be made on any platform and do not depend on the CPU architecture. The Windows bundle carries `gpui-shell.exe` for x86-64, which makes its zip about 20 MB; see [Windows](#windows) for where `bundle` gets it.

The names come from the manifest: `name` names the `.app`, the Windows executable and the window title, a lowercase form of it (`clock`) names the command and the archives, `version` goes into the archive names and `Info.plist`, and `id` is the bundle identifier, the Linux install directory and desktop entry, and the window's application id. A bundle needs `gpui-shell.json` for all of this.

| Option                  | Meaning                                                                                   |
| ----------------------- | ----------------------------------------------------------------------------------------- |
| `<name>`, `-d <dir>`    | The application, as for `run`; the current directory when omitted                        |
| `--platform <platform>` | `macos`, `linux` or `windows`; the platform the command runs on when omitted             |
| `--icon <png>`          | A square PNG, ideally 1024×1024                                                           |
| `--out <dir>`           | Where the bundle is written; `dist` in the current directory when omitted                 |
| `--download-url <url>`  | Linux only: where the archive will be published, so `install.sh` alone can download it    |

The application is copied without what only development needs: hidden files and directories, `node_modules/`, every `gpui-kit.d.ts`, and the root `jsconfig.json` and `AGENTS.md`. The output directory is left out too, so `dist/` may sit inside the application — `gpui-shell new` lists it in `.gitignore`. Everything else in the directory goes in, so keep tests and scripts that the application does not load elsewhere.

### The runtime

The macOS and Linux bundles need `gpui-shell` at the version that made them or later: a bundle made with `gpui-shell` 0.8.0 accepts 0.8.0 and anything newer. When the bundle is installed (Linux) or opened (macOS), it looks for one, in order: `~/.local/bin/gpui-shell`, `gpui-shell` on `PATH`, `/opt/homebrew/bin`, `/usr/local/bin`, `~/.cargo/bin`.

When none is recent enough, it downloads the matching release from GitHub, checks it against the release's `.sha256`, and installs it as `~/.local/bin/gpui-shell` — the same place the [install script](./getting-started.md) uses — without sudo. A newer runtime installed later is used as is; the bundle never downgrades one.

The first installation or launch therefore needs a network connection unless a recent enough `gpui-shell` is already installed. For machines without one, install `gpui-shell` first, from a release archive copied over.

The launcher starts the application with `gpui-shell run --dir <app> --packaged`. `--packaged` keeps the application's data under its manifest `id` wherever the directory is, titles the window with the manifest `name`, and writes no editor files into the directory, so a signed bundle stays sealed and a read-only install location works. `--packaged` arrives in the same release as `bundle`, so any runtime recent enough for a bundle has it.

### macOS

`Clock.app` installs by being dragged into Applications. Its executable is a launcher script: on first open it finds or downloads `gpui-shell`, showing a notification while it downloads and an alert if it cannot, then hands the application to it. `--icon` becomes `Clock.icns` with every standard size up to the image's.

The `.app` is not signed: signing needs your identity, so sign and notarize it as [Signing on macOS](#signing-on-macos) describes before giving it to anyone.

### Linux

`bundle` writes two files: the archive, and `install.sh` beside it. The archive alone is enough to run the application where it is unpacked:

```bash
tar -xzf clock-1.0.0-linux.tar.gz
./clock-1.0.0-linux/clock
```

`install.sh` installs it for the current user. It holds the archive's SHA-256 and installs only an archive that matches, from the first of:

1. the archive or URL given as its argument;
2. `clock-1.0.0-linux.tar.gz` beside the script;
3. the `--download-url` the bundle was made with, followed by the archive's name.

```bash
sh install.sh                                         # the archive beside it
curl -fsSL https://example.com/clock/install.sh | sh  # made with --download-url https://example.com/clock
```

For the second form, publish `install.sh` and the archive together at that URL — a release's assets, or any static host. `install.sh` makes sure `gpui-shell` is available first, so the first launch does not wait for a download, then writes only under `~/.local`, without sudo:

| Path                                                    | What                                       |
| ------------------------------------------------------- | ------------------------------------------ |
| `~/.local/share/com.example.clock/`                     | the launcher and `app/`                    |
| `~/.local/bin/clock`                                    | a link to the launcher                     |
| `~/.local/share/applications/com.example.clock.desktop` | the launcher entry                         |
| `~/.local/share/icons/hicolor/<size>x<size>/apps/`      | the icon, with `--icon`                    |

Running it again updates the installed copy: the new copy is made first, and the old one is restored if the swap fails. It refuses to replace a linked installation or a `~/.local/bin/clock` it did not create. `sh install.sh --uninstall` removes these files; the application's saved data, kept under its `id`, and the shared `gpui-shell` stay.

### Windows

`clock-1.0.0-windows-x86_64.zip` is a portable folder: the user unzips it anywhere and opens `Clock.exe`. Nothing is installed, and no administrator rights are needed.

```text
clock-1.0.0-windows-x86_64\
├── Clock.exe                 # gpui-shell.exe, marked as a GUI program
└── app\
    ├── gpui-shell.json
    └── main.js
```

`Clock.exe` is the release's `gpui-shell.exe` of the version doing the bundling, with one change: it is marked as a GUI program, so opening it shows no console window. It finds `app\` beside itself and keeps the application's data under its `id`, so moving the folder or replacing it with a newer one keeps the data. `--icon` is not embedded: the executable keeps `gpui-shell`'s icon. To sign it, run `signtool sign /fd SHA256 /a Clock.exe` with your certificate after bundling.

On Windows, `bundle` copies the running `gpui-shell.exe`. On macOS and Linux it downloads the release `v<version>` of `gpui-shell-x86_64-pc-windows-msvc.zip` from GitHub, checks it against its `.sha256`, and caches the executable as `~/.gpui-shell/cache/runtime/v<version>/x86_64-pc-windows-msvc/gpui-shell.exe` (under `GPUI_SHELL_HOME` when that is set). To bundle for Windows without a network connection, put that release's `gpui-shell.exe` there first.

## Signing on macOS

A bundle given to other people has to be signed and notarized, or Gatekeeper refuses to open it. Sign everything the bundle contains — the scripts are part of it:

```bash
codesign --force --options runtime --timestamp \
  --sign "Developer ID Application: Example Inc (TEAMID)" Clock.app
ditto -c -k --keepParent Clock.app Clock.zip
xcrun notarytool submit Clock.zip --keychain-profile notary --wait
xcrun stapler staple Clock.app
```

Signing seals the scripts too, so any change to `Resources/app` after signing breaks the signature: sign last.

## Standalone packages

`bundle` puts the binary only in the portable Windows folder. To ship it in a macOS or Linux package — for a machine that cannot download, or for an installer of your own — assemble the package by hand: the prebuilt `gpui-shell` binary and the application's directory side by side. The binary looks for an application next to itself when it is started with no arguments, which is how a desktop launcher — Finder, a `.desktop` entry, a Start menu shortcut — starts it, since none of them can pass a path. Such an application is treated the way `--packaged` treats one.

| Platform | Where the executable is            | Where the application goes          |
| -------- | ---------------------------------- | ----------------------------------- |
| macOS    | `Clock.app/Contents/MacOS/`        | `Clock.app/Contents/Resources/app/` |
| Linux    | the package directory              | `app/` beside the executable        |
| Windows  | the package directory              | `app\` beside the executable        |

The executable may be renamed — `Clock` instead of `gpui-shell` — because the application is found by where the executable is, not by what it is called. Leave out what only development needs, as `bundle` does.

### Getting the binary

Each GitHub release carries one archive per platform, with a `.sha256` beside it:

| Target                      | Asset                                        |
| --------------------------- | -------------------------------------------- |
| macOS, Apple silicon        | `gpui-shell-aarch64-apple-darwin.tar.gz`     |
| macOS, Intel                | `gpui-shell-x86_64-apple-darwin.tar.gz`      |
| Linux, x86-64               | `gpui-shell-x86_64-unknown-linux-gnu.tar.gz` |
| Windows, x86-64             | `gpui-shell-x86_64-pc-windows-msvc.zip`      |

```bash
curl -fsSLO https://github.com/longbridge/gpui-kit/releases/latest/download/gpui-shell-aarch64-apple-darwin.tar.gz
tar -xzf gpui-shell-aarch64-apple-darwin.tar.gz
```

Pin a version in a build script by replacing `latest/download` with `download/v0.7.3`. To build it yourself instead, run `cargo build -p gpui-shell --release` in the repository; the binary is `target/release/gpui-shell`.

### macOS

```text
Clock.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── Clock              # the gpui-shell binary, renamed
    └── Resources/
        ├── Clock.icns
        └── app/
            ├── gpui-shell.json
            └── main.js
```

`Info.plist` names the executable and the icon. Use the manifest's `id` as the bundle identifier so the two stay one identity:

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

Assembling it is a few copies:

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

`iconutil -c icns Clock.iconset` makes `Clock.icns` from a folder of PNG images. For one bundle that runs on both Apple silicon and Intel, merge the two release binaries into the executable instead of copying one:

```bash
lipo -create -output "$app/Contents/MacOS/Clock" \
  gpui-shell-aarch64-apple-darwin/gpui-shell \
  gpui-shell-x86_64-apple-darwin/gpui-shell
```

Then sign and notarize it as [Signing on macOS](#signing-on-macos) describes.

### Linux

```text
clock/
├── clock                     # the gpui-shell binary, renamed
├── clock.png
└── app/
    ├── gpui-shell.json
    └── main.js
```

Ship the directory as an archive, and install a desktop entry as `~/.local/share/applications/com.example.clock.desktop` (or under `/usr/share/applications` for every user), with `Exec` pointing at the unpacked executable. Name the file after the manifest `id`: the window's application id is that `id`, which is how a Wayland desktop matches the window to its entry.

```ini
[Desktop Entry]
Type=Application
Name=Clock
Exec=/opt/clock/clock
Icon=/opt/clock/clock.png
Categories=Utility;
StartupWMClass=com.example.clock
```

The same layout goes into a `.deb`, an `.rpm` or an AppImage unchanged: what matters is that `app/` stays beside the executable. The binary needs what any GPUI application needs at run time — a Vulkan driver, `libxkbcommon`, and a Wayland or X11 session.

### Windows

```text
Clock\
├── Clock.exe                 # gpui-shell.exe, renamed
└── app\
    ├── gpui-shell.json
    └── main.js
```

`gpui-shell.exe` is a console program, so its commands can print. Copied as it is and started from Explorer or a shortcut, it gets a console window of its own beside the application's; the executable `bundle` writes is marked as a GUI program instead, which this layout otherwise matches. Zip the folder, or hand it to an installer such as Inno Setup or WiX; a shortcut targets `Clock.exe` with no arguments.

## Related

| Page | What it covers |
| --- | --- |
| [Getting Started](./getting-started.md) | Installing the runtime, `run`, `new`, `check`, `bundle` and installed applications |
| [Capabilities](./capabilities.md) | What a manifest may ask for, and what a packaged application is granted |
| [Dependencies](./dependencies.md) | Git dependencies, fetched on first launch into `~/.gpui-shell/cache` |
