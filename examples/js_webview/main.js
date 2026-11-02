// A web page inside a script view.
//
// `WebViewState` keeps the page — its history, scroll position and the
// document itself — across renders; `WebView` places it in the layout. Both
// directions of traffic are shown: when a document finishes loading, the
// script asks it for its title with `evaluate_script`, and the page answers
// with `window.ipc.postMessage`, which arrives at `on_message`.
//
//   cargo run -p gpui-shell -- run examples/js_webview
//
// On Linux the page needs an X11 window; in a Wayland session, start it with
// `WAYLAND_DISPLAY=` so GPUI falls back to XWayland.

import { View, div } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";
/** @import { AsyncContext, Context } from "gpui-kit" */
/** @import { InputState as InputStateType, WebViewState as WebViewStateType } from "gpui-component" */
import {
  Button,
  Icon,
  Input,
  InputState,
  TitleBar,
  WebView,
  WebViewState,
} from "gpui-component";

const HOME = "https://gpui-kit.com";

const REPORT_TITLE = "window.ipc.postMessage(document.title)";

export default class Browser extends View {
  /** @param {unknown} _props @param {AsyncContext} _cx */
  init(_props, _cx) {
    /** @type {WebViewStateType} */
    this.page = WebViewState.new(HOME);
    /** @type {InputStateType} */
    this.address = InputState.new("Enter a URL", HOME);
    this.address.on("submit", (_event, cx) => this.open(cx));
    this.title = "";
  }

  /** @param {Context} cx */
  open(cx) {
    let url = this.address.value().trim();
    if (!/^https?:\/\//.test(url)) url = `https://${url}`;
    try {
      this.page.load_url(url);
    } catch (error) {
      console.error(String(error));
    }
    cx.notify();
  }

  /** @param {string} url @param {Context} cx */
  loaded(url, cx) {
    // The page navigates on its own as well, so the address follows the
    // document rather than what was last typed.
    this.address.set_value(url);
    this.page.evaluate_script(REPORT_TITLE);
    cx.notify();
  }

  /** @param {string} title @param {Context} cx */
  titled(title, cx) {
    this.title = title;
    cx.notify();
  }

  /** @param {Context} cx */
  render(cx) {
    const colors = cx.theme().colors;
    /** @param {string} id @param {string} icon @param {string} tooltip @param {() => void} action */
    const navigation = (id, icon, tooltip, action) =>
      Button.new(id)
        .ghost()
        .size("small")
        .tooltip(tooltip)
        .child(Icon.new(`icons/${icon}.svg`).size("small"))
        .on_click((_event, cx) => {
          action();
          cx.notify();
        });

    return v_flex()
      .size_full()
      .bg(colors.background)
      .child(
        TitleBar.new().child(
          div()
            .text_size(13)
            .font_medium()
            .text_color(colors.foreground)
            .child(this.title || "Browser"),
        ),
      )
      .child(
        h_flex()
          .gap(4)
          .px(8)
          .py(6)
          .border_b(1)
          .border_color(colors.border)
          .child(
            navigation("back", "chevron-left", "Back", () => this.page.back()),
          )
          .child(
            navigation("forward", "chevron-right", "Forward", () =>
              this.page.forward(),
            ),
          )
          .child(
            navigation("reload", "rotate-cw", "Reload", () =>
              this.page.reload(),
            ),
          )
          .child(div().flex_1().child(Input.new(this.address))),
      )
      .child(
        WebView.new(this.page)
          .flex_1()
          .min_h_0()
          .on_load((url, cx) => this.loaded(url, cx))
          .on_message((title, cx) => this.titled(title, cx)),
      );
  }
}
