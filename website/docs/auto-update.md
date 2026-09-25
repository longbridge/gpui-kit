---
title: Auto Update
description: Plan safe update checks, verified downloads, installation, and restart for GPUI Kit desktop apps.
order: -10.6
---

# Auto Update

An update has two separate jobs: the application presents a useful, responsive experience, and the release system delivers a trustworthy replacement. GPUI Kit gives you the UI, state, and task tools for the first job. **Your application and its distribution system own the update policy, release metadata, verification, installation, and recovery.** GPUI Kit does not select a release or replace an installed app. Start with [Packaging Desktop Apps](./packaging.md): an updater must match the package format and installation scope you ship.

## Choose who owns installation

| Distribution | Recommended update owner | Reason |
| --- | --- | --- |
| A writable, single-binary portable installation | The app can use a binary updater such as `self_update`, with a controlled restart | There is one executable to replace and no package database to keep in sync. |
| macOS `.app` delivered in a DMG | A bundle-aware updater or a new signed and notarized app/DMG | Updating only `Contents/MacOS/<binary>` changes the signed bundle. Replace and validate the complete `.app`. |
| Windows Inno Setup `.exe` installer | The installer | It owns installed files, shortcuts, permissions, version identity, and uninstall records. A running process may lock its executable. |
| Linux DEB/RPM or a managed software repository | The package manager | Replacing its binary behind its back leaves its package database and dependencies inconsistent. |
| Portable archive with several files | An installer or an app-specific staged update for the **whole** directory | A new binary alone may not match its sidecar libraries or resources. |

Do not silently switch an installation from one owner to another. In particular, a binary self-updater does not replace the Inno Setup installation flow, a macOS bundle update, or a Linux package upgrade. A signed update to a portable binary still needs a writable destination and a platform-specific restart strategy.

## Define the release contract

Before building the UI, publish enough information to make one unambiguous decision:

1. **Identity and policy:** stable application/package identity, current version, update channel (`stable` or `beta`, for example), minimum supported version, and whether downgrades are allowed. Compare versions with a defined version scheme; do not compare arbitrary version strings lexically.
2. **Exact target:** OS, CPU architecture, and any relevant ABI or installation type. A macOS Arm artifact must not be offered to an Intel build merely because both are called “macOS.” Match the asset name and the binary path inside its archive to the actual release package.
3. **Release metadata:** version, human-readable notes, artifact URL, size if known, and a digest or signature. Serve metadata and assets over HTTPS and treat redirects and release-host credentials as part of your trust policy. Do not embed a private release token in the distributed executable.
4. **Verification:** check the downloaded bytes against the selected release's expected digest **before** installation, then apply the platform's signature or publisher-authenticity checks. A `SHA256SUMS` file hosted beside the artifact catches corruption and mismatches; by itself it does not prove who published either file. Use a trusted signing key, signed metadata, or the applicable platform signature when publisher authenticity matters.
5. **Atomicity and recovery policy:** stage the artifact outside the live installation, reserve space, verify the full payload, and define what happens if extraction, replacement, relaunch, or the new version's startup fails. Do not claim rollback unless your installer actually retains and restores a known-good version.

The check may run at launch or on a reasonable timer, but network errors should leave the current app usable. A manual **Check for updates** action is useful even if checks are automatic. Cache a last-check time, avoid simultaneous checks, and do not install in the background without the app's stated policy.

## A simple path for a portable Rust executable

[`self_update` 1.3](https://docs.rs/self_update/1.3.0/self_update/) is an optional application dependency, not a GPUI Kit feature. It can discover a release, download an archive, verify a checksum, and replace a single executable. This example uses a public GitHub release with a `SHA256SUMS` asset. Adapt the repository, binary name, archive naming, and target selection to **your** release contract. In your application's `Cargo.toml`:

```toml
[dependencies]
self_update = { version = "1.3", default-features = false, features = ["ureq", "rustls", "github", "archive-tar", "compression-tar-gz", "archive-zip", "compression-zip-deflate", "checksums"] }
```

After the user accepts an available update, run the **blocking** installation function on a background worker, never inside `render` or a GPUI click callback:

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

`Some(version)` means the executable was replaced; `None` means it was already up to date. For a **check only**, build the same updater and call `is_update_available()?`; it returns `Some(release)` or `None` without installing. The [crate's current API](https://docs.rs/self_update/1.3.0/self_update/) also provides verification hooks and progress callbacks. A separate check and install can observe different “latest” releases if a release is published between them; production code should pin the selected release/version and revalidate its exact asset before replacement. A successful update changes the installed executable, **not** the code already running in memory. Show **Restart to finish**, save state, exit cleanly, and relaunch with a strategy suitable for that platform and install location.

The snippet is limited to a writable, single-binary portable app. If an archive also contains required assets, update that payload as a unit. If the app is inside a signed `.app`, an Inno Setup installation directory, DEB, or RPM, use the installation owner from the table instead. `self_update` does not automatically make an arbitrary application update atomic, provide a rollback policy, or preserve package-manager ownership.

## Keep GPUI responsive throughout

Represent each visible phase as application-owned state: `Idle`, `Checking`, `Available`, `Downloading`, `Verifying`, `ReadyToRestart`, and `Failed` are a useful starting set. Keep the selected version and an error message in that state. Use a foreground [Task](./task.md) to coordinate UI updates and a background worker for blocking download, hashing, archive extraction, or installer preparation. Return owned results to the foreground task, update the owning [Entity](./entity), and call `cx.notify()` so a later frame shows the new state. Keep the task handle alive while the operation should continue; cancel or ignore stale results if a newer request supersedes it.

Progress is optional when the server does not provide a trustworthy total size. In that case show an indeterminate status and the current phase. Do not drive an animation or redraw loop while the app is idle; GPUI renders in response to work and invalidation, as explained in [120 FPS and Rendering Models](./fps.md). If you animate progress, honor reduced-motion preferences, and expose phase, percentage when known, errors, and **Restart** through text or accessible status controls. Make a failed download retryable without implying that the existing installation is damaged.

## Install, restart, and recover

1. **Download and stage:** write to a temporary, application-owned location. Reject an unexpected version, target, archive layout, size, or missing required file. Never execute a partially downloaded payload.
2. **Verify:** validate the artifact digest and the publisher/authenticity policy before replacing anything. On macOS, validate the **complete** signed bundle after staging; on Windows, verify the publisher signature or installer according to your release policy. Recheck the exact artifact after any download redirect or cache substitution.
3. **Install through the owner:** let the installer/package manager apply managed updates. For a portable app, arrange replacement after the running process releases files if that platform requires it. Preserve user data outside the installation directory.
4. **Restart deliberately:** tell the user when the new version is ready, save work, and close or relaunch at a safe point. Report the active version only after the new process starts; a completed download is not an active update.
5. **Recover:** retain a known-good package or make the previous installer available, and test the actual failure paths. Rollback may require a platform installer or a separate helper; it cannot be inferred from a successful download or an updater crate call.

For macOS direct distribution, stage and verify a signed/notarized replacement `.app` rather than altering one executable inside the installed bundle. For Windows, test an update while the app is running and under the intended per-user or machine-wide install scope; the helper or installer must handle file locks and elevation rather than assuming the app can overwrite itself. For Linux DEB/RPM, direct the user to the configured package source or invoke the package manager through an appropriate installer flow. See [Packaging Desktop Apps](./packaging.md) for each platform's artifact and identity requirements.

## Test the release path

Test with **downloaded release artifacts** on clean machines or VMs for every supported OS, architecture, channel, and installation format:

| Scenario | Expected result |
| --- | --- |
| Up to date, offline, slow network, timeout, or bad metadata | The app remains usable; the status is truthful and retryable. |
| Wrong target, missing asset, truncated file, bad digest, or invalid signature | Nothing is installed; the error identifies the failed phase without exposing credentials. |
| New version while work is unsaved | The user can defer restart and keep working under the old running version. |
| Concurrent checks, repeated clicks, or a release published during check/install | Only the selected, revalidated version is installed once. |
| Read-only destination, Windows file lock, macOS bundle signature failure, or package-manager-owned install | The app follows its platform install path or fails safely without partially replacing files. |
| Interrupted install, failed relaunch, and downgrade/rollback | The tested recovery path restores a launchable version and preserves user data. |

Finally, install version N through each supported distribution format, update to N+1, verify the version **after restart**, and test ordinary uninstall. A working developer build or a successful update check does not establish that the published installer can upgrade a real installation.
