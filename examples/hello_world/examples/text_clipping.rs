use std::borrow::Cow;

use gpui_kit::component::{
    ActiveTheme, Root, Sizable, Size, Theme,
    button::{Button, Toggle},
    checkbox::Checkbox,
    dialog::{DialogFooter, DialogTitle},
    group_box::GroupBox,
    h_flex,
    radio::Radio,
    tab::Tab,
    tag::Tag,
    v_flex,
};
use gpui_kit::{
    App, AppContext, Bounds, Context, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, WindowBounds, WindowOptions, div, px, size,
};

const SAMPLE: &str = "Agypqj Singapore";

struct TextClipping;

fn clipped_text() -> impl IntoElement {
    div().truncate().child(SAMPLE)
}

impl Render for TextClipping {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("Text clipping — IBM Plex Sans")
            .child("Compare g, y, p, q and j with the unclipped reference. Custom text uses truncate().")
            .child(
                h_flex().gap_4().child(div().w_40()).children(
                    ["XSmall", "Small", "Medium", "Large"]
                        .map(|name| div().w_48().child(name)),
                ),
            )
            .children(
                ["Button", "Toggle", "Tab", "Checkbox label", "Radio label", "Tag"]
                    .into_iter()
                    .map(|name| {
                        h_flex()
                            .id(name)
                            .gap_4()
                            .h_12()
                            .flex_shrink_0()
                            .child(div().w_40().child(name))
                            .children(
                                [Size::XSmall, Size::Small, Size::Medium, Size::Large]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, size)| {
                                        h_flex().id(index).w_48().child(match name {
                                            "Button" => Button::new("sample")
                                                .with_size(size)
                                                .child(clipped_text())
                                                .into_any_element(),
                                            "Toggle" => Toggle::new("sample")
                                                .with_size(size)
                                                .child(clipped_text())
                                                .into_any_element(),
                                            "Tab" => Tab::new()
                                                .with_size(size)
                                                .child(clipped_text())
                                                .into_any_element(),
                                            "Checkbox label" => Checkbox::new("sample")
                                                .with_size(size)
                                                .label(SAMPLE)
                                                .into_any_element(),
                                            "Radio label" => div()
                                                .overflow_hidden()
                                                .child(Radio::new("sample").with_size(size).label(SAMPLE))
                                                .into_any_element(),
                                            _ => Tag::primary()
                                                .with_size(size)
                                                .child(clipped_text())
                                                .into_any_element(),
                                        })
                                    }),
                            )
                    }),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex().w_48().flex_shrink_0().gap_2()
                            .child("GroupBox title")
                            .child(GroupBox::new().title(clipped_text())),
                    )
                    .child(
                        v_flex().w_48().flex_shrink_0().gap_2()
                            .child("DialogTitle")
                            .child(DialogTitle::new().child(clipped_text())),
                    )
                    .child(
                        v_flex().w_48().flex_shrink_0().gap_2()
                            .child("DialogFooter")
                            .child(DialogFooter::new().child(clipped_text())),
                    ),
            )
            .child(format!("Reference (no clipping): {SAMPLE}"))
    }
}

fn open_example(cx: &mut App) -> anyhow::Result<()> {
    gpui_kit::init(cx);
    // Use the bundled font so installed fonts do not hide descender clipping.
    cx.text_system().add_fonts(vec![Cow::Borrowed(
        include_bytes!("../../../crates/story-web/fonts/IBMPlexSans-Regular.ttf").as_slice(),
    )])?;
    Theme::global_mut(cx).font_family = "IBM Plex Sans".into();
    let bounds = Bounds::centered(None, size(px(1100.), px(680.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|_| TextClipping);
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    cx.activate(true);
    Ok(())
}

fn main() {
    gpui_kit::application().run(|cx| {
        if let Err(error) = open_example(cx) {
            eprintln!("Failed to open text clipping example: {error:#}");
            cx.quit();
        }
    });
}
