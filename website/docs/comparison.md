---
title: Comparison
description: How GPUI Kit compares with Iced, egui and Qt 6.
order: 10
---

# Comparison

How GPUI Kit compares with other desktop UI frameworks. The table is
maintained by hand; please open an issue or a pull request if you spot a
mistake or something outdated.

| Features              | GPUI Kit                       | [Iced]             | [egui]                | [Qt 6]                                            |
| --------------------- | ------------------------------ | ------------------ | --------------------- | ------------------------------------------------- |
| Language              | Rust                           | Rust               | Rust                  | C++/QML                                           |
| Core Render           | GPUI                           | wgpu               | wgpu                  | QT                                                |
| License               | Apache 2.0                     | MIT                | MIT/Apache 2.0        | [Commercial/LGPL](https://www.qt.io/qt-licensing) |
| Min Binary Size [^1]  | 12MB                           | 11MB               | 5M                    | 20MB [^2]                                         |
| Cross-Platform        | Yes                            | Yes                | Yes                   | Yes                                               |
| Documentation         | Simple                         | Simple             | Simple                | Good                                              |
| Web                   | Yes (WASM)                     | Yes                | Yes                   | Yes                                               |
| UI Style              | Modern                         | Basic              | Basic                 | Basic                                             |
| CJK Support           | Yes                            | Yes                | Bad                   | Yes                                               |
| Chart                 | Yes                            | No                 | No                    | Yes                                               |
| Table (Large dataset) | Yes<br>(Virtual Rows, Columns) | No                 | Yes<br>(Virtual Rows) | Yes<br>(Virtual Rows, Columns)                    |
| Table Column Resize   | Yes                            | No                 | Yes                   | Yes                                               |
| Text base             | Rope                           | [COSMIC Text] [^3] | trait TextBuffer [^4] | [QTextDocument]                                   |
| CodeEditor            | Simple                         | Simple             | Simple                | Basic API                                         |
| Dock Layout           | Yes                            | Yes                | Yes                   | Yes                                               |
| Syntax Highlight      | [Tree Sitter]                  | [Syntect]          | [Syntect]             | [QSyntaxHighlighter]                              |
| Markdown Rendering    | Yes                            | Yes                | Basic                 | No                                                |
| Markdown mix HTML     | Yes                            | No                 | No                    | No                                                |
| HTML Rendering        | Basic                          | No                 | No                    | Basic                                             |
| Text Selection        | TextView                       | No                 | Any Label             | Yes                                               |
| Custom Theme          | Yes                            | Yes                | Yes                   | Yes                                               |
| Built Themes          | Yes                            | No                 | No                    | No                                                |
| I18n                  | Yes                            | Yes                | Yes                   | Yes                                               |

[Iced]: https://github.com/iced-rs/iced
[egui]: https://github.com/emilk/egui
[QT 6]: https://www.qt.io/product/qt6
[Tree Sitter]: https://tree-sitter.github.io/tree-sitter/
[Syntect]: https://github.com/trishume/syntect
[QSyntaxHighlighter]: https://doc.qt.io/qt-6/qsyntaxhighlighter.html
[QTextDocument]: https://doc.qt.io/qt-6/qtextdocument.html
[COSMIC Text]: https://github.com/pop-os/cosmic-text

[^1]: Release builds by use simple hello world example.

[^2]: [Reducing Binary Size of Qt Applications](https://www.qt.io/blog/reducing-binary-size-of-qt-applications-part-3-more-platforms)

[^3]: Iced Editor: <https://github.com/iced-rs/iced/blob/db5a1f6353b9f8520c4f9633d1cdc90242c2afe1/graphics/src/text/editor.rs#L65-L68>

[^4]: egui TextBuffer: <https://github.com/emilk/egui/blob/0a81372cfd3a4deda640acdecbbaf24bf78bb6a2/crates/egui/src/widgets/text_edit/text_buffer.rs#L20>
