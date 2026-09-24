---
title: Comparison
description: 按平台和功能比较 GPUI Kit、Iced、egui、Qt 6 与 Slint。
order: -14
---

# Comparison

这是一份功能指南，不是性能基准。它比较 GPUI Kit 与 Iced、egui、Qt 6、Slint 当前公开的 API。可选功能、平台后端、独立库或第三方 crate 都可能改变应用的能力。发布前请查看链接中的文档，确认所用版本和目标平台。

| 方面 | GPUI Kit | Iced | egui | Qt 6 | Slint |
| --- | --- | --- | --- | --- | --- |
| UI 模型 | Rust，保留式 [Entity](/zh-CN/docs/entity) 与元素 | Rust，[state/update/view 架构](https://book.iced.rs/architecture.html) | Rust，[立即模式](https://docs.rs/egui/latest/egui/#the-immediate-mode-paradigm) | C++ 与 QML；[Widgets 和 Qt Quick](https://doc.qt.io/qt-6/topics-ui.html) | 声明式 `.slint` UI，可[集成 Rust、C++、JavaScript 或 Python](https://docs.slint.dev/latest/docs/slint/) |
| 桌面平台 | macOS、Windows、Linux | [桌面平台](https://github.com/iced-rs/iced) | 通过 [eframe 构建原生应用](https://github.com/emilk/egui/blob/main/README.md#official-integrations) | [桌面平台](https://doc.qt.io/qt-6/supported-platforms.html) | [桌面与嵌入式](https://docs.slint.dev/latest/docs/slint/) |
| WebAssembly | [浏览器应用](/zh-CN/docs/webassembly) | [Web 示例](https://github.com/iced-rs/iced/blob/master/examples/README.md#tour) | [eframe Web 目标](https://github.com/emilk/egui/blob/main/README.md#official-integrations) | [Qt for WebAssembly](https://doc.qt.io/qt-6/wasm.html)，模块与浏览器功能有限制 | [Rust 编译为 Wasm](https://docs.slint.dev/latest/docs/slint/guide/platforms/web/)；canvas 渲染，不支持浏览器屏幕阅读器 |
| 原生移动平台 | [实验性 iOS 集成](/zh-CN/docs/mobile) | 尚无官方移动平台指南；[移动支持 issue](https://github.com/iced-rs/iced/issues/302) 仍未关闭 | [eframe 支持 Android 和 iOS](https://github.com/emilk/egui/issues/2066)；需测试平台集成 | [Android 与 iOS](https://doc.qt.io/qt-6/supported-platforms.html) | [Android 与 iOS](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/general/)；iOS 使用 Rust |
| 图表与绘图 | 内置[图表与绘图组件](/zh-CN/component/chart) | 使用 [Canvas](https://docs.rs/iced/latest/iced/widget/struct.Canvas.html) 自定义绘图 | 独立的 [egui_plot](https://docs.rs/egui_plot/latest/egui_plot/) crate | [Qt Graphs](https://doc.qt.io/qt-6/qtgraphs-index.html) 支持 2D 和 3D | 用 [Path](https://docs.slint.dev/latest/docs/slint/reference/elements/path/) 自定义绘图；[标准组件不含图表](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/overview/) |
| 表格与大数据 | [DataTable](/zh-CN/component/data-table) 与 [VirtualList](/zh-CN/component/virtual-list) | [Table 组件](https://docs.rs/iced/latest/iced/widget/table/fn.table.html) | [egui_extras TableBuilder](https://docs.rs/egui_extras/latest/egui_extras/struct.TableBuilder.html)，可只渲染可见行 | [Model/View 表格](https://doc.qt.io/qt-6/modelview.html)；Qt Quick [TableView 复用单元格](https://doc.qt.io/qt-6/qml-qtquick-tableview.html) | [StandardTableView](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/standardtableview/) |
| 动画与动效 | [过渡、弹簧与减弱动态效果](/zh-CN/docs/animation) | [Animation API](https://docs.rs/iced/latest/iced/animation/struct.Animation.html) | [Context 动画辅助方法](https://docs.rs/egui/latest/egui/struct.Context.html#method.animate_bool) | [Qt Quick 动画](https://doc.qt.io/qt-6/qtquick-statesanimations-animations.html)与 [QObject 动画框架](https://doc.qt.io/qt-6/animation-overview.html) | [声明式属性动画](https://docs.slint.dev/latest/docs/slint/reference/language/animations/) |
| Markdown 与富文本 | [TextView 支持 Markdown 和 HTML](/zh-CN/component/text-view) | 通过可选功能使用内置 [Markdown 组件](https://docs.rs/iced/latest/iced/widget/markdown/) | 社区 [egui_commonmark](https://docs.rs/egui_commonmark/latest/egui_commonmark/) 等 crate | [QTextDocument 支持 Markdown 和 HTML](https://doc.qt.io/qt-6/qtextdocument.html#setMarkdown) | 组合 [Text 与自定义元素](https://docs.slint.dev/latest/docs/slint/reference/elements/text/)；无标准 Markdown 组件 |
| 无障碍 | [GPUI Kit 组件](/zh-CN/docs/accessibility)提供语义角色和标签；需逐一验证目标平台 | [原生无障碍集成仍是未完成工作](https://github.com/iced-rs/iced/issues/552) | [受支持的原生平台使用 AccessKit](https://github.com/emilk/egui/blob/main/docs/accessibility.md)；Web 屏幕阅读器仍属实验性 | [平台无障碍 API 和可访问控件](https://doc.qt.io/qt-6/accessible.html)；[Wasm 提供基础支持](https://doc.qt.io/qt-6/wasm.html#accessibility-and-screen-readers) | [无障碍属性](https://docs.slint.dev/latest/docs/slint/reference/common/#accessibility-properties)；[Wasm canvas 输出不可用](https://docs.slint.dev/latest/docs/slint/guide/platforms/web/) |
| 许可证 | Apache-2.0 | [MIT](https://github.com/iced-rs/iced/blob/master/LICENSE) | [MIT 或 Apache-2.0](https://github.com/emilk/egui/blob/main/LICENSE-MIT) | [按模块采用商业、LGPLv3 或 GPLv3](https://doc.qt.io/qt-6/licensing.html)；Qt Graphs 为商业/GPLv3 | [GPLv3 或 Slint 商业/免版税条款](https://slint.dev/pricing) |

### 如何阅读此表

- **Web 支持指基于 canvas 的 Wasm 应用，不等同于 HTML 应用。** 浏览器输入、无障碍、包体积和模块可用性存在差异。例如，[Slint 明确不推荐将其 Wasm 后端用于通用 Web 应用](https://docs.slint.dev/latest/docs/slint/guide/platforms/web/)；[Qt 列出了 WebAssembly 模块和功能限制](https://doc.qt.io/qt-6/wasm.html)。
- **移动支持的成熟度不同。** GPUI Kit 的 iOS 路径仍属实验性；[Slint 提供两个移动平台的文档](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/general/)，[Qt 将 Android 和 iOS 列为支持平台](https://doc.qt.io/qt-6/supported-platforms.html)，eframe 虽能构建 Android/iOS 应用，仍应针对实际应用测试平台集成。
- **Qt 拥有此表中覆盖范围最广、较成熟的工具集。** [Model/View 框架](https://doc.qt.io/qt-6/modelview.html)、[Qt Graphs](https://doc.qt.io/qt-6/qtgraphs-index.html)、[动画 API](https://doc.qt.io/qt-6/qtquick-statesanimations-animations.html) 和[无障碍集成](https://doc.qt.io/qt-6/accessible.html)都是它的强项。许可条款取决于所用模块；[Qt Graphs 采用 GPLv3 或商业许可](https://doc.qt.io/qt-6/licensing.html)。

旧版的二进制大小和主观风格评分已删除：构建功能、字体、渲染器、链接设置与打包方式都会影响结果。请测量准备发布到目标平台的实际应用。
