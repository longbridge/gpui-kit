// A todo list.
//
// It exists to exercise the whole runtime rather than to be minimal: retained
// input state, controlled checkboxes, a dialog, a toast, capability-gated
// storage, and a filter that has to survive every mutation. If something in
// gpui-shell is broken, this is where it shows.
//
//   cargo run -p gpui-shell -- examples/js_todolist

import { View } from "gpui-kit";
import { v_flex, h_flex, InputState } from "gpui-base";
/** @import { AsyncContext, Context } from "gpui-kit" */
import { load, save } from "./storage.js";
import confirmClear from "./confirm.js";
import {
  SPACE,
  button,
  icon,
  iconButton,
  checkbox,
  emptyState,
  field,
  label,
  muted,
  row,
  rule,
  surface,
  title,
} from "./ui.js";

/** @type {{ id: Filter, caption: string }[]} */
const FILTERS = [
  { id: "all", caption: "All" },
  { id: "active", caption: "Active" },
  { id: "done", caption: "Done" },
];

export default class TodoList extends View {
  /** @param {unknown} _props @param {AsyncContext} cx */
  init(_props, cx) {
    // Annotated where they are assigned rather than declared as class fields.
    // `View`'s constructor calls `init` from inside `super()`, so a field
    // declaration — even one with no initializer — would run afterwards and
    // write `undefined` over everything set here.
    /** @type {InputState} */
    this.draft = InputState.new({ placeholder: "What needs doing?" });
    // Enter is how a list like this is actually used; the Add button is for
    // the pointer, not the primary path.
    this.draft.on("submit", (_event, cx) => this.add(cx));
    /** @type {Todo[]} */
    this.items = load();
    /** @type {Filter} */
    this.filter = "all";
    this.nextId = this.items.reduce((max, item) => Math.max(max, item.id), 0) + 1;
    this.persisted = true;
  }

  get remaining() {
    return this.items.filter((item) => !item.done).length;
  }

  get completed() {
    return this.items.filter((item) => item.done).length;
  }

  visible() {
    if (this.filter === "all") return this.items;
    return this.items.filter((item) => (this.filter === "done") === item.done);
  }

  /** @param {Context} cx */
  commit(cx) {
    this.persisted = save(this.items);
    cx.notify();
  }

  /** @param {Context} cx */
  add(cx) {
    const caption = this.draft.value().trim();
    if (caption === "") return;

    this.items = [...this.items, { id: this.nextId, caption, done: false }];
    this.nextId += 1;
    this.draft.set_value("");
    this.commit(cx);
  }

  /** @param {number} id @param {boolean} done @param {Context} cx */
  toggle(id, done, cx) {
    this.items = this.items.map((item) => (item.id === id ? { ...item, done } : item));
    this.commit(cx);
  }

  /** @param {number} id @param {Context} cx */
  remove(id, cx) {
    this.items = this.items.filter((item) => item.id !== id);
    this.commit(cx);
  }

  /** @param {Filter} filter @param {Context} cx */
  setFilter(filter, cx) {
    this.filter = filter;
    cx.notify();
  }

  /** @param {Context} cx */
  clearCompleted(cx) {
    const count = this.completed;
    if (count === 0) return;
    // The dialog is a function returning an element. What it shows comes from
    // what this call closed over, so there is no second channel for handing a
    // view its starting state.
    window.open_dialog(
      confirmClear(count, cx, () => {
        this.items = this.items.filter((item) => !item.done);
        this.persisted = save(this.items);
        window.push_toast({
          title: `Deleted ${count} ${count === 1 ? "item" : "items"}`,
          level: "info",
          id: "cleared",
        });
      }),
    );
  }

  /** @param {Context} cx */
  render(cx) {
    const visible = this.visible();

    return v_flex()
      .size_full()
      .bg(cx.theme().colors.background)
      .p(SPACE.xl)
      .gap(SPACE.lg)
      .child(this.header(cx))
      .child(this.composer(cx))
      .child(
        surface(cx)
          .child(this.toolbar(cx))
          .child(rule(cx))
          .child(
            visible.length === 0
              ? emptyState(...this.emptyCopy(), cx)
              : v_flex().flex_1().py(SPACE.xs).children(visible.map((item) => this.row(item, cx))),
          ),
      )
      .child(this.footer(cx));
  }

  /** @param {Context} cx */
  header(cx) {
    return h_flex()
      .items_center()
      .justify_between()
      .child(row().gap(SPACE.sm).child(icon("list", 18)).child(title("Todo", cx)))
      .child(
        muted(
          this.items.length === 0
            ? "Nothing yet"
            : `${this.remaining} of ${this.items.length} remaining`,
          cx,
        ),
      );
  }

  /** @param {Context} cx */
  composer(cx) {
    return row()
      .child(field(this.draft, cx))
      .child(
        button("add", "Add", (_event, cx) => this.add(cx), cx, {
          variant: "primary",
          icon: "plus",
        }),
      );
  }

  /** @param {Context} cx */
  toolbar(cx) {
    const filters = FILTERS.map((entry) =>
      button(`filter-${entry.id}`, entry.caption, (_event, cx) => this.setFilter(entry.id, cx), cx, {
        variant: "ghost",
        selected: this.filter === entry.id,
      }),
    );

    return h_flex()
      .items_center()
      .justify_between()
      .px(SPACE.md)
      .py(SPACE.sm)
      .child(h_flex().gap(SPACE.xs).children(filters))
      .child(
        button("clear", "Clear completed…", (_event, cx) => this.clearCompleted(cx), cx, {
          variant: "danger",
          disabled: this.completed === 0,
        }),
      );
  }

  /** @param {Todo} item @param {Context} cx */
  row(item, cx) {
    return checkbox(
      `item-${item.id}`,
      item.done,
      (done, cx) => this.toggle(item.id, done, cx),
      h_flex()
        .flex_1()
        .items_center()
        .justify_between()
        .gap(SPACE.md)
        .child(
          label(item.caption, cx).when(item.done, (el) =>
            el.text_color(cx.theme().colors.muted_foreground).line_through(),
          ),
        )
        .child(
          iconButton(
            "remove-" + item.id,
            "trash",
            `Remove “${item.caption}”`,
            (_event, cx) => this.remove(item.id, cx),
            cx,
          ),
        ),
      cx,
    );
  }

  /** @param {Context} cx */
  footer(cx) {
    return h_flex()
      .items_center()
      .justify_between()
      .child(
        muted(
          this.persisted
            ? "Saved"
            : "Not saved — this host did not grant storage, so the list lasts for this run only",
          cx,
        ),
      )
      .child(muted(`${this.completed} completed`, cx));
  }

  /** @returns {[string, string]} */
  emptyCopy() {
    if (this.items.length === 0) {
      return ["No items yet", "Type above and press Add."];
    }
    if (this.filter === "done") {
      return ["Nothing finished yet", "Tick an item to see it here."];
    }
    return ["All done", "Switch to All to review what you finished."];
  }
}
