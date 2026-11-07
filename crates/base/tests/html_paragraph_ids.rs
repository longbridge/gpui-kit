#![cfg(feature = "test-support")]

#[cfg(feature = "gpui-fast")]
use gpui_fast as gpui;

use gpui::{Context, IntoElement, ParentElement as _, Render, TestAppContext, Window, div};
use gpui_base::{TextView, test_support::texts};

struct HtmlDoc(String);

impl Render for HtmlDoc {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(TextView::html("doc", self.0.clone()))
    }
}

/// Span-less HTML paragraphs shared one element id, so every inline flow
/// mounted under it and read another paragraph's cached layout: all but the
/// last paragraph painted no text (#3668).
#[gpui::test]
fn html_paragraphs_paint_their_own_text(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    let html = r##"<p>1 at the end <code>y</code></p>
<p>2 in the middle <code>y</code> z</p>
<p>3 link <a href="#">w <code>y</code> v</a> z</p>
<p>4 link <a href="#">w <code>y</code></a> z</p>
<p>5 <code>y</code> at the start</p>
<p>6 two <code>a</code> and <code>b</code> end</p>"##
        .to_string();
    let (_, vcx) = cx.add_window_view(move |_, _| HtmlDoc(html));
    vcx.run_until_parked();
    vcx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let painted: Vec<String> = texts(window)
            .iter()
            .map(|snapshot| snapshot.text().to_string())
            .collect();
        for fragment in [
            "1 at the end",
            "2 in the middle",
            "3 link",
            "4 link",
            "at the start",
            "6 two",
        ] {
            assert!(
                painted.iter().any(|text| text.contains(fragment)),
                "{fragment:?} was not painted: {painted:?}"
            );
        }
    });
}
