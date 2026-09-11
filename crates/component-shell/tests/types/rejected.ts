import { div } from 'gpui-kit';
import { Spinner, Empty, EmptyHeader, EmptyMedia } from 'gpui-component';
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput, TextareaState } from 'gpui-component';

// @ts-expect-error Group sizing uses the standard control tiers.
new InputGroup('group').size('icon-small');
// @ts-expect-error Addons use logical inline/block alignment.
new InputGroupAddon('addon').align('left');
// @ts-expect-error Addon content uses one ordered child sequence.
new InputGroupAddon('addon').button(new InputGroupButton('action'));
// @ts-expect-error Icons use an asset path.
new InputGroupButton('button').icon(123);
// @ts-expect-error Buttons expose a closed set of variants.
new InputGroupButton('button').variant('custom');
// @ts-expect-error A single-line part requires InputState.
new InputGroupInput(TextareaState());
// @ts-expect-error A part style is a declaration function.
new InputGroup('group').focused_style(12);
// @ts-expect-error Style declarations cannot add children.
new InputGroupButton('button').label_style(style => style.child('text'));
// @ts-expect-error Style declarations cannot register event handlers.
new InputGroupButton('button').icon_style(style => style.on_click(() => {}));
// @ts-expect-error Only style methods and value types are available.
new InputGroupButton('button').label_style(style => style.font_size('large'));

// @ts-expect-error Empty actions belong to its child controls.
new Empty().p(4).on_click(() => {});
// @ts-expect-error The header is an element, not plain text.
new Empty().header('No results');
// @ts-expect-error Named title slots take elements, not plain text.
new EmptyHeader().title('No results');
// @ts-expect-error EmptyMedia supports only default and icon variants.
new EmptyMedia().p(4).variant('avatar');

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
