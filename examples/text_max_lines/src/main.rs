//! Clamp rendered Markdown to a number of whole lines with
//! `TextView::max_lines`.
//!
//! - Drag the slider to change the line budget: a line of text is never cut in
//!   half, and nothing is shown with less than a line of itself to show.
//! - The photo crossing the edge is cut there, keeping the part that fits.
//! - "Show more" appears only while `TextViewState::is_clamped()` reports that
//!   the previous painted frame clipped something.
//!
//! Run: `cargo run -p text_max_lines`

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    scroll::ScrollableElement as _,
    slider::{Slider, SliderEvent, SliderState},
    text::{TextView, TextViewState},
    *,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

const DEFAULT_MAX_LINES: usize = 5;

const LONG_MARKDOWN: &str = r#"### Quarterly summary

**Revenue** grew by *18%* quarter over quarter, driven by the desktop client
rollout and the new [market data](https://longbridge.com) subscriptions —
legacy plans are ~~discontinued~~ and folded into `pro`.

> The clip must land on a whole line: however you drag the slider, no line of
> glyphs is ever cut in half.

Inline image mix: PNG avatars <img src="https://avatars.githubusercontent.com/u/5518" alt="Jason Lee avatar" width="32" height="32" /> and <img src="https://avatars.githubusercontent.com/u/28998859" alt="GitHub avatar" width="32" height="32" /> stay inside the same text flow, and an SVG badge ![Rust](https://rust-lang.org/static/images/rust-logo-blk.svg) wraps with the text around it.

- Desktop DAU is up **24%**
  - macOS **+31%**, Windows *+19%*
  - Linux ships via `install.sh` now
- The `max_lines` preview lands in this release
- Churn stayed flat at 2.1%

| Segment | QoQ    | Note                 |
| ------- | ------ | -------------------- |
| Desktop | +24%   | new dock layout      |
| Mobile  | +9%    | steady               |
| Web     | -3%    | migrating to desktop |

![Img](https://miro.medium.com/v2/resize:fit:1400/format:webp/1*WgEz5f3n3lD7MfC7NeQGOA.jpeg)

---

```rust
fn main() {
    println!("hidden until expanded");
}
```

Text lines are kept whole; the photo above is cut on the box edge instead, so
the preview never holds blank space it could have filled."#;

const SHORT_MARKDOWN: &str = "A **short** note that fits inside the cap, so it renders at its natural \
     height and no button appears.";

struct MaxLinesExample {
    long: Entity<TextViewState>,
    short: Entity<TextViewState>,
    slider: Entity<SliderState>,
    max_lines: usize,
    expanded: bool,
}

impl MaxLinesExample {
    fn new(cx: &mut Context<Self>) -> Self {
        let long = cx.new(|cx| TextViewState::markdown(LONG_MARKDOWN, cx));
        let short = cx.new(|cx| TextViewState::markdown(SHORT_MARKDOWN, cx));
        // `is_clamped` is written while the view paints; observe the states so
        // the button follows it without the caller measuring the text again.
        cx.observe(&long, |_, _, cx| cx.notify()).detach();
        cx.observe(&short, |_, _, cx| cx.notify()).detach();

        let slider = cx.new(|_| {
            SliderState::new()
                .min(1.)
                .max(60.)
                .step(1.)
                .default_value(DEFAULT_MAX_LINES as f32)
        });
        cx.subscribe(&slider, |this, _, event, cx| {
            if let SliderEvent::Change(value) = event {
                this.max_lines = value.start() as usize;
                cx.notify();
            }
        })
        .detach();

        Self {
            long,
            short,
            slider,
            max_lines: DEFAULT_MAX_LINES,
            expanded: false,
        }
    }

    /// Caption above a preview, then the preview itself on a hairline surface.
    fn section(&self, caption: impl Into<SharedString>, body: Div, cx: &App) -> Div {
        v_flex()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(caption.into()),
            )
            .child(
                body.p_4()
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border),
            )
    }
}

impl Render for MaxLinesExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clamped = self.long.read(cx).is_clamped();
        let expanded = self.expanded;
        let max_lines = self.max_lines;

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                v_flex()
                    .px_6()
                    .pt_6()
                    .pb_4()
                    .gap_1()
                    .child(div().text_lg().font_semibold().child("Clamped previews"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                "Rendered Markdown bounded to a number of whole lines. \
                                 Drag the slider, or resize the window to reflow the text.",
                            ),
                    ),
            )
            .child(
                h_flex()
                    .px_6()
                    .pb_4()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Lines"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .max_w(px(320.))
                            .child(Slider::new(&self.slider)),
                    )
                    // A fixed width keeps the slider still as the value grows.
                    .child(div().w(px(24.)).text_sm().child(max_lines.to_string())),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .px_6()
                    .pb_6()
                    .gap_6()
                    .child(
                        self.section(
                            if expanded {
                                "Expanded"
                            } else {
                                "Clamped to the line budget"
                            },
                            v_flex()
                                .gap_3()
                                .child(
                                    TextView::new(&self.long)
                                        .selectable(true)
                                        .when(!expanded, |this| this.max_lines(max_lines)),
                                )
                                .when(clamped || expanded, |this| {
                                    this.child(
                                        h_flex().child(
                                            Button::new("toggle")
                                                .ghost()
                                                .small()
                                                .icon(if expanded {
                                                    IconName::ChevronUp
                                                } else {
                                                    IconName::ChevronDown
                                                })
                                                .label(if expanded {
                                                    "Show less"
                                                } else {
                                                    "Show more"
                                                })
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.expanded = !this.expanded;
                                                    cx.notify();
                                                })),
                                        ),
                                    )
                                }),
                            cx,
                        ),
                    )
                    .child(
                        self.section(
                            "Shorter than the budget",
                            v_flex().child(
                                TextView::new(&self.short)
                                    .selectable(true)
                                    .max_lines(max_lines),
                            ),
                            cx,
                        ),
                    )
                    .overflow_y_scrollbar(),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_kit::init(cx);

        // The document embeds remote images; without an HTTP client they
        // silently never load.
        let http_client = reqwest_client::ReqwestClient::user_agent("gpui-component/example")
            .expect("Failed to create the HTTP client");
        cx.set_http_client(std::sync::Arc::new(http_client));

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(720.), px(680.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| MaxLinesExample::new(cx));
                // The first level view on the window should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
