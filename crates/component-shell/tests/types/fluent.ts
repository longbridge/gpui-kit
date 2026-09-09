import { div, View, type Element, type NativeElement, type Context } from 'gpui-kit';
import { Spinner, Separator, Skeleton, HForm, Field, type SpinnerElement } from 'gpui-component';
import { TextView } from 'gpui-base';

function padded(element: Element): Element {
  return element.p(2).when(true, current => current.p(4));
}

export default class FluentContracts extends View {
  render(_cx: Context): Element {
    const spinner: SpinnerElement = new Spinner()
      .size('medium').p(4).flex()
      .hover(style => style.p(2))
      .active(style => style.p(1))
      .focus(style => style.p(2))
      .bg('#fff')
      .when(true, element => element.size('small').p(2))
      .map(element => element.size('large')).size('small');
    const element: Element = spinner;
    const answer: number = new Spinner().map(current => {
      current.size('small');
      return 42;
    });
    if (answer !== 42) throw new Error('map must preserve its return value');
    const native: NativeElement = div();
    return native.p(2).size(240).role('status').transition('opacity', 120).children([
      padded(element),
      new Separator().p(2).label('Section'),
      new Skeleton().flex().secondary(),
      new HForm().child(new Field().label('Name').child('Ada')).children([]).columns(2),
      TextView.markdown('text', '# Hello').p(2).selectable().flex().scrollable(),
    ]);
  }
}
