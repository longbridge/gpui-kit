---
title: FAQ
description: GPUI Kit 与 Zed GPUI、gpui-pre 包以及 GPUI Fast 的关系。
order: -13.5
---

# FAQ

## GPUI Kit 是 Zed GPUI 的 fork 吗？

不是。GPUI Kit 是构建在 GPUI（Zed Industries 的 UI 框架）之上的框架，提供
`gpui-base`、`gpui-component` 和 `gpui-shell`，不修改 GPUI 本身。GPUI Kit 由
Longbridge 开发，不是 Zed 官方项目。

## `gpui-pre-*` 包是什么？

它们是 Zed GPUI crate 的原样快照。由于 Zed 不定期向 crates.io 发布 GPUI，这些快照以
`gpui-pre` 名称发布。[Release GPUI](https://github.com/longbridge/gpui-kit/blob/main/.github/workflows/release-gpui.yml)
工作流每周从 Zed 仓库发布一次新快照，并保留 Zed 的许可证与声明。GPUI Kit 固定使用其中一个快照作为默认后端，
所以 `gpui-pre` 只是上游 GPUI 的包名，不是另一个框架。

## GPUI Fast 是什么？和 GPUI Kit 有什么关系？

[GPUI Fast](https://github.com/longbridge/gpui-fast) 是一个独立的、专注性能的 GPUI fork，
定期同步上游。它是可替换的后端，不属于 GPUI Kit 的组件层。GPUI Kit 默认使用上游 GPUI，只有开启
`gpui-fast` feature 时才切换到 GPUI Fast（见[安装](./installation.md)），应用代码无需改动。

## 应该选哪个？

- 默认（`gpui-pre`，上游 GPUI）最贴近 Zed 的 GPUI，建议从它开始。
- 渲染性能重要时（例如大列表、表格），在自己的应用里实测后再开启 `gpui-fast`。
- 如果只需要渲染基础、想自己实现所有组件，可以直接依赖 GPUI，不用 GPUI Kit。

## GPUI Kit 会合并回 Zed 的 GPUI 吗？

GPUI Kit 是构建在 GPUI 之上的独立项目，不是 Zed 的预备分支。GPUI Fast 跟随上游 GPUI，
但不承诺把 GPUI Kit 组件或 GPUI Fast 的改动合并进 Zed 的 GPUI。
