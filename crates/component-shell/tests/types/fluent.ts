import { div, View, type Element, type NativeElement, type Context } from 'gpui-kit';
import { Spinner, Separator, Skeleton, HForm, Field, type SpinnerElement } from 'gpui-component';
import { TextView } from 'gpui-base';
import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription, EmptyContent } from 'gpui-component';
import {
  InputState, TextareaState, InputGroup, InputGroupInput, InputGroupTextarea,
  InputGroupAddon, InputGroupButton, InputGroupText,
} from 'gpui-component';

function padded(element: Element): Element {
  return element.p(2).when(true, current => current.p(4));
}

export default class FluentContracts extends View {
  query!: ReturnType<typeof InputState>;
  message!: ReturnType<typeof TextareaState>;

  init() {
    this.query = InputState('Search');
    this.message = TextareaState();
  }

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
      new Empty().p(16)
        .header(new EmptyHeader().items_start()
          .media(new EmptyMedia().variant('icon').p(2).child('!'))
          .title(new EmptyTitle().font_semibold().child('No results'))
          .description(new EmptyDescription().child('Try another query.')))
        .content(new EmptyContent().gap(8).child(div().child('Custom content')))
        .child('Additional content'),
      new InputGroup('typed-search').w(320).size('medium').invalid(false)
        .focused_style(style => style.border_color('#2563eb'))
        .invalid_style(style => style.border_color('#dc2626'))
        .disabled_style(style => style.opacity(0.7))
        .input(new InputGroupInput(this.query).aria_label('Search').value('')
          .content_type('url').readonly(false)
          .editor_style(style => style.px(12).text_base())
          .on_change((_value, _cx) => {}))
        .addon(new InputGroupAddon('search-actions').align('inline-end')
          .child(new InputGroupText().child('Results'))
          .child(new InputGroupButton('clear').label('Clear').icon('icons/x.svg').size('xsmall')
            .label_style(style => style.font_semibold())
            .icon_style(style => { style.size_4().when(true, s => s.opacity(0.8)); })
            .on_click((_event, _cx) => {}))),
      new InputGroup('typed-message').w(320)
        .input(new InputGroupTextarea(this.message).placeholder('Message').auto_grow(2, 6)
          .editor_style(style => style.p(12).text_base()))
        .addon(new InputGroupAddon('message-actions').align('block-end')
          .child(new InputGroupButton('send').variant('primary').label('Send'))),
    ]);
  }
}
