import { div } from 'gpui-kit';
import { Spinner } from 'gpui-component';

// @ts-expect-error Registered components reject undeclared click handlers.
new Spinner().on_click(() => {});
// @ts-expect-error Style calls must not expose a forbidden handler.
new Spinner().p(4).on_click(() => {});
// @ts-expect-error Nullary styles must not expose a forbidden handler.
new Spinner().flex().on_click(() => {});
// @ts-expect-error Child calls must preserve the component contract.
new Spinner().child('Loading').on_click(() => {});
// @ts-expect-error Children calls must preserve the component contract.
new Spinner().children([]).disabled(true);
// @ts-expect-error Conditional callbacks must preserve the component contract.
new Spinner().when(true, element => element.on_click(() => {}));
// @ts-expect-error Conditional results must preserve the component contract.
new Spinner().when(false, element => element).selected(true);
// @ts-expect-error Map callbacks must preserve the component contract.
new Spinner().map(element => element.on_click(() => {}));
// @ts-expect-error Map results must preserve the component contract.
new Spinner().map(element => element.p(2)).on_click(() => {});
// @ts-expect-error An adapter size is not a general length.
new Spinner().p(4).size(24);
// @ts-expect-error Invalid size names remain invalid after conditionals.
new Spinner().when(true, element => element).size('huge');
// @ts-expect-error General elements do not have TextView behaviors.
div().p(2).selectable();

// @ts-expect-error Native size remains a length, not an adapter size name.
div().size('small');

// @ts-expect-error Registered Spinner does not expose the native role behavior.
new Spinner().role('status');
// @ts-expect-error Styling must not restore the unsupported role behavior.
new Spinner().p(4).role('status');
// @ts-expect-error Registered Spinner does not expose native transitions.
new Spinner().transition('opacity', 120);
// @ts-expect-error Styling must not restore unsupported transitions.
new Spinner().p(4).transition('opacity', 120);
