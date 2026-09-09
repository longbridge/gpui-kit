use gpui_kit::component::{button::*, h_flex, v_flex, *};
use gpui_kit::*;

pub struct Example {
    trap1_handle: FocusHandle,
    trap2_handle: FocusHandle,
}
impl Example {
    fn new(cx: &mut App) -> Self {
        Self {
            trap1_handle: cx.focus_handle(),
            trap2_handle: cx.focus_handle(),
        }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_6()
            .p_8()
            .child(div().text_xl().font_bold().child("Focus Trap Example"))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Press Tab to navigate between buttons. Notice how focus cycles within different areas."),
            )
            // Outside buttons - not in focus trap
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("Outside Area (No Focus Trap)"),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("outside-1").label("Outside Button 1"))
                            .child(Button::new("outside-2").label("Outside Button 2"))
                            .child(Button::new("outside-3").label("Outside Button 3")),
                    ),
            )
            // Focus trap area 1
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_base().font_semibold().child("Focus Trap Area 1"))
                    .child(
                        h_flex()
                            .gap_2()
                            .p_4()
                            .bg(cx.theme().secondary)
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("trap1-1")
                                    .label("Trap 1 - Button 1")
                                    .on_click(|_, _, _| println!("Trap 1 - Button 1 clicked")),
                            )
                            .child(
                                Button::new("trap1-2")
                                    .label("Trap 1 - Button 2")
                                    .on_click(|_, _, _| println!("Trap 1 - Button 2 clicked")),
                            )
                            .child(
                                Button::new("trap1-3")
                                    .label("Trap 1 - Button 3")
                                    .on_click(|_, _, _| println!("Trap 1 - Button 3 clicked")),
                            )
                            .focus_trap("trap1", &self.trap1_handle),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("→ Press Tab in this area, focus cycles through 3 buttons without escaping"),
                    ),
            )
            // Middle outside buttons
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_base()
                            .font_semibold()
                            .child("Outside Area (No Focus Trap)"),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("outside-4").label("Outside Button 4"))
                            .child(Button::new("outside-5").label("Outside Button 5")),
                    ),
            )
            // Focus trap area 2
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_base().font_semibold().child("Focus Trap Area 2"))
                    .child(
                        v_flex()
                            .focus_trap("trap2", &self.trap2_handle)
                            .gap_2()
                            .p_4()
                            .grid()
                            .grid_cols(4)
                            .bg(cx.theme().accent.opacity(0.1))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().accent)
                            .child(Button::new("trap2-1").label("Trap 2 - Button 1"))
                            .child(Button::new("trap2-2").label("Trap 2 - Button 2"))
                            .child(
                                Button::new("trap2-3").label("Trap 2 - Button 3"),
                            )
                            .child(Button::new("trap2-4").label("Trap 2 - Button 4"))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("→ Press Tab in this area, focus cycles through 4 buttons without escaping"),
                    ),
            )
    }
}

fn main() {
    let app = gpui_kit::application();

    app.run(move |cx| {
        gpui_kit::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| Example::new(cx));
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
