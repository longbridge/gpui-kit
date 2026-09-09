---
title: 原语
description: gpui-base 面向用户的完整原语目录。
order: 4
---

# 原语

GPUI Base 原语只提供行为，不规定视觉表现。每个页面都会说明公开导入路径和最小可用组合。页面上方的在线示例由 `crates/base/examples` 构建，也可以作为原生 GPUI 应用运行。

## 原语目录

- [Accordion](./accordion.md) — 由可独立设置样式的标题、触发器和面板组成的折叠组。
- [Alert Dialog](./alert-dialog.md) — 用于需要明确确认之操作的模态对话框。
- [Avatar](./avatar.md) — 带可组合后备内容的人物或实体图像。
- [Button](./button.md) — 无样式、可访问且支持键盘激活的按钮。
- [Calendar](./calendar.md) — 支持选择匹配器和自定义日期渲染的状态驱动日历。
- [Checkbox](./checkbox.md) — 指示器可单独设置样式的受控三态复选框。
- [Collapsible](./collapsible.md) — 不限定触发器样式的可折叠内容区域。
- [Color Picker](./color-picker.md) — 构建自定义颜色选择器所需的状态和交互基础。
- [Combobox](./combobox.md) — 结合文本输入、键盘导航建议和选择行为的组合框。
- [Date Picker](./date-picker.md) — 将日历与弹出层组合起来、可感知焦点的日期输入。
- [Dialog](./dialog.md) — 带焦点管理、遮罩、标题和关闭部件的可组合模态层。
- [Hover Card](./hover-card.md) — 与指针或键盘触发器关联的延迟浮动卡片。
- [Input](./input.md) — 支持选择、掩码、验证和数值步进的单行输入。
- [Textarea](./textarea.md) — 支持固定行数、换行和自动增高的多行文本框。
- [Editor](./editor.md) — 支持高亮、行号槽、折叠、装饰和 LSP 扩展的代码编辑器基础。
- [Link](./link.md) — 样式由应用定义的可访问链接控件。
- [Nav Stack](./nav-stack.md) — 支持 push、pop、forward 与 replace 的视图导航栈，过渡生命周期可动画。
- [Number Input](./number-input.md) — 带递增、递减和步进行为的数字输入。
- [OTP Input](./otp-input.md) — 由共享文本状态驱动的多单元格验证码输入。
- [Pagination](./pagination.md) — 显式管理当前页与总页数的受控分页器。
- [Popover](./popover.md) — 支持受控或内部开关状态的锚定浮层。
- [Popup](./popup.md) — 底层触发器与锚定浮动内容宿主。
- [Progress](./progress.md) — 用轨道和指示器报告任务完成度。
- [Radio](./radio.md) — 具有选中与禁用语义的受控单选项。
- [Radio Group](./radio-group.md) — 为单项选择提供分组与键盘导航。
- [Resizable](./resizable.md) — 用于可调整分栏布局的面板组和拖拽手柄。
- [Scrollbar](./scrollbar.md) — 连接 GPUI 滚动句柄或统一列表句柄的滚动条。
- [Select](./select.md) — 由锚定且支持键盘导航的弹层驱动的选择控件。
- [Sheet](./sheet.md) — 从边缘进入并管理关闭和焦点的模态层。
- [Slider](./slider.md) — 轨道、指示区和滑块可独立设置样式的范围输入。
- [Switch](./switch.md) — 轨道与滑块可分别设置样式的受控开关。
- [Table](./table.md) — 用于组合表头、表体、行和单元格的语义化表格原语。
- [Tabs](./tabs.md) — 带受控选择的标签列表和可访问标签控件。
- [Toast](./toast.md) — 受管理、带动画的临时状态消息栈。
- [Toggle](./toggle.md) — 用于格式等持久选择的受控双态按钮。
- [Toggle Group](./toggle-group.md) — 将多个 Toggle 协调为单选或多选组。
- [Tooltip](./tooltip.md) — 与触发元素关联、延迟显示且可定位的说明。
- [Tree](./tree.md) — 显式管理展开与选择状态的虚拟化层级列表。
