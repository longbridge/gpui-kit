import { div, View } from 'gpui-kit';
import { Spinner, Separator, Skeleton } from 'gpui-component';
import { TextView } from 'gpui-base';

export default class FluentContracts extends View {
  render() {
    return div().children([
      new Spinner().size('medium').p(4).flex().child(false).children([])
        .when(true, element => element.size('small').p(2))
        .map(element => element.size('large')).size('small'),
      new Separator().p(2).label('Section'),
      new Skeleton().flex().secondary(),
      TextView.markdown('text', '# Hello').p(2).selectable().flex().scrollable(),
    ]);
  }
}
