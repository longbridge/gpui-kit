// The script half's presentation layer.
//
// `gpui-shell` binds `gpui-base`, which ships behavior and no visual style: a
// Button here has hit testing, focus and hover state, and not one pixel of
// appearance. Every colour, size and radius below is this file's decision —
// read from the render's call-scoped `cx.theme()`, so changing the shell theme
// moves this half too.

import { div } from "gpui-kit";
import { h_flex, v_flex, Button } from "gpui-base";
/** @import { AbsoluteLength, ClickEvent, Context } from "gpui-kit" */
/** @import { Quote } from "market" */

/// Every measurement here is in **rems**, so the panel scales with the window's
/// text size instead of pinning itself to a pixel grid that only exists at the
/// default zoom. The one exception is a hairline rule, which is a rule at any
/// size rather than a measurement.
///
/// `shell_story.rs` carries the same numbers. The two boards sit side by side,
/// so one that only lines up at 100% lines up by accident.
/** @type {Record<"xxs" | "xs" | "sm" | "md" | "lg" | "xl", AbsoluteLength>} */
export const SPACE = {
  xxs: "0.125rem",
  xs: "0.25rem",
  sm: "0.5rem",
  md: "0.75rem",
  lg: "1rem",
  xl: "1.5rem",
};

/** @type {Record<"symbol" | "price" | "percent" | "volume", AbsoluteLength>} */
export const COLUMN = {
  symbol: "4.875rem",
  price: "4.25rem",
  percent: "4.125rem",
  volume: "5.125rem",
};

/** @type {Record<"padding" | "gap" | "inset" | "marker" | "halfMarker", AbsoluteLength>} */
export const ROW = {
  padding: "0.125rem",
  gap: "0.125rem",
  inset: "0.5rem",
  marker: "0.375rem",
  /// Spelled out because a rem string cannot be halved by dividing.
  halfMarker: "0.1875rem",
};

/** @type {{ title: AbsoluteLength, body: AbsoluteLength, lineHeight: number }} */
export const TYPE = { title: "0.8125rem", body: "0.6875rem", lineHeight: 1.4 };

/// Down is `destructive`, flat is ordinary text — both semantic roles. Up is a
/// literal, and deliberately: the semantic set is shadcn's, which has a
/// `destructive` and no counterpart for it, so there is no token that means
/// "gain". `accent` is the near-white hover surface, and reading it as one is
/// how this column came out white on a light theme.
///
/// A gain/loss pair is a domain color anyway, the way a chart series is — it
/// belongs to the market, not to the interface — so it picks its own value per
/// appearance rather than borrowing a role that means something else.
/** @param {Context} cx */
const gain = (cx) => (cx.theme().is_dark ? "#4ade80" : "#16a34a");

/** @param {number} direction @param {Context} cx */
export const directionColor = (direction, cx) => {
  if (direction > 0) return gain(cx);
  if (direction < 0) return cx.theme().colors.destructive;
  return cx.theme().colors.foreground;
};

// -- Type -------------------------------------------------------------------

/** @param {string} value @param {Context} cx */
export const title = (value, cx) =>
  div()
    .text_size(TYPE.title)
    .line_height(1.3)
    .font_semibold()
    .text_color(cx.theme().colors.foreground)
    .child(value);

/** @param {string} value @param {Context} cx */
export const label = (value, cx) =>
  div()
    .text_size(TYPE.body)
    .line_height(TYPE.lineHeight)
    .text_color(cx.theme().colors.foreground)
    .child(value);

/** @param {string} value @param {Context} cx */
export const muted = (value, cx) =>
  div()
    .text_size(TYPE.body)
    .line_height(TYPE.lineHeight)
    .text_color(cx.theme().colors.muted_foreground)
    .child(value);

// -- Surfaces ---------------------------------------------------------------

/// The panel's root: layout only. The Rust `section` around it already draws the
/// card, and the Rust board has no inner frame either.
export const surface = () => v_flex().w_full().gap(SPACE.md);

// One real pixel: a rule is a rule at any zoom, not a measurement that scales.
/** @param {Context} cx */
export const rule = (cx) => div().w_full().h(1).flex_none().bg(cx.theme().colors.border);

// -- Board parts ------------------------------------------------------------

/** @param {AbsoluteLength} width @param {{ right?: boolean }} [options] */
export const cell = (width, options = {}) => {
  const box = div().w(width).flex_none();
  return options.right ? box.text_right() : box;
};

/// The header. It ends with an empty cell the width of the watched marker,
/// because a trailing column the header does not know about puts every caption
/// out of line with the numbers under it.
/** @param {Context} cx */
export const header = (cx) =>
  h_flex()
    .w_full()
    .items_center()
    .gap(ROW.inset)
    .px(ROW.inset)
    .pb(SPACE.xs)
    .border_b(1)
    .border_color(cx.theme().colors.border)
    .child(cell(COLUMN.symbol).child(muted("Symbol", cx)))
    .child(div().flex_1())
    .child(cell(COLUMN.price, { right: true }).child(muted("Last", cx)))
    .child(cell(COLUMN.percent, { right: true }).child(muted("Change", cx)))
    .child(cell(COLUMN.volume, { right: true }).child(muted("Volume", cx)))
    .child(cell(ROW.marker));

/// A full-width row that behaves like a button. The id is the symbol rather than
/// the row's position, so identity follows the instrument if the board reorders.
/**
 * @param {Quote} quote
 * @param {(event: ClickEvent, cx: Context) => void} onClick
 * @param {Context} cx
 */
export const quoteRow = (quote, onClick, cx) =>
  Button.new(`quote-${quote.symbol}`)
    .accessibility_label(`Watch ${quote.name}`)
    .flex()
    .w_full()
    .items_center()
    .gap(ROW.inset)
    .px(ROW.inset)
    .py(ROW.padding)
    .rounded(cx.theme().radius.md)
    .hover((style) => style.bg(cx.theme().colors.muted))
    .on_click(onClick)
    .child(cell(COLUMN.symbol).child(label(quote.symbol, cx).font_medium()))
    .child(div().flex_1().child(muted(quote.name, cx)))
    .child(
      cell(COLUMN.price, { right: true }).child(
        label(quote.last, cx).text_color(directionColor(quote.direction, cx)),
      ),
    )
    .child(
      cell(COLUMN.percent, { right: true }).child(
        label(quote.percent, cx).text_color(directionColor(quote.direction, cx)),
      ),
    )
    .child(cell(COLUMN.volume, { right: true }).child(muted(quote.volume, cx)))
    .child(watchMarker(quote.watched, cx));

/** @param {boolean} watched @param {Context} cx */
export const watchMarker = (watched, cx) =>
  div()
    .w(ROW.marker)
    .h(ROW.marker)
    .flex_none()
    .rounded(ROW.halfMarker)
    .when(watched, (el) => el.bg(cx.theme().colors.primary));

/// A labelled action. Two treatments only — filled and outlined.
/**
 * @param {string} id
 * @param {string} caption
 * @param {(event: ClickEvent, cx: Context) => void} onClick
 * @param {Context} cx
 * @param {{ disabled?: boolean }} [options]
 */
export const action = (id, caption, onClick, cx, options = {}) => {
  const { disabled = false } = options;

  return Button.new(id)
    .disabled(disabled)
    .flex()
    .items_center()
    .justify_center()
    .h("1.25rem")
    .px(SPACE.sm)
    .rounded(cx.theme().radius.md)
    .border(1)
    .border_color(cx.theme().colors.border)
    .bg(cx.theme().colors.background)
    .when(disabled, (el) => el.opacity(0.5))
    .when(!disabled, (el) =>
      el.hover((style) => style.bg(cx.theme().colors.muted)).on_click(onClick),
    )
    .child(
      div()
        .text_size(TYPE.body)
        .line_height(1)
        .text_color(cx.theme().colors.foreground)
        .child(caption),
    );
};
