---
title: FAQ
description: How GPUI Kit relates to Zed's GPUI, the gpui-pre crates and GPUI Fast.
order: -13.5
---

# FAQ

## Is GPUI Kit a fork of Zed's GPUI?

No. GPUI Kit is a framework layered on top of GPUI, the UI framework from Zed
Industries. It adds `gpui-base`, `gpui-component` and `gpui-shell`; it does not
modify GPUI itself. GPUI Kit is developed by Longbridge and is not an official
Zed project.

## What are the `gpui-pre-*` crates?

They are unmodified snapshots of Zed's GPUI crates, published to crates.io under
the `gpui-pre` name because Zed does not publish GPUI there regularly. The
[Release GPUI](https://github.com/longbridge/gpui-kit/blob/main/.github/workflows/release-gpui.yml)
workflow publishes a fresh snapshot from Zed's repository every week, with Zed's
license and notices intact. GPUI Kit pins one snapshot as its default backend,
so `gpui-pre` is a package name for upstream GPUI, not a separate framework.

## What is GPUI Fast, and how does it relate to GPUI Kit?

[GPUI Fast](https://github.com/longbridge/gpui-fast) is a separate,
performance-focused fork of GPUI that regularly syncs from upstream. It is an
alternative backend, not part of GPUI Kit's component layers. GPUI Kit uses
upstream GPUI by default and switches to GPUI Fast only when the `gpui-fast`
feature is enabled (see [Installation](./installation.md)); application code
stays the same.

## Which one should I use?

- Start with the default (`gpui-pre`, upstream GPUI) for the closest match to
  Zed's GPUI.
- Enable `gpui-fast` when rendering performance matters, for example large lists
  or tables, after measuring it in your application.
- Depend on GPUI directly, without GPUI Kit, if you only need the rendering
  foundation and want to build every component yourself.

## Will GPUI Kit be merged back into Zed's GPUI?

GPUI Kit is meant to be its own project built on GPUI, not a staging area for
Zed. GPUI Fast follows upstream GPUI, but there is no commitment to upstream
GPUI Kit components or GPUI Fast changes into Zed's GPUI.
