import { div, View, Background, type Element, type NativeElement, type Context, type BoxShadowSpec } from 'gpui-kit';
import { Button, Spinner, Separator, Skeleton, HForm, Field, type SpinnerElement } from 'gpui-component';
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
    const raised: BoxShadowSpec[] = [
      { spread: 1, color: '#0d0d0f' },
      { y: 1, color: '#ffffff10', inset: true },
      { y: 1, blur: 2, color: '#00000066' },
    ];
    return native.p(2).size(240).role('status').transition('opacity', 120).children([
      padded(element),
      Button.new('typed-action').label('Run').p(2)
        .accessibility_label('Run the primary action').primary()
        .on_click((_event, _cx) => {}),
      div().rounded(10)
        .bg(Background.linear_gradient(180, Background.stop('#f7da74', 0), Background.stop('#fdcd12', 1)))
        .shadow(raised)
        .hover(style => style.bg('#fff').shadow([])),
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
        .input(new InputGroupInput(this.query).aria_label('Search').value('')
          .content_type('url').readonly(false).px(12).text_base()
          .on_change((_value, _cx) => {}))
        .addon(new InputGroupAddon('search-actions').align('inline-end')
          .child(new InputGroupText().child('Results'))
          .child(new InputGroupButton('clear').label('Clear').icon('icons/x.svg').size('xsmall')
            .on_click((_event, _cx) => {}))),
      new InputGroup('typed-message').w(320)
        .input(new InputGroupTextarea(this.message).placeholder('Message').auto_grow(2, 6)
          .p(12).text_base())
        .addon(new InputGroupAddon('message-actions').align('block-end')
          .child(new InputGroupButton('send').variant('primary').label('Send'))),
    ]);
  }
}
