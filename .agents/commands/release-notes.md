# Release Notes

Write release notes using the input supplied by the user: either a tag such
as `v0.6.2`, or the auto-generated "## What's Changed" list copied from the
GitHub release draft. When no input is supplied, use the `version` in the
workspace `Cargo.toml` with a `v` prefix.

## Gather

1. Read the previous release with `gh release view <previous-tag> --json body -q .body`
   to match its structure, tone, and section names. The previous tag is
   `git describe --tags --abbrev=0 <tag>^` (or the latest published release
   when `<tag>` does not exist yet).
2. If no "What's Changed" list was pasted, generate it:

   ```bash
   gh api repos/longbridge/gpui-kit/releases/generate-notes \
     -f tag_name=<tag> -f previous_tag_name=<previous-tag> -q .body
   ```

   Keep this list, its "New Contributors" section, and the "Full Changelog"
   line verbatim; they close the notes.
3. Read the body of every PR that adds a component, a public API, a platform,
   or a behavior change, with `gh pr view <n> --json title,body`. Take API
   names, method names, and `## Breaking Changes` diff blocks from the PR
   text; never invent a name from the title alone. Fix-only PRs can be
   summarized from their titles.

## Write

Produce one Markdown document in this shape, in English:

```markdown
# GPUI Kit <tag>

<Two or three sentences: the headline features first, then what else the
release improves.>

## <Theme>

- **<Feature>**: what a consumer can now do, with the API in backticks.
  ([#123](https://github.com/longbridge/gpui-kit/pull/123))
- Fixed ... ([#124](...), [#125](...))

...

## Breaking Changes            <!-- only when a PR has one -->

<One sentence per change naming the PR, then its diff block.>

Thank you to everyone who contributed code, documentation, testing, and feedback!

## What's Changed
<verbatim list>

## New Contributors
<verbatim list>

**Full Changelog**: <verbatim line>
```

Rules:

- Group by product area, ordered by significance: headline features (new
  platforms, new components), then editor and text input, Markdown and text
  rendering, motion, dock, other components (one bold component name per
  bullet), shell, runtime and platform, documentation and website. Drop a
  section that has nothing to say; merge small ones.
- Every bullet cites its PR numbers as `[#n](https://github.com/longbridge/gpui-kit/pull/n)`
  at the end, in parentheses. Combine PRs that land one change into one bullet.
- Leave out chores, CI, lockfile refreshes, test-only changes, and a PR that
  reverts another PR in the same release (leave both out).
- Bold the feature, not the sentence; write what it does for an application,
  not what the diff touched.
- Do not create or edit the GitHub release unless asked. Save the document to
  the scratchpad and show it, so it can be pasted into the release draft.
