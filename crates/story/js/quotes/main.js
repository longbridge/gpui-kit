// The JavaScript half of the "Shell" story.
//
//   cargo run   →   Gallery   →   Shell
//
// Read from disk when the story opens, and again on "Reload script". Editing
// this file changes the right-hand panel with no `cargo build` in between,
// which is the entire argument for a script layer.
//
// It owns no state. The board lives in a Rust `Entity<Market>`, imported from
// the host module the story registered before the runtime started:
//
//   import { quotes, ticks, watch, watch_all, summary } from "market";
//
// `summary()` is the asynchronous one: it answers with a promise and its work
// runs off the main thread, which is visible here as the board continuing to
// tick while it is in flight.
//
// Twenty rows of six cells, rebuilt from scratch every time a price moves —
// twenty times a second with the default feed. The counters under the panels
// report what that costs, and what a repaint costs when nothing here changed.

import { View } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";
import {
  quotes as readQuotes,
  summary as readSummary,
  ticks as readTicks,
  watch,
  watch_all,
} from "market";
/** @import { Quote } from "market" */
import {
  ROW,
  SPACE,
  action,
  header,
  label,
  muted,
  quoteRow,
  rule,
  surface,
  title,
} from "./ui.js";

export default class QuoteBoard extends View {
  render(cx) {
    const quotes = readQuotes();
    const watched = quotes.filter((quote) => quote.watched).length;

    // With the quotes feed running this climbs; with the repaint feed it holds
    // still, because this render is not being called and the frame on screen is
    // the one it produced last time.
    const ticks = readTicks();

    return surface()
      .child(this.heading(quotes.length, watched, ticks, cx))
      .child(header(cx))
      .child(this.rows(quotes, cx))
      .child(rule(cx))
      .child(this.actions(quotes.length, watched, cx));
  }

  /**
   * @param {number} total
   * @param {number} watched
   * @param {number} ticks
   * @param {import("gpui-kit").Context} cx
   */
  heading(total, watched, ticks, cx) {
    return h_flex()
      .w_full()
      .items_start()
      .justify_between()
      .gap(ROW.inset)
      .child(
        v_flex()
          .gap(SPACE.xxs)
          .child(title("Live quotes", cx))
          .child(muted("Drawn by main.js · prices imported from \"market\"", cx)),
      )
      .child(
        v_flex()
          .items_end()
          .gap(SPACE.xxs)
          .child(label(`${watched} / ${total} watched`, cx))
          .child(muted(`tick ${ticks}`, cx)),
      );
  }

  /**
   * @param {Quote[]} quotes
   * @param {import("gpui-kit").Context} cx
   */
  rows(quotes, cx) {
    if (quotes.length === 0) {
      return muted("The Rust panel is holding no quotes.", cx);
    }

    return v_flex()
      .w_full()
      .gap(ROW.gap)
      .children(
        // No `cx.notify()`: the host call asks Rust to change the board, Rust
        // notifies its observers, and both halves re-render from one change.
        quotes.map((quote) => quoteRow(quote, () => watch(quote.symbol), cx)),
      );
  }

  /**
   * @param {number} total
   * @param {number} watched
   * @param {import("gpui-kit").Context} cx
   */
  actions(total, watched, cx) {
    return h_flex()
      .w_full()
      .items_center()
      .justify_between()
      .gap(ROW.inset)
      // One line, showing whichever of the two is worth saying: the summary
      // once it has been asked for, and otherwise the empty watchlist — which
      // is the only thing the heading's "N / M watched" does not already say.
      .child(muted(this.summaryLine(watched), cx))
      .child(
        h_flex()
          .gap(SPACE.xs)
          .child(
            action("summary", "Summary", (_event, cx) => this.loadSummary(cx), cx, {
              disabled: this.loading === true,
            }),
          )
          .child(
            action("watch-all", "Watch all", () => watch_all(true), cx, {
              disabled: watched === total,
            }),
          )
          .child(
            action("watch-none", "Clear", () => watch_all(false), cx, {
              disabled: watched === 0,
            }),
          ),
      );
  }

  /**
   * @param {number} watched
   * @returns {string}
   */
  summaryLine(watched) {
    if (this.loading) return "Reading the session…";
    if (this.summary) {
      const { leader, leader_percent, laggard, laggard_percent, average_percent } = this.summary;
      return `${leader} ${leader_percent} · ${laggard} ${laggard_percent} · avg ${average_percent}`;
    }
    return watched === 0 ? "Nothing on the watchlist" : "";
  }

  /**
   * `summary()` answers with a promise, so this returns immediately and the
   * board keeps ticking while it is in flight — which is the whole reason that
   * function is `async_function` on the Rust side rather than `function`.
   *
   * @param {import("gpui-kit").Context} cx
   */
  loadSummary(cx) {
    this.loading = true;
    this.summary = null;
    cx.notify();

    cx.spawn(async (cx) => {
      try {
        this.summary = await readSummary();
      } catch (error) {
        this.summary = null;
        console.error(`summary failed: ${error.message}`);
      }
      this.loading = false;
      cx.notify();
    });
  }
}
