# Generated type contracts

Run from the repository root:

```sh
npm --prefix crates/component-shell/tests/types ci
npm --prefix crates/component-shell/tests/types test
```

The harness generates fresh declarations with the styled host, compiles the strict positive TypeScript fixture and executes its emitted JavaScript, checks a separate strict negative fixture, checks the shipped registered-component JavaScript gallery, and runs valid and invalid scripts through the actual CLI. Temporary declarations and applications are removed after each run. Set `GPUI_COMPONENT_SHELL_BIN` to use an already built host; otherwise the harness uses `cargo run --locked`.

Fluent methods retain the receiver's component contract. For example, `new Spinner().p(4).size('small')` remains valid, while `.on_click(...)` remains unavailable after styling or conditional chaining. `Spinner.role(...)` and `Spinner.transition(...)` are also rejected, including after styling; these behaviors remain supported on native `div()` builders. There is no separate hand-maintained runtime copy of the positive fixture. Native factories return `NativeElement`; `Element` is the union of renderable native and registered builders. Use `NativeElement` when a helper specifically needs native signatures such as numeric size, and `Element` for general render results and children.

Consumer checking uses `skipLibCheck`, matching the generated editor configuration. It does not validate all ambient runtime library declarations. `@ts-expect-error` fixtures fail if a forbidden call becomes accepted; they are not casts that suppress unrelated fixture failures.
