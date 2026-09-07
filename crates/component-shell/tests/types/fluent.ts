import { div, View, type Element, type NativeElement, type Context } from 'gpui-kit';
import { Spinner, Separator, Skeleton, type SpinnerElement } from 'gpui-component';
import { TextView } from 'gpui-base';

const spinner: SpinnerElement = new Spinner()
  .size('medium')
  .p(4)
  .flex()
  .hover(style => style.p(2))
  .active(style => style.p(1))
  .focus(style => style.p(2))
  .bg('#fff')
  .role('status')
  .transition('opacity', 120)
  .child('Loading')
  .children(['More'])
  .when(true, element => element.size('small').p(2))
  .map(element => element.size('large'))
  .size('small');
const element: Element = spinner;
div().child(spinner).children([new Separator().p(2).label('Section'), new Skeleton().flex().secondary()]);
class Example extends View {
  render(_cx: Context): Element { return new Spinner().p(4).size('small'); }
}
TextView.markdown('text', '# Hello').p(2).selectable().flex().scrollable();
const answer: number = new Spinner().map(element => {
  element.size('small');
  return 42;
});

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

function padded(element: Element): Element { return element.p(2).child('Content').when(true, current => current.p(4)); }
const native: NativeElement = div();
native.p(2).size(24);
padded(new Spinner().size('small'));
