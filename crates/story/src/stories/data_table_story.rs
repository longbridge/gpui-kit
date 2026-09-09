use std::{
    ops::Range,
    path::PathBuf,
    sync::LazyLock,
    time::{self, Duration},
};

use fake::Fake;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, StyleSized as _, StyledExt,
    button::Button,
    h_flex,
    menu::PopupMenu,
    spinner::Spinner,
    table::{
        Column, ColumnFixed, ColumnGroup, ColumnSort, DataTable, TableDelegate, TableEvent,
        TableState,
    },
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::{Deserialize, Serialize};

use crate::story_toolbar_group;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
struct ChangeSize(Size);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
struct ChangeRows(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
struct ChangeExtraColumns(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
struct OpenDetail(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
enum ToggleTableOption {
    LoopSelection,
    ColumnResize,
    ColumnOrder,
    Sortable,
    ColumnSelection,
    RowSelection,
    CellSelection,
    RowHeader,
    FixedColumn,
    Striped,
    Loading,
    LazyLoad,
    RefreshData,
    GroupHeaders,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
struct ClearTableSelection;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = data_table_story, no_json)]
enum GoToTablePosition {
    Top,
    Bottom,
    Cell53,
    Cell107,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Counter {
    symbol: SharedString,
    market: SharedString,
    name: SharedString,
}

static ALL_COUNTERS: LazyLock<Vec<Counter>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../fixtures/counters.json")).unwrap());
static INCREMENT_ID: LazyLock<std::sync::Mutex<usize>> = LazyLock::new(|| std::sync::Mutex::new(0));

const CSV_EXPORT_BATCH_ROWS: usize = 2_000;

impl Counter {
    fn random() -> Self {
        let len = ALL_COUNTERS.len();
        let ix = rand::random::<usize>() % len;
        ALL_COUNTERS[ix].clone()
    }

    /// The symbol carrying its market, e.g. `AAPL.US`.
    ///
    /// Hong Kong symbols already ship the suffix (`0700.HK`), so appending the
    /// market again would read `0700.HK.HK`.
    fn symbol_code(&self) -> SharedString {
        if self.symbol.contains('.') {
            self.symbol.clone()
        } else {
            format!("{}.{}", self.symbol, self.market).into()
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Stock {
    id: usize,
    counter: Counter,
    price: f64,
    change: f64,
    change_percent: f64,
    volume: f64,
    turnover: f64,
    market_cap: f64,
    ttm: f64,
    five_mins_ranking: f64,
    th60_days_ranking: f64,
    year_change_percent: f64,
    bid: f64,
    bid_volume: f64,
    ask: f64,
    ask_volume: f64,
    open: f64,
    prev_close: f64,
    high: f64,
    low: f64,
    turnover_rate: f64,
    rise_rate: f64,
    amplitude: f64,
    pe_status: f64,
    pb_status: f64,
    volume_ratio: f64,
    bid_ask_ratio: f64,
    latest_pre_close: f64,
    latest_post_close: f64,
    pre_market_cap: f64,
    pre_market_percent: f64,
    pre_market_change: f64,
    post_market_cap: f64,
    post_market_percent: f64,
    post_market_change: f64,
    float_cap: f64,
    shares: i64,
    shares_float: i64,
    day_5_ranking: f64,
    day_10_ranking: f64,
    day_30_ranking: f64,
    day_120_ranking: f64,
    day_250_ranking: f64,
}

impl Stock {
    /// Ticks the quote, keeping the fields consistent with the new price.
    fn random_update(&mut self) {
        self.change_percent = (-0.1..0.1).fake::<f64>();
        self.price = (5.0..999.0).fake::<f64>();
        self.change = self.price * self.change_percent;
        self.prev_close = self.price - self.change;
        self.volume = (1e4..5e7).fake::<f64>();
        self.turnover = self.volume * self.price;
        self.market_cap = self.price * self.shares as f64;
        self.ttm = (5.0..80.0).fake::<f64>();
        self.five_mins_ranking = self.five_mins_ranking * (1.0 + (-0.2..0.2).fake::<f64>());
        self.bid = self.price * (1.0 - (0.0..0.01).fake::<f64>());
        self.bid_volume = (100.0..5e4).fake::<f64>();
        self.ask = self.price * (1.0 + (0.0..0.01).fake::<f64>());
        self.ask_volume = (100.0..5e4).fake::<f64>();
        self.bid_ask_ratio = self.bid_volume / self.ask_volume;
        self.volume_ratio = (0.0..3.0).fake::<f64>();
        self.high = self.price * (1.0 + (0.0..0.08).fake::<f64>());
        self.low = self.price * (1.0 - (0.0..0.08).fake::<f64>());
    }
}

/// Returns the (foreground, background) colors for a change value.
///
/// A rise is green and a fall is red, both taken from the theme. Unchanged
/// values keep the default cell colors.
fn change_colors(val: f64, cx: &App) -> Option<(Hsla, Hsla)> {
    if val > 0. {
        Some((cx.theme().green, cx.theme().green_light))
    } else if val < 0. {
        Some((cx.theme().red, cx.theme().red_light))
    } else {
        None
    }
}

/// Formats a large number with a compact unit, e.g. `1.24B`, so wide columns
/// stay readable.
fn compact(val: f64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")];

    UNITS
        .into_iter()
        .find(|(scale, _)| val.abs() >= *scale)
        .map(|(scale, unit)| format!("{:.2}{unit}", val / scale))
        .unwrap_or_else(|| format!("{val:.2}"))
}

/// Restarts ids from zero and builds a fresh set of rows.
fn regenerate_stocks(size: usize) -> Vec<Stock> {
    {
        let mut id_lock = INCREMENT_ID.lock().unwrap();
        *id_lock = 0;
    }

    random_stocks(size)
}

fn random_stocks(size: usize) -> Vec<Stock> {
    // Incremental ID with size.
    let start = {
        let mut id_lock = INCREMENT_ID.lock().unwrap();
        let start = *id_lock;
        *id_lock += size + 1;
        start
    };

    // Drawing all 40-odd fields per row costs ~24s for a million rows in a debug
    // build, which is most of the time spent switching to the largest example.
    // Draw a pool of fully random rows instead and clone from it, still giving
    // every row its own id and counter so no two rows read alike on screen.
    const POOL_SIZE: usize = 512;
    if size > POOL_SIZE * 2 {
        let pool = random_stocks_exact(0, POOL_SIZE);
        return (start..start + size)
            .map(|id| {
                let mut stock = pool[(id - start) % POOL_SIZE].clone();
                stock.id = id;
                stock.counter = Counter::random();
                stock
            })
            .collect();
    }

    random_stocks_exact(start, size)
}

/// Builds `size` rows of randomized quotes.
///
/// The fields of one row hang together the way a real quote does: the change is
/// the percentage applied to the price, the previous close sits a change away
/// from it, and the bid, ask, open, high and low stay within a day's range of
/// it. A table of unrelated numbers reads as noise.
fn random_stocks_exact(start: usize, size: usize) -> Vec<Stock> {
    (start..start + size)
        .map(|id| {
            let price = (5.0..999.0).fake::<f64>();
            let change_percent = (-0.1..0.1).fake::<f64>();
            let change = price * change_percent;
            let shares = (1_000_000..3_000_000_000i64).fake::<i64>();
            let volume = (1e4..5e7).fake::<f64>();

            Stock {
                id,
                counter: Counter::random(),
                price,
                change,
                change_percent,
                volume,
                turnover: volume * price,
                market_cap: price * shares as f64,
                ttm: (5.0..80.0).fake(),
                five_mins_ranking: (0.0..1000.0).fake(),
                th60_days_ranking: (0.0..1000.0).fake(),
                year_change_percent: (-1.0..1.0).fake(),
                bid: price * (1.0 - (0.0..0.01).fake::<f64>()),
                bid_volume: (100.0..5e4).fake(),
                ask: price * (1.0 + (0.0..0.01).fake::<f64>()),
                ask_volume: (100.0..5e4).fake(),
                open: price * (1.0 + (-0.05..0.05).fake::<f64>()),
                prev_close: price - change,
                high: price * (1.0 + (0.0..0.08).fake::<f64>()),
                low: price * (1.0 - (0.0..0.08).fake::<f64>()),
                turnover_rate: (0.0..0.2).fake(),
                rise_rate: (0.0..1.0).fake(),
                amplitude: (0.0..0.15).fake(),
                pe_status: (5.0..60.0).fake(),
                pb_status: (0.5..12.0).fake(),
                volume_ratio: (0.0..3.0).fake(),
                bid_ask_ratio: (0.0..3.0).fake(),
                latest_pre_close: price * (1.0 + (-0.03..0.03).fake::<f64>()),
                latest_post_close: price * (1.0 + (-0.03..0.03).fake::<f64>()),
                pre_market_cap: price * shares as f64,
                pre_market_percent: (-0.05..0.05).fake(),
                pre_market_change: price * (-0.05..0.05).fake::<f64>(),
                post_market_cap: price * shares as f64,
                post_market_percent: (-0.05..0.05).fake(),
                post_market_change: price * (-0.05..0.05).fake::<f64>(),
                float_cap: price * shares as f64 * (0.3..1.0).fake::<f64>(),
                shares,
                shares_float: (shares as f64 * (0.3..1.0).fake::<f64>()) as i64,
                day_5_ranking: (0.0..1000.0).fake(),
                day_10_ranking: (0.0..1000.0).fake(),
                day_30_ranking: (0.0..1000.0).fake(),
                day_120_ranking: (0.0..1000.0).fake(),
                day_250_ranking: (0.0..1000.0).fake(),
            }
        })
        .collect()
}

struct StockTableDelegate {
    stocks: Vec<Stock>,
    columns: Vec<Column>,
    /// Number of extra "Column N" columns appended after the built-in columns.
    extra_columns_count: usize,
    size: Size,
    loading: bool,
    lazy_load: bool,
    full_loading: bool,
    show_group_headers: bool,
    clicked_row: Option<usize>,
    eof: bool,
    visible_rows: Range<usize>,
    visible_cols: Range<usize>,

    _load_task: Task<()>,
}

impl StockTableDelegate {
    fn new(size: usize) -> Self {
        Self {
            size: Size::default(),
            stocks: random_stocks(size),
            lazy_load: false,
            clicked_row: None,
            columns: vec![
                Column::new("id", "ID")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(true)
                    .min_width(40.)
                    .max_width(100.),
                Column::new("market", "Market")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(true)
                    .min_width(50.),
                Column::new("name", "Name")
                    .width(180.)
                    .fixed(ColumnFixed::Left)
                    .max_width(300.),
                Column::new("symbol", "Symbol")
                    .width(100.)
                    .fixed(ColumnFixed::Left)
                    .sortable(),
                Column::new("price", "Price").sortable().text_right().p_0(),
                Column::new("change", "Chg").sortable().text_right().p_0(),
                Column::new("change_percent", "Chg%")
                    .sortable()
                    .text_right()
                    .p_0(),
                Column::new("volume", "Volume").text_right().p_0(),
                Column::new("turnover", "Turnover").text_right().p_0(),
                Column::new("market_cap", "Market Cap").text_right().p_0(),
                Column::new("ttm", "TTM").text_right().p_0(),
                Column::new("five_mins_ranking", "5m Ranking")
                    .text_right()
                    .p_0(),
                Column::new("th60_days_ranking", "60d Ranking").text_right(),
                Column::new("year_change_percent", "Year Chg%").text_right(),
                Column::new("bid", "Bid").text_right().p_0(),
                Column::new("bid_volume", "Bid Vol").text_right().p_0(),
                Column::new("ask", "Ask").text_right().p_0(),
                Column::new("ask_volume", "Ask Vol").text_right().p_0(),
                Column::new("open", "Open").text_right().p_0(),
                Column::new("prev_close", "Prev Close").text_right().p_0(),
                Column::new("high", "High").text_right().p_0(),
                Column::new("low", "Low").text_right().p_0(),
                Column::new("turnover_rate", "Turnover Rate").text_right(),
                Column::new("rise_rate", "Rise Rate").text_right(),
                Column::new("amplitude", "Amplitude").text_right(),
                Column::new("pe_status", "P/E").text_right(),
                Column::new("pb_status", "P/B").text_right(),
                Column::new("volume_ratio", "Volume Ratio")
                    .text_right()
                    .p_0(),
                Column::new("bid_ask_ratio", "Bid Ask Ratio")
                    .text_right()
                    .p_0(),
                Column::new("latest_pre_close", "Latest Pre Close").text_right(),
                Column::new("latest_post_close", "Latest Post Close").text_right(),
                Column::new("pre_market_cap", "Pre Mkt Cap").text_right(),
                Column::new("pre_market_percent", "Pre Mkt%").text_right(),
                Column::new("pre_market_change", "Pre Mkt Chg").text_right(),
                Column::new("post_market_cap", "Post Mkt Cap").text_right(),
                Column::new("post_market_percent", "Post Mkt%").text_right(),
                Column::new("post_market_change", "Post Mkt Chg").text_right(),
                Column::new("float_cap", "Float Cap").text_right(),
                Column::new("shares", "Shares").text_right(),
                Column::new("shares_float", "Float Shares").text_right(),
                Column::new("day_5_ranking", "5d Ranking").text_right(),
                Column::new("day_10_ranking", "10d Ranking").text_right(),
                Column::new("day_30_ranking", "30d Ranking").text_right(),
                Column::new("day_120_ranking", "120d Ranking").text_right(),
                Column::new("day_250_ranking", "250d Ranking").text_right(),
            ],
            extra_columns_count: 0,
            loading: false,
            full_loading: false,
            show_group_headers: true,
            eof: false,
            visible_cols: Range::default(),
            visible_rows: Range::default(),
            _load_task: Task::ready(()),
        }
    }

    fn set_stocks(&mut self, stocks: Vec<Stock>) {
        self.eof = stocks.len() <= 50;
        self.stocks = stocks;
        self.loading = false;
        self.full_loading = false;
    }

    /// The frame shared by the value cells: it fills the row and follows the
    /// column alignment.
    ///
    /// Columns that carry their own padding (`Column::p_0`) leave it to the cell,
    /// so the frame supplies it for them.
    fn value_cell(&self, col: &Column) -> Div {
        div()
            .h_full()
            .h_flex()
            .items_center()
            .when(col.paddings.is_some(), |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Right, |this| this.justify_end())
    }

    /// A plain number, already formatted by the caller.
    fn render_number(&self, col: &Column, text: String) -> AnyElement {
        self.value_cell(col).child(text).into_any_element()
    }

    /// A percentage, tinted over the whole cell as ticker tables do.
    fn render_percent(&self, col: &Column, val: f64, cx: &mut App) -> AnyElement {
        self.value_cell(col)
            .when_some(change_colors(val, cx), |this, (foreground, background)| {
                this.text_color(foreground).bg(background.alpha(0.05))
            })
            .child(format!("{:+.2}%", val * 100.))
            .into_any_element()
    }

    /// A signed change, colored by its direction.
    fn render_change(&self, col: &Column, val: f64, cx: &mut App) -> AnyElement {
        self.value_cell(col)
            .when_some(change_colors(val, cx), |this, (foreground, _)| {
                this.text_color(foreground)
            })
            .child(format!("{val:+.2}"))
            .into_any_element()
    }
}

impl TableDelegate for StockTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len() + self.extra_columns_count
    }

    fn rows_count(&self, _: &App) -> usize {
        self.stocks.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        if let Some(col) = self.columns.get(col_ix) {
            col.clone()
        } else {
            let n = col_ix - self.columns.len() + 1;
            Column::new(format!("extra_{n}"), format!("Column {n}"))
        }
    }

    fn group_headers(&self, cx: &App) -> Option<Vec<Vec<ColumnGroup>>> {
        if !self.show_group_headers {
            return None;
        }
        // Both rows must span every column, and the lower row subdivides the
        // upper one, so each group lines up with the columns it names.
        let trailing = self.columns_count(cx) - self.columns.len();

        Some(vec![
            vec![
                ColumnGroup {
                    label: "Stock".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Market Data".into(),
                    span: 10,
                },
                ColumnGroup {
                    label: "Quotes".into(),
                    span: 8,
                },
                ColumnGroup {
                    label: "Stats".into(),
                    span: 7,
                },
                ColumnGroup {
                    label: "Extended Hours".into(),
                    span: 8,
                },
                ColumnGroup {
                    label: "Shares & Rankings".into(),
                    span: 8 + trailing,
                },
            ],
            vec![
                ColumnGroup {
                    label: "Identity".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Price & Change".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Turnover".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Momentum".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Order Book".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Session".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Activity".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Valuation".into(),
                    span: 2,
                },
                ColumnGroup {
                    label: "Ratios".into(),
                    span: 2,
                },
                ColumnGroup {
                    label: "Pre & Post Market".into(),
                    span: 8,
                },
                ColumnGroup {
                    label: "Shares".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Rankings".into(),
                    span: 5 + trailing,
                },
            ],
        ])
    }
    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let col = self.column(col_ix, cx);

        div()
            // Same rule as the cells: supply the padding only for the columns
            // that dropped their own, so a header lines up with its column.
            .when(col.paddings.is_some(), |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Center, |this| {
                this.h_flex().w_full().justify_center()
            })
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().w_full().justify_end()
            })
            .child(col.name.clone())
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        menu.menu(
            format!("Selected Row: {}", row_ix),
            Box::new(OpenDetail(row_ix)),
        )
        .separator()
        .menu("Size 48px", Box::new(ChangeSize(Size::Size(px(48.)))))
        .menu("Size Large", Box::new(ChangeSize(Size::Large)))
        .menu("Size Medium", Box::new(ChangeSize(Size::Medium)))
        .menu("Size Small", Box::new(ChangeSize(Size::Small)))
        .menu("Size XSmall", Box::new(ChangeSize(Size::XSmall)))
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        div()
            .id(row_ix)
            .on_click(cx.listener(move |table, ev: &ClickEvent, _window, cx| {
                println!(
                    "You have clicked row with secondary: {}",
                    ev.modifiers().secondary()
                );

                table.delegate_mut().clicked_row = Some(row_ix);
                cx.notify();
            }))
    }

    /// NOTE: Performance metrics
    ///
    /// last render 561 cells total: 232.745µs, avg: 414ns
    /// frame duration: 8.825083ms
    ///
    /// This is means render the full table cells takes 232.745µs. Then 232.745µs / 8.82ms = 2.6% of the frame duration.
    ///
    /// If we improve the td rendering, we can reduce the time to render the full table cells.
    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let stock = self.stocks.get(row_ix).unwrap();
        let Some(col) = self.columns.get(col_ix) else {
            return div().child("--").into_any_element();
        };

        match col.key.as_ref() {
            "id" => self
                .value_cell(&col)
                .text_color(cx.theme().muted_foreground)
                .when(col.align == TextAlign::Center, |this| this.justify_center())
                .child(stock.id.to_string())
                .into_any_element(),
            "market" => self
                .value_cell(&col)
                .map(|this| {
                    if stock.counter.market == "US" {
                        this.text_color(cx.theme().blue)
                    } else {
                        this.text_color(cx.theme().magenta)
                    }
                })
                .child(stock.counter.market.clone())
                .into_any_element(),
            "symbol" => self
                .value_cell(&col)
                .font_medium()
                .child(stock.counter.symbol_code())
                .into_any_element(),
            "name" => self
                .value_cell(&col)
                .child(div().truncate().child(stock.counter.name.clone()))
                .into_any_element(),
            "price" => self
                .value_cell(&col)
                .font_semibold()
                .child(format!("{:.2}", stock.price))
                .into_any_element(),
            "change" => self.render_change(&col, stock.change, cx),
            "change_percent" => self.render_percent(&col, stock.change_percent, cx),
            "volume" => self.render_number(&col, compact(stock.volume)),
            "turnover" => self.render_number(&col, compact(stock.turnover)),
            "market_cap" => self.render_number(&col, compact(stock.market_cap)),
            "ttm" => self.render_number(&col, compact(stock.ttm)),
            "five_mins_ranking" => {
                self.render_number(&col, format!("{:.0}", stock.five_mins_ranking))
            }
            "th60_days_ranking" => {
                self.render_number(&col, format!("{:.0}", stock.th60_days_ranking))
            }
            "year_change_percent" => self.render_percent(&col, stock.year_change_percent, cx),
            "bid" => self.render_number(&col, format!("{:.2}", stock.bid)),
            "bid_volume" => self.render_number(&col, compact(stock.bid_volume)),
            "ask" => self.render_number(&col, format!("{:.2}", stock.ask)),
            "ask_volume" => self.render_number(&col, compact(stock.ask_volume)),
            "open" => self.render_number(&col, format!("{:.2}", stock.open)),
            "prev_close" => self.render_number(&col, format!("{:.2}", stock.prev_close)),
            "high" => self.render_number(&col, format!("{:.2}", stock.high)),
            "low" => self.render_number(&col, format!("{:.2}", stock.low)),
            "turnover_rate" => {
                self.render_number(&col, format!("{:.2}%", stock.turnover_rate * 100.))
            }
            "rise_rate" => self.render_number(&col, format!("{:.2}%", stock.rise_rate * 100.)),
            "amplitude" => self.render_number(&col, format!("{:.2}%", stock.amplitude * 100.)),
            "pe_status" => self.render_number(&col, format!("{:.2}", stock.pe_status)),
            "pb_status" => self.render_number(&col, format!("{:.2}", stock.pb_status)),
            "volume_ratio" => self.render_number(&col, format!("{:.2}", stock.volume_ratio)),
            "bid_ask_ratio" => self.render_number(&col, format!("{:.2}", stock.bid_ask_ratio)),
            "latest_pre_close" => {
                self.render_number(&col, format!("{:.2}", stock.latest_pre_close))
            }
            "latest_post_close" => {
                self.render_number(&col, format!("{:.2}", stock.latest_post_close))
            }
            "pre_market_cap" => self.render_number(&col, compact(stock.pre_market_cap)),
            "pre_market_percent" => self.render_percent(&col, stock.pre_market_percent, cx),
            "pre_market_change" => self.render_change(&col, stock.pre_market_change, cx),
            "post_market_cap" => self.render_number(&col, compact(stock.post_market_cap)),
            "post_market_percent" => self.render_percent(&col, stock.post_market_percent, cx),
            "post_market_change" => self.render_change(&col, stock.post_market_change, cx),
            "float_cap" => self.render_number(&col, compact(stock.float_cap)),
            "shares" => self.render_number(&col, compact(stock.shares as f64)),
            "shares_float" => self.render_number(&col, compact(stock.shares_float as f64)),
            "day_5_ranking" => self.render_number(&col, format!("{:.0}", stock.day_5_ranking)),
            "day_10_ranking" => self.render_number(&col, format!("{:.0}", stock.day_10_ranking)),
            "day_30_ranking" => self.render_number(&col, format!("{:.0}", stock.day_30_ranking)),
            "day_120_ranking" => self.render_number(&col, format!("{:.0}", stock.day_120_ranking)),
            "day_250_ranking" => self.render_number(&col, format!("{:.0}", stock.day_250_ranking)),
            _ => self
                .value_cell(&col)
                .text_color(cx.theme().muted_foreground)
                .child("--")
                .into_any_element(),
        }
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let col = self.columns.remove(col_ix);
        self.columns.insert(to_ix, col);
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.key.as_ref() {
                "id" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.id.cmp(&a.id),
                    _ => a.id.cmp(&b.id),
                }),
                "symbol" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.counter.symbol.cmp(&a.counter.symbol),
                    _ => a.id.cmp(&b.id),
                }),
                "change" | "change_percent" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b
                        .change
                        .partial_cmp(&a.change)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => a.id.cmp(&b.id),
                }),
                _ => {}
            }
        }
    }

    fn loading(&self, _: &App) -> bool {
        self.full_loading
    }

    fn has_more(&self, _: &App) -> bool {
        if !self.lazy_load {
            return false;
        }
        if self.loading {
            return false;
        }

        return !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<TableState<Self>>) {
        if !self.lazy_load {
            return;
        }

        self.loading = true;

        self._load_task = cx.spawn(async move |view, cx| {
            // Simulate network request, delay 1s to load data.
            cx.background_executor().timer(Duration::from_secs(1)).await;

            _ = cx.update(|cx| {
                let _ = view.update(cx, |view, _| {
                    view.delegate_mut().stocks.extend(random_stocks(200));
                    view.delegate_mut().loading = false;
                    view.delegate_mut().eof = view.delegate().stocks.len() >= 6000;
                });
            });
        });
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_rows = visible_range;
    }

    fn visible_columns_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_cols = visible_range;
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _cx: &App) -> String {
        let Some(stock) = self.stocks.get(row_ix) else {
            return String::new();
        };
        let Some(col) = self.columns.get(col_ix) else {
            return String::new();
        };

        match col.key.as_ref() {
            "id" => stock.id.to_string(),
            "market" => stock.counter.market.to_string(),
            "symbol" => stock.counter.symbol_code().to_string(),
            "name" => stock.counter.name.to_string(),
            "price" => format!("{:.3}", stock.price),
            "change" => format!("{:.3}", stock.change),
            "change_percent" => format!("{:.2}%", stock.change_percent * 100.),
            "volume" => format!("{:.3}", stock.volume),
            "turnover" => format!("{:.3}", stock.turnover),
            "market_cap" => format!("{:.3}", stock.market_cap),
            "ttm" => format!("{:.3}", stock.ttm),
            "five_mins_ranking" => format!("{:.3}", stock.five_mins_ranking),
            "th60_days_ranking" => stock.th60_days_ranking.floor().to_string(),
            "year_change_percent" => format!("{:.2}%", stock.year_change_percent * 100.),
            "bid" => format!("{:.3}", stock.bid),
            "bid_volume" => format!("{:.3}", stock.bid_volume),
            "ask" => format!("{:.3}", stock.ask),
            "ask_volume" => format!("{:.3}", stock.ask_volume),
            "open" => format!("{:.3}", stock.open),
            "prev_close" => format!("{:.3}", stock.prev_close),
            "high" => format!("{:.3}", stock.high),
            "low" => format!("{:.3}", stock.low),
            "turnover_rate" => format!("{:.0}", stock.turnover_rate * 100.),
            "rise_rate" => format!("{:.0}", stock.rise_rate * 100.),
            "amplitude" => format!("{:.0}", stock.amplitude * 100.),
            "pe_status" => stock.pe_status.floor().to_string(),
            "pb_status" => stock.pb_status.floor().to_string(),
            "volume_ratio" => format!("{:.3}", stock.volume_ratio),
            "bid_ask_ratio" => format!("{:.3}", stock.bid_ask_ratio),
            "latest_pre_close" => stock.latest_pre_close.floor().to_string(),
            "latest_post_close" => stock.latest_post_close.floor().to_string(),
            "pre_market_cap" => stock.pre_market_cap.floor().to_string(),
            "pre_market_percent" => format!("{:.2}%", stock.pre_market_percent * 100.),
            "pre_market_change" => stock.pre_market_change.floor().to_string(),
            "post_market_cap" => stock.post_market_cap.floor().to_string(),
            "post_market_percent" => format!("{:.2}%", stock.post_market_percent * 100.),
            "post_market_change" => stock.post_market_change.floor().to_string(),
            "float_cap" => stock.float_cap.floor().to_string(),
            "shares" => stock.shares.to_string(),
            "shares_float" => stock.shares_float.to_string(),
            "day_5_ranking" => stock.day_5_ranking.floor().to_string(),
            "day_10_ranking" => stock.day_10_ranking.floor().to_string(),
            "day_30_ranking" => stock.day_30_ranking.floor().to_string(),
            "day_120_ranking" => stock.day_120_ranking.floor().to_string(),
            "day_250_ranking" => stock.day_250_ranking.floor().to_string(),
            _ => String::new(),
        }
    }
}

pub struct DataTableStory {
    table: Entity<TableState<StockTableDelegate>>,
    stripe: bool,
    refresh_data: bool,
    size: Size,

    _subscriptions: Vec<Subscription>,
    _load_task: Task<()>,
    _load_rows_task: Task<()>,
}

impl super::Story for DataTableStory {
    fn title() -> &'static str {
        "DataTable"
    }

    fn description() -> &'static str {
        "A complex data table with selection, sorting, column moving, and loading more."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn closable() -> bool {
        false
    }
}

impl Focusable for DataTableStory {
    fn focus_handle(&self, cx: &gpui_kit::App) -> gpui_kit::FocusHandle {
        self.table.focus_handle(cx)
    }
}

impl DataTableStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = StockTableDelegate::new(5000);
        let table = cx.new(|cx| TableState::new(delegate, window, cx));

        let _subscriptions = vec![cx.subscribe_in(&table, window, Self::on_table_event)];

        let _load_task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(time::Duration::from_millis(33))
                    .await;

                this.update(cx, |this, cx| {
                    if !this.refresh_data {
                        return;
                    }

                    this.table.update(cx, |table, _| {
                        // Only walk the head of the table. It paints a few dozen
                        // rows at a time, so ticking every row of the million-row
                        // example would stall the frame for nothing on screen.
                        const MAX_REFRESH_ROWS: usize = 2_000;
                        table
                            .delegate_mut()
                            .stocks
                            .iter_mut()
                            .take(MAX_REFRESH_ROWS)
                            .enumerate()
                            .for_each(|(i, stock)| {
                                let n = (3..10).fake::<usize>();
                                // update 30% of the stocks
                                if i % n == 0 {
                                    stock.random_update();
                                }
                            });
                    });
                    cx.notify();
                })
                .ok();
            }
        });

        Self {
            table,
            stripe: false,
            refresh_data: false,
            size: Size::default(),
            _subscriptions,
            _load_task,
            _load_rows_task: Task::ready(()),
        }
    }

    fn toggle_loop_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.loop_selection = *checked;
            cx.notify();
        });
    }

    fn toggle_col_resize(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_resizable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_order(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_movable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_sort(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.sortable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_fixed(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_fixed = *checked;
            cx.notify();
        });
    }

    fn toggle_col_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_selectable = *checked;
            cx.notify();
        });
    }

    fn toggle_row_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.row_selectable = *checked;
            cx.notify();
        });
    }

    fn toggle_cell_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.cell_selectable = *checked;
            cx.notify();
        });
    }

    fn toggle_row_header(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.row_header = *checked;
            cx.notify();
        });
    }

    fn toggle_stripe(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.stripe = *checked;
        cx.notify();
    }

    fn on_change_size(&mut self, a: &ChangeSize, _: &mut Window, cx: &mut Context<Self>) {
        self.size = a.0;
        cx.notify();
    }

    fn toggle_refresh_data(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh_data = *checked;
        cx.notify();
    }

    fn toggle_group_headers(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().show_group_headers = *checked;
            table.refresh_header_layout(cx);
        });
    }

    fn on_table_event(
        &mut self,
        _: &Entity<TableState<StockTableDelegate>>,
        event: &TableEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            TableEvent::ColumnWidthsChanged(col_widths) => {
                println!("Column widths changed: {:?}", col_widths)
            }
            TableEvent::SelectColumn(ix) => println!("Select col: {}", ix),
            TableEvent::SelectCell(row_ix, col_ix) => {
                println!("Select cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::DoubleClickedCell(row_ix, col_ix) => {
                println!("Double clicked cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::DoubleClickedRow(ix) => println!("Double clicked row: {}", ix),
            TableEvent::SelectRow(ix) => println!("Select row: {}", ix),
            TableEvent::MoveColumn(origin_idx, target_idx) => {
                println!("Move col index: {} -> {}", origin_idx, target_idx);
            }
            TableEvent::RightClickedRow(ix) => println!("Right clicked row: {:?}", ix),
            TableEvent::RightClickedCell(row_ix, col_ix) => {
                println!("Right clicked cell: row={}, col={}", row_ix, col_ix)
            }
            TableEvent::ClearSelection => {
                println!("Selection cleared");
            }
        }
    }

    fn dump_csv(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(download_dir) = dirs::download_dir() else {
            eprintln!("Failed to get download directory");
            return;
        };

        let receiver = cx.prompt_for_new_path(&download_dir, Some("export.csv"));
        let table = self.table.clone();
        cx.spawn_in(window, async move |_, window| {
            let Some(path) = receiver.await.ok().into_iter().flatten().flatten().next() else {
                println!("CSV export cancelled by user");
                return;
            };

            match Self::write_csv(table, path, window).await {
                Ok(()) => println!("CSV exported successfully"),
                Err(error) => eprintln!("Failed to export CSV: {error}"),
            }
        })
        .detach();
    }

    async fn write_csv(
        table: Entity<TableState<StockTableDelegate>>,
        path: PathBuf,
        window: &mut gpui_kit::AsyncWindowContext,
    ) -> anyhow::Result<()> {
        let (headers, rows_count) = table.update_in(window, |table, _, cx| {
            (table.headers(cx), table.delegate().rows_count(cx))
        })?;

        let (sender, receiver) = async_channel::bounded::<Vec<Vec<String>>>(1);
        let writer_task = window.background_spawn(async move {
            let mut writer = csv::Writer::from_path(path)?;
            writer.write_record(headers)?;

            while let Ok(rows) = receiver.recv().await {
                for row in rows {
                    writer.write_record(row)?;
                }
            }

            writer.flush()?;
            Ok::<(), anyhow::Error>(())
        });

        for start in (0..rows_count).step_by(CSV_EXPORT_BATCH_ROWS) {
            let end = start.saturating_add(CSV_EXPORT_BATCH_ROWS).min(rows_count);
            let (_, rows) =
                table.update_in(window, |table, _, cx| table.dump_range(start..end, cx))?;

            if sender.send(rows).await.is_err() {
                break;
            }
        }

        drop(sender);
        writer_task.await?;
        Ok(())
    }
}

impl Render for DataTableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui_kit::IntoElement {
        let table = &self.table.read(cx);
        let delegate = table.delegate();
        let rows_count = delegate.rows_count(cx);
        let columns_count = delegate.columns_count(cx);
        let size = self.size;
        let loop_selection = table.loop_selection;
        let col_resizable = table.col_resizable;
        let col_movable = table.col_movable;
        let sortable = table.sortable;
        let col_selectable = table.col_selectable;
        let row_selectable = table.row_selectable;
        let cell_selectable = table.cell_selectable;
        let row_header = table.row_header;
        let col_fixed = table.col_fixed;
        let striped = self.stripe;
        let full_loading = delegate.full_loading;
        let lazy_load = delegate.lazy_load;
        let refresh_data = self.refresh_data;
        let show_group_headers = delegate.show_group_headers;

        v_flex()
            .on_action(cx.listener(Self::on_change_size))
            .on_action(cx.listener(|this, action: &ChangeRows, _, cx| {
                if action.0 == this.table.read(cx).delegate().stocks.len() {
                    return;
                }

                // Building the largest example allocates a few hundred MB, which
                // drops frames if it runs between two paints. Show the loading
                // state, build off-thread, and swap the rows in when they land.
                let size = action.0;
                this.table.update(cx, |table, cx| {
                    table.delegate_mut().full_loading = true;
                    cx.notify();
                });
                this._load_rows_task = cx.spawn(async move |this, cx| {
                    let stocks = cx
                        .background_spawn(async move { regenerate_stocks(size) })
                        .await;

                    this.update(cx, |this, cx| {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().set_stocks(stocks);
                            table.refresh(cx);
                        });
                        cx.notify();
                    })
                    .ok();
                });
            }))
            .on_action(cx.listener(|this, action: &ChangeExtraColumns, _, cx| {
                this.table.update(cx, |table, cx| {
                    table.delegate_mut().extra_columns_count = action.0;
                    table.refresh(cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ClearTableSelection, _, cx| {
                this.table.update(cx, |table, cx| table.clear_selection(cx));
            }))
            .on_action(cx.listener(|this, action: &GoToTablePosition, _, cx| {
                this.table.update(cx, |table, cx| match action {
                    GoToTablePosition::Top => table.scroll_to_row(0, cx),
                    GoToTablePosition::Bottom => {
                        table.scroll_to_row(table.delegate().rows_count(cx) - 1, cx)
                    }
                    GoToTablePosition::Cell53 => table.set_selected_cell(5, 3, cx),
                    GoToTablePosition::Cell107 => table.set_selected_cell(10, 7, cx),
                });
            }))
            .on_action(cx.listener(
                |this, action: &ToggleTableOption, window, cx| match action {
                    ToggleTableOption::LoopSelection => {
                        let checked = !this.table.read(cx).loop_selection;
                        this.toggle_loop_selection(&checked, window, cx);
                    }
                    ToggleTableOption::ColumnResize => {
                        let checked = !this.table.read(cx).col_resizable;
                        this.toggle_col_resize(&checked, window, cx);
                    }
                    ToggleTableOption::ColumnOrder => {
                        let checked = !this.table.read(cx).col_movable;
                        this.toggle_col_order(&checked, window, cx);
                    }
                    ToggleTableOption::Sortable => {
                        let checked = !this.table.read(cx).sortable;
                        this.toggle_col_sort(&checked, window, cx);
                    }
                    ToggleTableOption::ColumnSelection => {
                        let checked = !this.table.read(cx).col_selectable;
                        this.toggle_col_selection(&checked, window, cx);
                    }
                    ToggleTableOption::RowSelection => {
                        let checked = !this.table.read(cx).row_selectable;
                        this.toggle_row_selection(&checked, window, cx);
                    }
                    ToggleTableOption::CellSelection => {
                        let checked = !this.table.read(cx).cell_selectable;
                        this.toggle_cell_selection(&checked, window, cx);
                    }
                    ToggleTableOption::RowHeader => {
                        let checked = !this.table.read(cx).row_header;
                        this.toggle_row_header(&checked, window, cx);
                    }
                    ToggleTableOption::FixedColumn => {
                        let checked = !this.table.read(cx).col_fixed;
                        this.toggle_col_fixed(&checked, window, cx);
                    }
                    ToggleTableOption::Striped => {
                        this.toggle_stripe(&!this.stripe, window, cx);
                    }
                    ToggleTableOption::Loading => {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().full_loading = !table.delegate().full_loading;
                            cx.notify();
                        });
                    }
                    ToggleTableOption::LazyLoad => {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().lazy_load = !table.delegate().lazy_load;
                            cx.notify();
                        });
                    }
                    ToggleTableOption::RefreshData => {
                        this.toggle_refresh_data(&!this.refresh_data, window, cx);
                    }
                    ToggleTableOption::GroupHeaders => {
                        let checked = !this.table.read(cx).delegate().show_group_headers;
                        this.toggle_group_headers(&checked, window, cx);
                    }
                },
            ))
            .size_full()
            .text_sm()
            .gap_4()
            .child(
                story_toolbar_group()
                    .dropdown_child(
                        Button::new("size").label(format!("Size: {:?}", self.size)),
                        move |menu, _, _| {
                            menu.menu_with_check(
                                "48px",
                                size == Size::Size(px(48.)),
                                Box::new(ChangeSize(Size::Size(px(48.)))),
                            )
                            .menu_with_check(
                                "Large",
                                size == Size::Large,
                                Box::new(ChangeSize(Size::Large)),
                            )
                            .menu_with_check(
                                "Medium",
                                size == Size::Medium,
                                Box::new(ChangeSize(Size::Medium)),
                            )
                            .menu_with_check(
                                "Small",
                                size == Size::Small,
                                Box::new(ChangeSize(Size::Small)),
                            )
                            .menu_with_check(
                                "XSmall",
                                size == Size::XSmall,
                                Box::new(ChangeSize(Size::XSmall)),
                            )
                        },
                    )
                    .dropdown_child(
                        Button::new("rows").label(format!("Rows: {rows_count}")),
                        move |menu, _, _| {
                            menu.menu_with_check(
                                "100",
                                rows_count == 100,
                                Box::new(ChangeRows(100)),
                            )
                            .menu_with_check("500", rows_count == 500, Box::new(ChangeRows(500)))
                            .menu_with_check(
                                "5,000",
                                rows_count == 5_000,
                                Box::new(ChangeRows(5_000)),
                            )
                            .menu_with_check(
                                "10,000",
                                rows_count == 10_000,
                                Box::new(ChangeRows(10_000)),
                            )
                            .menu_with_check(
                                "1,000,000",
                                rows_count == 1_000_000,
                                Box::new(ChangeRows(1_000_000)),
                            )
                        },
                    )
                    .dropdown_child(
                        Button::new("extra-columns")
                            .label(format!("Extra Columns: {}", delegate.extra_columns_count)),
                        {
                            let extra_columns_count = delegate.extra_columns_count;
                            move |menu, _, _| {
                                menu.menu_with_check(
                                    "None",
                                    extra_columns_count == 0,
                                    Box::new(ChangeExtraColumns(0)),
                                )
                                .menu_with_check(
                                    "4",
                                    extra_columns_count == 4,
                                    Box::new(ChangeExtraColumns(4)),
                                )
                                .menu_with_check(
                                    "8",
                                    extra_columns_count == 8,
                                    Box::new(ChangeExtraColumns(8)),
                                )
                                .menu_with_check(
                                    "16",
                                    extra_columns_count == 16,
                                    Box::new(ChangeExtraColumns(16)),
                                )
                                .menu_with_check(
                                    "32",
                                    extra_columns_count == 32,
                                    Box::new(ChangeExtraColumns(32)),
                                )
                            }
                        },
                    )
                    .dropdown_child(Button::new("table-options").label("Options"), {
                        move |menu, _, _| {
                            menu.menu_with_check(
                                "Loop Selection",
                                loop_selection,
                                Box::new(ToggleTableOption::LoopSelection),
                            )
                            .menu_with_check(
                                "Column Resize",
                                col_resizable,
                                Box::new(ToggleTableOption::ColumnResize),
                            )
                            .menu_with_check(
                                "Column Order",
                                col_movable,
                                Box::new(ToggleTableOption::ColumnOrder),
                            )
                            .menu_with_check(
                                "Sortable",
                                sortable,
                                Box::new(ToggleTableOption::Sortable),
                            )
                            .menu_with_check(
                                "Column Selectable",
                                col_selectable,
                                Box::new(ToggleTableOption::ColumnSelection),
                            )
                            .menu_with_check(
                                "Row Selectable",
                                row_selectable,
                                Box::new(ToggleTableOption::RowSelection),
                            )
                            .menu_with_check(
                                "Cell Selectable",
                                cell_selectable,
                                Box::new(ToggleTableOption::CellSelection),
                            )
                            .menu_with_check(
                                "Row Header",
                                row_header,
                                Box::new(ToggleTableOption::RowHeader),
                            )
                            .menu_with_check(
                                "Fixed Column",
                                col_fixed,
                                Box::new(ToggleTableOption::FixedColumn),
                            )
                            .separator()
                            .menu_with_check(
                                "Striped Rows",
                                striped,
                                Box::new(ToggleTableOption::Striped),
                            )
                            .menu_with_check(
                                "Loading",
                                full_loading,
                                Box::new(ToggleTableOption::Loading),
                            )
                            .menu_with_check(
                                "Lazy Load",
                                lazy_load,
                                Box::new(ToggleTableOption::LazyLoad),
                            )
                            .menu_with_check(
                                "Refresh Data",
                                refresh_data,
                                Box::new(ToggleTableOption::RefreshData),
                            )
                            .menu_with_check(
                                "Group Headers",
                                show_group_headers,
                                Box::new(ToggleTableOption::GroupHeaders),
                            )
                            .separator()
                            .menu("Clear Selection", Box::new(ClearTableSelection))
                        }
                    })
                    .dropdown_child(Button::new("go-to").label("Go To"), |menu, _, _| {
                        menu.menu("Top", Box::new(GoToTablePosition::Top))
                            .menu("Bottom", Box::new(GoToTablePosition::Bottom))
                            .separator()
                            .menu("Cell 5:3", Box::new(GoToTablePosition::Cell53))
                            .menu("Cell 10:7", Box::new(GoToTablePosition::Cell107))
                    })
                    .child(
                        Button::new("dump-csv")
                            .label("Export CSV")
                            .on_click(cx.listener(Self::dump_csv)),
                    ),
            )
            .child(
                v_flex()
                    .min_h_0()
                    .flex_1()
                    .child(
                        DataTable::new(&self.table)
                            .with_size(self.size)
                            .stripe(self.stripe),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .min_h_9()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .bg(cx.theme().muted.opacity(0.35))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Total · {} rows · {} columns",
                                rows_count, columns_count
                            ))
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_end()
                                    .gap_2()
                                    .when(delegate.loading, |this| {
                                        this.child(Spinner::new().xsmall())
                                    })
                                    .child(format!(
                                        "Current · rows {:?} · columns {:?}",
                                        delegate.visible_rows, delegate.visible_cols
                                    ))
                                    .when_some(table.selected_cell(), |this, (row, col)| {
                                        this.child(format!("· cell {}:{}", row, col))
                                    })
                                    .when(delegate.eof, |this| this.child("· complete")),
                            ),
                    ),
            )
    }
}
