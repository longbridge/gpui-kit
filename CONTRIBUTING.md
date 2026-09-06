# Contributing Guide

Contributions are welcome, if you find some bugs or have some ideas, please open an issue or submit a pull request.

Please ensure that you are using clean code, following the coding style and code organization in existing code, and make sure all the tests pass.

Please submit **one PR that does one thing**, this is important, and helps us to review your code more easily and push to merge fast.

## AI-Assisted Contributions

GPUI Kit fully embraces AI-assisted development. We welcome contributions
written with AI, including pull requests where 100% of the code was generated
by AI.

What matters is not who wrote the code, but whether the change is thoughtful,
focused, and well validated. The contributor is always responsible for the
final result.

Before opening a pull request, please make sure that:

- The problem is real and the change is necessary.
- The API and implementation follow the existing patterns of GPUI and GPUI Kit.
- UI changes follow GPUI Kit's existing visual and interaction conventions.
- The code has been reviewed by you and properly tested. Run the affected
  stories or examples; screenshots or recordings are encouraged for UI changes.
- A pull request focuses on one thing and keeps the diff as small as practical.
  When using AI, ask it to avoid unrelated refactors, cleanup, or formatting.
  **Less is better.**

Well-prepared pull requests are easier for us to review and may be merged very
quickly. If a contribution is already in good shape, maintainers may directly
help polish small details and move it forward.

Issues are always welcome, but when practical, we encourage you to try a fix
and open a pull request. Concrete code often communicates a problem more
efficiently than a long discussion. A pull request is still subject to review
and is not a guarantee that the proposed change will be accepted.

## Code Style

Before you start to write code, please read the existing code to follow the same coding style and code organization.

- Inspired by existing code or refer to macOS/Windows controls API design to name your functions, properties, structs etc.

## Development and Testing

### System dependencies

The `script` folder contains some useful scripts to help you set up the development environment.

To install the system dependencies, run the following script:

```bash
./script/bootstrap
```

For Windows, you can run the following command in PowerShell:

```powershell
.\script\install-window.ps1
```

### Accessibility-driven UI testing

Use accessibility-driven interaction as the default manual UI testing method
for focus, keyboard, selection, menu, and input behavior. See
[Accessibility-driven UI testing](docs/ACCESSIBILITY-UI-TESTING.md) for the
required Story app launch method, accessibility-tree workflow, and completion
evidence.

### Run story

There are a lot of UI test cases in the `crates/story` folder, if you change the existing features you can run the tests to make sure they are working.

Use `cargo run` to run the complete story examples to display them all in a gallery of GPUI components.

```bash
cargo run
```

### Run single example

Standalone examples live in `examples/`. Run a package directly; see [the examples README](examples/README.md) for the available commands.

```bash
cargo run -p example-editor
```

## UI Guides

GPUI Component is inspired by macOS and Windows controls, combined with shadcn/ui design for a modern experience.

So please refer to the following UI guides when you design or change the UI components:

- [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)
- [Microsoft Fluent Design System](https://learn.microsoft.com/en-us/windows/apps/design/)
- [shadcn/ui](https://ui.shadcn.com/)

### Rules

- Use `default` mouse cursor not `pointer` for buttons, unless it's a link button, we are building desktop apps, not web apps.
- Use `md` size for most cases and as the default.

## Profile the performance

When you change the rendering code, please profile the performance to make sure the FPS is still good.

You can use `MTL_HUD_ENABLED=1` environment variable to enable the Metal HUD to see the FPS and other performance metrics.

```bash
MTL_HUD_ENABLED=1 cargo run
```

> NOTE: Only available on macOS with Metal backend, and the FPS is up **limited your monitor refresh rate**, usually 60 or 120.

### Use Samply to profile the the performance

You can use [Samply](https://github.com/mstange/samply) to profile the performance of the application to get more detailed information.

```bash
samply record cargo run
```

Use `samply record` command to start rust development, and do some operations in the app that you want to profile, then stop the terminal with `ctrl-c`, then samply will open the browser to show the profile results.

## Release crates version

When we are ready to release a new version, please follow the steps below:

### Use the script to bump the version(Recommended)

```bash
./script/bump-version.sh x.y.z
```

### Manually bump the version

1. Run `cargo set-version` to set the new version for all crates.

   ```bash
   cargo set-version x.y.z
   ```

2. Git Commit the changes with message `Bump vx.y.z`.
3. Create a new git tag with the version `vx.y.z` and push `main` branch and the tag to remote.

   ```bash
   git tag vx.y.z
   git push origin vx.y.z
   ```

4. Then GitHub Actions will automatically publish the crates to crates.io and create a new release in GitHub.
