---
title: 框架对比
description: GPUI Kit 与 Iced、egui、Qt 6 的对比。
order: 10
---

# 框架对比

GPUI Kit 与其他桌面 UI 框架的对比。表格由人工维护，如发现任何错误或过时信息，请提交 issue 或 PR。

| 特性                | GPUI Kit             | [Iced]             | [egui]                | [Qt 6]                                            |
| ------------------- | -------------------- | ------------------ | --------------------- | ------------------------------------------------- |
| 语言                | Rust                 | Rust               | Rust                  | C++/QML                                           |
| 核心                | GPUI                 | wgpu               | wgpu                  | QT                                                |
| 许可证              | Apache 2.0           | MIT                | MIT/Apache 2.0        | [Commercial/LGPL](https://www.qt.io/qt-licensing) |
| 最小二进制大小 [^1] | 12MB                 | 11MB               | 5M                    | 20MB [^2]                                         |
| 跨平台              | 是                   | 是                 | 是                    | 是                                                |
| 文档                | 一般                 | 一般               | 一般                  | 良好                                              |
| Web 支持            | 是（WASM）           | 是                 | 是                    | 是                                                |
| UI 风格             | 现代                 | 基础               | 基础                  | 基础                                              |
| CJK 支持            | 是                   | 是                 | 差                    | 是                                                |
| Chart               | 是                   | 否                 | 否                    | 是                                                |
| Table（大数据集）   | 是<br>（虚拟行、列） | 否                 | 是<br>（虚拟行）      | 是<br>（虚拟行、列）                              |
| Table 列宽调整      | 是                   | 否                 | 是                    | 是                                                |
| 文本基础            | Rope                 | [COSMIC Text] [^3] | trait TextBuffer [^4] | [QTextDocument]                                   |
| Code Editor         | 简单                 | 简单               | 简单                  | 基础 API                                          |
| Dock 布局           | 是                   | 是                 | 是                    | 是                                                |
| 语法高亮            | [Tree Sitter]        | [Syntect]          | [Syntect]             | [QSyntaxHighlighter]                              |
| Markdown 渲染       | 是                   | 是                 | 基础                  | 否                                                |
| Markdown 混合 HTML  | 是                   | 否                 | 否                    | 否                                                |
| HTML 渲染           | 基础                 | 否                 | 否                    | 基础                                              |
| 文本选择            | TextView             | 否                 | 任意 Label            | 是                                                |
| 自定义主题          | 是                   | 是                 | 是                    | 是                                                |
| 内置主题            | 是                   | 否                 | 否                    | 否                                                |
| 国际化              | 是                   | 是                 | 是                    | 是                                                |

[Iced]: https://github.com/iced-rs/iced
[egui]: https://github.com/emilk/egui
[QT 6]: https://www.qt.io/product/qt6
[Tree Sitter]: https://tree-sitter.github.io/tree-sitter/
[Syntect]: https://github.com/trishume/syntect
[QSyntaxHighlighter]: https://doc.qt.io/qt-6/qsyntaxhighlighter.html
[QTextDocument]: https://doc.qt.io/qt-6/qtextdocument.html
[COSMIC Text]: https://github.com/pop-os/cosmic-text

[^1]: 使用简单 Hello World 示例的 Release 构建。

[^2]: [减小 Qt 应用程序的二进制大小](https://www.qt.io/blog/reducing-binary-size-of-qt-applications-part-3-more-platforms)

[^3]: Iced Editor: <https://github.com/iced-rs/iced/blob/db5a1f6353b9f8520c4f9633d1cdc90242c2afe1/graphics/src/text/editor.rs#L65-L68>

[^4]: egui TextBuffer: <https://github.com/emilk/egui/blob/0a81372cfd3a4deda640acdecbbaf24bf78bb6a2/crates/egui/src/widgets/text_edit/text_buffer.rs#L20>
