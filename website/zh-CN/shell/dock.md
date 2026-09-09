---
title: Dock 与面板
description: 完全由脚本绘制的可停靠布局——重启后仍在原处的面板、自己画的 chrome，以及用命令代替回调。
order: 13
---

# Dock 与面板

只能铺满整个窗口的 View 算不上一个应用。**dock area** 把脚本 View 变成*面板*：可拖动、可停靠、可放大，重启之后仍然停在用户上次放的位置。

```js
import { View, div } from "gpui-kit";
import { DockArea, dock_area, v_flex } from "gpui-base";

class Notes extends View {
  render() {
    return div().p(16).child("Notes");
  }
}

export default class Workspace extends View {
  init(_props, cx) {
    DockArea.register_panel("notes", Notes);
    this.dock = DockArea.new("workspace");
    this.dock.add_panel(cx.new(Notes), {
      name: "notes",
      placement: "left",
      size: 240,
    });
  }

  render() {
    return dock_area(this.dock).size_full();
  }
}
```

这样就已经能停靠、拖动、调整大小、放大与持久化了。它不会画出标签栏，因为 **base 完全不画 chrome**——见[绘制 chrome](#绘制-chrome)。

## base 给了什么，没给什么

`gpui_kit::base::dock` 已经把停靠系统里难做的那一半做好了：一棵**纯数据**的布局树、一个能按持久化文件里的名字重建面板的 `PanelRegistry`，以及跟着每块面板走的一份 payload。容器用稳定的 node id 寻址，面板用稳定的 panel id 寻址，因此一次拖动改的是一个值，而不是拆掉再重建一堆 View。

它没有的是外观。引擎什么都不画——没有标签栏、没有 dock 外框、没有拖拽条、没有落点提示——这些全都作为「返回元素的回调」交还给你。这不是需要绕开的限制，而正是这套东西能被脚本用起来的原因：外观不是覆盖在某个默认外观之上的一层，因为根本没有默认外观。

## area 是 retained 的

`DockArea.new(id)` 创建的是跨帧存活的状态，和 `InputState` 一样；而且理由是其他 handle 都没有的一条：**布局是用户改的**。拖动、调整大小、关掉一个标签页、折叠一侧 dock，全都发生在脚本没有渲染的时候。一个从描述里重建出来的 dock，会把这些统统还原成上一次渲染所描述的样子。

所以它在 `init` 里创建一次，`render` 只负责*画*：

```js
init() { this.dock = DockArea.new("workspace", { version: 1 }); }
render() { return dock_area(this.dock).size_full(); }
```

`DockArea.new` 需要一次活的 Host 调用，所以它属于 `init` 或事件处理器——绝不能放在 `render` 里。每个会改动布局的方法同样如此；从 `render` 里调用会在写下它的那一行被拒绝，而不是产出一帧「画的是一套布局、描述的是另一套」的画面。

## 编辑在调用返回时生效

面板的主体来自 `cx.new(Class)`——你把它交出去的那一刻，它自己都还在构造中；`load` 还会再构造更多面板。这些都不可能在脚本正在运行时发生。所以**每一次编辑都会排队，等到发起它的那次调用返回之后再按调用顺序应用**。

实际影响只有一句话：`panels()` 和 `dump()` 读到的是本轮编辑*之前*的布局。

```js
init(_props, cx) {
  this.dock = DockArea.new("workspace");
  this.dock.add_panel(cx.new(Notes), { name: "notes" });
  this.dock.panels();          // 还是空的——这次 add 尚未应用
  this.dock.on("layout_changed", (cx) => {
    this.dock.panels();        // 三块面板、一个 dock 尺寸、一次标签页移动
    cx.notify();
  });
}
```

`layout_changed` 会在每次编辑时触发，包括 tile 拖动的每一步，所以要用定时器落盘，而不是在事件里直接写。

## 面板

面板就是恰好被 dock 拿在手里的一个 View。`add_panel` 接收这个 View 并说明它去哪里：

```js
this.dock.add_panel(cx.new(Editor, { file }), {
  name: "editor", // 必填——保存布局时用它归档
  placement: "center", // "center" | "left" | "right" | "bottom"
  size: 240, // 当这块面板是该 dock 里的第一块时，用它作为初始尺寸
  closable: true,
  zoomable: true,
  visible: true,
});
```

`name` 必填，因为它不是装饰：保存布局写的是它，`register_panel` 也靠它找回类。命名空间由运行时加好——`shell:<application>/<name>`——所以两个都把面板叫 `inbox` 的应用永远不会撞车，脚本面板也不可能盖掉 Host 面板。

`panels()` 报告现在有什么，以及在哪里：

```js
this.dock.panels();
// [{ id, name, placement, node, index, active, visible, closable, zoomable }, …]
```

`id` 就是 `remove_panel(id)` 要的那个，也是关闭按钮交给 `close_panel` 的那个。

## 熬过一次重启

两半，缺一不可。

**注册类**，让保存的布局能重建它：

```js
DockArea.register_panel("editor", Editor);
```

**保存并恢复布局**，它就是纯数据：

```js
init(_props, cx) {
  DockArea.register_panel("editor", Editor);
  this.dock = DockArea.new("workspace", { version: 1 });

  const saved = localStorage.getItem("layout");
  if (saved) this.dock.load(JSON.parse(saved));
  else this.dock.add_panel(cx.new(Editor), { name: "editor" });

  this.dock.on("layout_changed", () =>
    localStorage.setItem("layout", JSON.stringify(this.dock.dump())));
}
```

面板自己的状态会跟着位置一起走。 View 类上有两个可选方法负责这件事：

| 方法                | 何时调用        | 说明                                                                       |
| ------------------- | --------------- | -------------------------------------------------------------------------- |
| `serialize()`       | 保存布局时      | 运行时**没有 Host 调用**：返回纯数据，别碰别的——不要碰 entity，不要碰 `cx` |
| `deserialize(data)` | View 刚重建之后 | 有一次真正的 Host 调用，因此可以碰 entity                                  |

`version` 由你在保存格式变化时递增；base 会拒绝加载在别的 version 下写出的布局，于是旧文件是被忽略，而不是被一知半解地读进来。

### 卸载了的应用仍然保留位置

这是最值得围绕着设计的一条性质。

如果某块面板的名字下没有注册任何东西——应用被卸载了，或者类被改名了——这块面板**不会被丢掉**。一个什么都不画的占位面板会顶上，并原样报告它拿到的状态，于是下一次保存会把这块面板的名字、payload 和位置原封不动写回去。卸载一个应用，把窗口用上一周，再装回来：它的面板会回到原来的位置，带着原来的状态。

再往里一步也是同样的承诺：一块*已经*注册、但类在构造时抛异常的面板，会以同样的方式被带下去——一个写坏的脚本，代价是这一次会话里看不到那块面板的内容，而不是丢掉它在布局里的位置。

## 绘制 chrome

六个 handler，全都可选，挂在 `dock_area(...)` 元素上：

| Handler                          | 画什么                              |
| -------------------------------- | ----------------------------------- |
| `tab_bar(group => …)`            | 一个 group 当前显示面板上方的标签栏 |
| `empty_group(group => …)`        | 没有可显示面板的 group 显示什么     |
| `drop_indicator(drop => …)`      | 被拖动的面板会落在哪里              |
| `dock(dock => …)`                | 一侧 dock 包住内容的外框            |
| `tile_drag_bar(tile => …)`       | 拖动 tile 用的那条拖拽条            |
| `tile_resize_handles(tile => …)` | tile 的缩放把手                     |

每一个都会先在 GPUI 的 layout pass 内部被调用，拿到的是 base **已经解析好的**状态——从不包含拖拽事件、鼠标位置或命中测试，因为 base 会把这些自己挂到拿回去的元素上。生成的描述按 handler 与解析后的状态缓存；未变化的帧只在 Rust 中重放，不会进入 JavaScript。

```js
dock_area(this.dock)
  .size_full()
  .tab_bar((group, cx) =>
    h_flex()
      .h(30)
      .bg(cx.theme().colors.secondary)
      .children(
        group.tabs
          .filter((tab) => tab.visible)
          .map((tab) =>
            h_flex()
              .id("tab-" + tab.id)
              .px(10)
              .items_center()
              .bg(
                tab.active
                  ? cx.theme().colors.background
                  : cx.theme().colors.secondary,
              )
              .select_tab(group, tab.index)
              .drag_tab(group, tab.index)
              .child(tab.name)
              .child(
                div()
                  .id("x-" + tab.id)
                  .close_panel(group, tab.id)
                  .child("×"),
              ),
          ),
      ),
  );
```

### 命令，不是回调

再看一眼上面那个标签页：它带的是 `select_tab` 和 `drag_tab`，不是 `on_click`。这是这套 API 里唯一一条值得理解、而不是死记的规则。

chrome 描述会被缓存，并且可以比生成它的 handler 调用活得更久。因此，在其中注册的脚本回调没有可靠的事件生命周期，而且每次原生状态变化都可能再创建一个。这样的注册会在写下它的那一行被拒绝；chrome 改用原生命令。

**命令**完全不携带脚本值。它只是指名 area 里的某个容器、以及要请它做什么，剩下的由 base 完成：

| 命令                           | 触发 | 作用                                          |
| ------------------------------ | ---- | --------------------------------------------- |
| `select_tab(group, index)`     | 点击 | 显示那个标签页                                |
| `close_panel(group, panel_id)` | 点击 | 关闭该面板（如果它所在的 group 允许）         |
| `toggle_zoom(group)`           | 点击 | 放大 group，或还原                            |
| `drag_tab(group, index)`       | 拖动 | 让该元素成为这个标签页的拖动源                |
| `drop_tab(group, index?)`      | 放下 | 在此接收被拖来的面板；不给 index 就追加到末尾 |
| `toggle_dock(dock)`            | 点击 | 展开或收起这侧 dock                           |
| `resize_dock(dock)`            | 拖动 | 拖动 dock 的边                                |
| `move_tile(tile)`              | 拖动 | 在画布上移动这个 tile                         |
| `resize_tile(tile, side)`      | 拖动 | 拖动某条边或某个角                            |
| `raise_tile(tile)`             | 按下 | 把这个 tile 提到最上层                        |
| `toggle_tile_zoom(tile)`       | 点击 | 让 tile 放大占满所在 dock                     |
| `close_tile(tile)`             | 点击 | 关闭这个 tile                                 |

每一个的第一个参数都是它所在 handler 拿到的那个对象。它们只能挂在 `div`、`h_flex` 或 `v_flex` 上：`Button` 自己构造内部结构，没有地方安放这些命令。

拖动产生的一切，base 都会在下一帧看到它之前钳制、吸附并取整，所以一个缩放把手只是一块命中区加一点颜色，仅此而已。

### dock handler 自己安放内容

`dock` 是唯一一个除了状态之外还会拿到一个元素的 handler，而它返回什么就*替换*掉这侧 dock 的内容。把 `dock_content()` 放在面板该出现的位置：

```js
.dock((dock, cx) =>
  v_flex()
    .size_full()
    .relative()
    .child(
      h_flex()
        .h(30)
        .justify_between()
        .child(dock.placement.toUpperCase())
        .child(div().id("collapse").toggle_dock(dock).child(dock.open ? "–" : "+")),
    )
    .child(dock_content().flex_1())
    .child(div().absolute().right(0).w(4).h_full().cursor_col_resize().resize_dock(dock)),
)
```

忘了写 `dock_content()` 的 handler 仍然会显示它的面板——面板会画在它返回的内容之后，并带一条警告——而不是悄悄丢掉。

## Tiles

一个区域也可以是自由浮动的画布，而不是标签组。传入 `bounds`，面板就成为一个 tile：

```js
this.dock.add_panel(cx.new(Chart), {
  name: "chart",
  placement: "center",
  bounds: { x: 40, y: 40, width: 320, height: 240 },
});
```

tile 需要自己的那两个 handler，因为 base 在那里同样什么都不画：`tile_drag_bar`（高度固定为 base 的拖拽条高度，吸附算法以此为前提）与 `tile_resize_handles`。两者拿到的 `tile` 里，bounds 都是**已经解析好的**。

## 完整接口

```js
area.add_panel(view, options);
area.remove_panel(id);
area.panels();
area.dump();
area.load(state);
area.has_dock(placement);
area.is_dock_open(placement);
area.toggle_dock(placement);
area.remove_dock(placement);
area.dock_size(placement);
area.set_dock_size(placement, size);
area.set_dock_collapsible(placement, collapsible);
area.is_locked();
area.set_locked(locked);
area.is_zoomed();
area.zoom_out();
area.on("layout_changed", handler);
area.release();
```

被锁定的 area 不能重新排列，也不能接受放入操作；dock 和 tile 仍可调整大小。因此「锁定布局」固定的是面板所在位置，而不是面板的可用尺寸。

## 一个完整的例子

```bash
cargo run -p gpui-shell -- examples/js_dock
```

`examples/js_dock/` 是一个工作区：左侧 dock 里是文件列表，中间是文档，标签栏与 dock 外框画在 `ui.js` 里，布局用定时器写进 `localStorage`。它是用上本页每一部分的最短的完整程序。

## 从 Rust 使用

`gpui_kit::shell::dock` 是公开的，因此 Host 不写脚本也能接到同一处接缝。`ScriptPanel` 把 `ScriptView` 包成 `gpui_kit::base::dock::Panel`；`register_panel(application, panel, script, cx)` 教会注册表用一个 `PanelScript` 重建它；`ScriptDockSkin` 把 base 的三个 renderer trait 统一转发给一个 `DockChrome`。`tab_group_data`、`dock_data`、`tile_data` 与 `drop_indicator_data` 是引擎交给脚本代码的那几个 JSON 转换，Host 自己写绑定时同样用得上。
