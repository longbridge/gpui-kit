Closes #[issue number]

## Description

Describe in English for the changes made in this pull request and the problem it solves.
Please keep **1 PR to solve 1 problem**, and keep **Small improvements should be small modifications** to make PR easier to review and to merge.

## Screenshot

| Before                       | After                       |
| ---------------------------- | --------------------------- |
| [Put Before Screenshot here] | [Put After Screenshot here] |

## Public API

List every public item this pull request adds, changes or removes, grouped by crate, with its signature and one line on what it is for. Include JavaScript methods and TypeScript declarations. If none, remove this section.

- `gpui_component::input::Example::builder(value: bool) -> Self` — what it does.

## Breaking Changes

Describe any breaking changes introduced by this pull request. If none, remove this section.

- Change 1

```diff
- Old code snippet
+ New code snippet
```

## How to Test

Please describe the tests that you ran to verify your changes. Provide instructions so we can reproduce.

## Checklist

- [ ] I have read the [CONTRIBUTING](../CONTRIBUTING.md) document and followed the guidelines.
- [ ] Reviewed the changes in this PR and confirmed AI generated code (If any) is accurate.
- [ ] Passed `cargo run` for story tests related to the changes.
- [ ] Tested macOS, Windows and Linux platforms performance (if the change is platform-specific)
