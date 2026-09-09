// The application's own shapes.
//
// Hand-written, unlike `gpui-kit.d.ts`: the runtime knows what `div()` returns, and
// has no idea what a todo is. Declaring them here rather than as `@typedef`
// blocks in the source keeps the type in one place and the annotations at the
// call sites down to a name.

/** One item on the list, and the only shape storage round-trips. */
interface Todo {
  id: number;
  caption: string;
  done: boolean;
}

/** Which items the toolbar is showing. */
type Filter = "all" | "active" | "done";

/** The two treatments plus the two quiet ones. */
type Variant = "primary" | "secondary" | "ghost" | "danger";

interface ButtonOptions {
  variant?: Variant;
  disabled?: boolean;
  /** Drawn selected, for a filter that is the current one. */
  selected?: boolean;
  /** A name under `icons/`, drawn before the caption. */
  icon?: string;
}
