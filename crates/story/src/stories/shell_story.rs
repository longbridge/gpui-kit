//! One window, two languages, one ticking quote board.
//!
//! The left panel is ordinary Rust built from `gpui-component`. The right panel
//! is a `gpui-shell` script view whose JavaScript lives in
//! `crates/story/js/quotes/` and is read from disk when the story opens. A
//! separate `crates/story/js/motion/` view demonstrates native motion without
//! becoming part of the quote-board performance comparison.
//! Neither half owns the data: a single `Entity<Market>` does, and the script
//! reaches it through the **`market` host module** this story registers before the
//! script runtime starts.
//!
//! ```text
//!   Rust panel ──┐                                  ┌── main.js
//!   (rows drawn  │                                  │   (rows drawn
//!    with        ▼                                  ▼    with div/text)
//!    Label)   Entity<Market>  ◀── import "market" ───┐
//!                    │            quotes / ticks / watch
//!                    │ cx.notify()
//!                    ▼
//!              cx.observe(...) ──▶ re-renders both halves
//! ```
//!
//! # Why a quote board
//!
//! Because it is the load that decides whether a scripting layer is viable. A
//! feed arrives on its own, several times a second, while the window is already
//! repainting for reasons of its own — and the question the runtime has to
//! answer is which of those two frequencies the script pays for.
//!
//! The board ticks every 50 ms out of the box, and the counters under it show
//! both numbers as live rates. Switch the feed to **Repaint only** and watch
//! them come apart: frames keep climbing, script renders drop to zero, because
//! nothing the script reads has changed and the description it already
//! published is simply drawn again.
//!
//! # Editing the script
//!
//! `js/quotes/` carries a generated `gpui-kit.d.ts` and a hand-written
//! `market.d.ts`, so an editor knows what `import ... from "gpui-kit"` holds and
//! what the `market` module answers. A misspelled style method, a colour token
//! that does not exist, or a host module nobody registered is an error in the
//! editor rather than an exception on the next tick. Regenerate the first after
//! a runtime change:
//!
//! ```bash
//! cargo run -p gpui-shell -- types crates/story/js/quotes
//! ```
//!
//! Nothing but plain data crosses. `quotes()` returns an array of records;
//! `watch(symbol)` takes a string and answers a boolean, or fails with a
//! sentence the script sees as an exception. The script cannot hand Rust a
//! callback and Rust cannot hand the script a handle. Theme tokens need no
//! story-specific bridge: every render receives a live shell context and reads
//! them through `cx.theme()`.

use std::{path::PathBuf, rc::Rc, time::Duration};

use gpui_kit::base::Button as BaseButton;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _,
    button::Button,
    h_flex,
    label::Label,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement as _,
    IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative, rems,
};
use gpui_shell::Watcher;
use gpui_shell::{
    RuntimeMetrics, ScriptView, ShellRoot, ShellRuntime,
    host_modules::{HostError, HostModule, HostObject, HostValue},
};

use crate::section;

/// One instrument on the board. The only data either half of the window reads.
#[derive(Clone)]
struct Quote {
    symbol: SharedString,
    name: SharedString,
    /// The session's opening price, so a change is a fact rather than a memory
    /// of the previous frame.
    open: f32,
    last: f32,
    volume: u64,
    watched: bool,
}

impl Quote {
    fn change(&self) -> f32 {
        self.last - self.open
    }

    fn change_percent(&self) -> f32 {
        if self.open == 0. {
            0.
        } else {
            self.change() / self.open * 100.
        }
    }

    /// Up, down, or flat. Both halves colour from this rather than each deciding
    /// what "unchanged" means, because the two panels sitting side by side is
    /// the whole point of the story.
    fn direction(&self) -> i32 {
        match self.change() {
            change if change > 0.0005 => 1,
            change if change < -0.0005 => -1,
            _ => 0,
        }
    }
}

/// The shared state, owned by GPUI and reachable from both languages.
///
/// It is an `Entity` rather than a field on the story so the host module can
/// hold it: a host function is a plain closure with no access to the story's
/// `&mut self`, and an entity handle is the one way to reach host state from
/// inside a script call and still notify observers afterwards.
pub struct Market {
    quotes: Vec<Quote>,
    /// How many feed ticks have landed. The script paints it, which is what
    /// makes "the script did not run" visible rather than merely asserted.
    ticks: u64,
    /// Deterministic, so the board moves the same way on every run and a test
    /// can assert on it. A real feed is not random either — it is just not ours.
    seed: u64,
}

/// The board. A watchlist-sized twenty rows, which is around three hundred
/// description nodes once the cells and the wrappers are counted — a real
/// description to rebuild, and still a panel a reader can take in at a glance.
///
/// There is no virtualization here yet, so a thousand-row board would be an
/// honest measurement of something this runtime does not claim to do well. A
/// watchlist is what it does claim.
///
/// US names first and only a handful of HK ones, because the board is read left
/// to right and top to bottom by people who will recognize the first rows
/// fastest. The mix is there at all so the symbol column has two shapes in it —
/// a ticker and a numeric code — which is where a fixed-width column earns its
/// keep.
const BOARD: [(&str, &str, f32); 10] = [
    ("AAPL.US", "Apple", 214.29),
    ("NVDA.US", "NVIDIA", 118.11),
    ("MSFT.US", "Microsoft", 421.53),
    ("TSLA.US", "Tesla", 249.83),
    ("AMZN.US", "Amazon", 186.34),
    ("GOOGL.US", "Alphabet", 165.27),
    ("META.US", "Meta", 502.18),
    ("700.HK", "Tencent", 372.40),
    ("9988.HK", "Alibaba", 78.15),
    ("0005.HK", "HSBC", 62.05),
];

impl Market {
    fn open() -> Self {
        Self {
            quotes: BOARD
                .into_iter()
                .map(|(symbol, name, open)| Quote {
                    symbol: symbol.into(),
                    name: name.into(),
                    open,
                    last: open,
                    volume: 0,
                    watched: false,
                })
                .collect(),
            ticks: 0,
            seed: 0x2545_f491_4f6c_dd1d,
        }
    }

    /// One tick of the feed: every price moves a little, every volume grows.
    ///
    /// This is deliberately a *whole-board* update. A feed that moved one row
    /// would let a future subtree memoization hide the cost this story exists to
    /// show.
    fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        for index in 0..self.quotes.len() {
            let drift = self.next_signed();
            let traded = self.next_unsigned() % 4_000;
            let quote = &mut self.quotes[index];
            // Proportional, so a 400-dollar name and a 17-dollar one move by
            // amounts that look alike on screen.
            quote.last = (quote.last * (1. + drift * 0.0012)).max(0.01);
            quote.volume = quote.volume.wrapping_add(traded);
        }
    }

    /// xorshift64. A dependency-free generator is worth more here than a good
    /// one: the board only has to move plausibly, and it has to move the same
    /// way twice.
    fn next_unsigned(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    /// Roughly -1.0 to 1.0.
    fn next_signed(&mut self) -> f32 {
        (self.next_unsigned() % 2_001) as f32 / 1_000. - 1.
    }

    fn watched_count(&self) -> usize {
        self.quotes.iter().filter(|quote| quote.watched).count()
    }

    /// Sets one row, for the Rust button, which already knows the value it
    /// wants.
    fn set_watched(&mut self, symbol: &str, watched: bool) {
        if let Some(quote) = self.quotes.iter_mut().find(|quote| quote.symbol == symbol) {
            quote.watched = watched;
        }
    }

    /// Flips one row, for the script, which asks by symbol only.
    ///
    /// An unknown symbol is the script's mistake, so it gets a sentence naming
    /// what does exist rather than a silent no-op.
    fn watch(&mut self, symbol: &str) -> Result<bool, HostError> {
        match self
            .quotes
            .iter_mut()
            .find(|quote| quote.symbol.as_ref() == symbol)
        {
            Some(quote) => {
                quote.watched = !quote.watched;
                Ok(quote.watched)
            }
            None => {
                let known = self
                    .quotes
                    .iter()
                    .map(|quote| quote.symbol.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(HostError::new(format!(
                    "no quote for `{symbol}`; the board holds {known}"
                )))
            }
        }
    }

    /// Returns how many rows actually moved, so the script can report a no-op as
    /// a no-op.
    fn watch_all(&mut self, watched: bool) -> usize {
        let mut changed = 0;
        for quote in &mut self.quotes {
            if quote.watched != watched {
                quote.watched = watched;
                changed += 1;
            }
        }
        changed
    }

    /// The board as it crosses the boundary: an array of records.
    ///
    /// The numbers are formatted here rather than in the script, so both halves
    /// round the same way. A price that reads 372.40 on the left and 372.4 on
    /// the right would make the comparison about formatting instead of about
    /// rendering.
    /// The board as plain owned data, for work that runs off the main thread.
    ///
    /// An asynchronous host function's future cannot reach the `App`, so
    /// whatever it needs is copied out while the synchronous half still can.
    /// This is that copy, and it is deliberately the smallest one that answers
    /// the question rather than the whole board.
    fn movers(&self) -> Vec<(String, f32)> {
        self.quotes
            .iter()
            .map(|quote| (quote.symbol.to_string(), quote.change_percent()))
            .collect()
    }

    fn to_host_value(&self) -> HostValue {
        HostValue::Array(
            self.quotes
                .iter()
                .map(|quote| {
                    HostValue::from(
                        HostObject::new()
                            .field("symbol", quote.symbol.to_string())
                            .field("name", quote.name.to_string())
                            .field("last", format!("{:.2}", quote.last))
                            .field("change", format!("{:+.2}", quote.change()))
                            .field("percent", format!("{:+.2}%", quote.change_percent()))
                            .field("volume", thousands(quote.volume))
                            .field("direction", quote.direction())
                            .field("watched", quote.watched),
                    )
                })
                .collect(),
        )
    }
}

/// `1234567` as `1,234,567`. Both halves call it, for the same reason the prices
/// are formatted in Rust.
fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What is driving the script view right now.
///
/// Two feeds rather than one, because they separate the two frequencies this
/// runtime keeps apart. A **quotes** feed moves prices the script reads, so the
/// script re-renders; a **repaint** feed only tells GPUI the view needs drawing
/// again, which is what a hover, a scroll, a cursor blink or an animation does.
///
/// Run the second one and watch the readout: frames climb, script renders stay
/// at zero. That is the architecture, live, rather than in a test.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Feed {
    Idle,
    /// Ticks the board, which invalidates the script view. The number is the
    /// interval in milliseconds — a feed is described by how often it arrives,
    /// not by a frequency nobody quotes.
    Quotes(u64),
    /// Notifies the script view without changing anything it reads.
    Repaint(u64),
}

impl Feed {
    fn interval(self) -> Option<Duration> {
        match self {
            Feed::Idle => None,
            Feed::Quotes(ms) | Feed::Repaint(ms) => Some(Duration::from_millis(ms.max(1))),
        }
    }

    fn detail(self) -> String {
        match self {
            Feed::Idle => "nothing is driving the board".to_owned(),
            Feed::Quotes(ms) => format!("every price moves every {ms} ms"),
            Feed::Repaint(ms) => format!("the view is redrawn every {ms} ms"),
        }
    }

    fn caption(self) -> String {
        match self {
            Feed::Idle => "Off".to_owned(),
            Feed::Quotes(ms) => format!("Quotes · {ms} ms"),
            Feed::Repaint(ms) => format!("Repaint only · {ms} ms"),
        }
    }
}

/// The board as one half last saw it.
///
/// Pausing works differently on the two sides, and the difference is worth
/// seeing rather than hiding. The script half pauses by simply not being told
/// its description is stale: it keeps drawing the snapshot it already published,
/// which is a thing the runtime does for free. The Rust half has no snapshot —
/// its render reads the entity every time — so pausing it means keeping a copy
/// of what it read last.
///
/// That asymmetry is the argument. One side holds still because nobody
/// invalidated it; the other holds still because somebody kept a copy.
#[derive(Clone)]
struct Frozen {
    quotes: Vec<Quote>,
    watched: usize,
    ticks: u64,
}

impl Frozen {
    fn capture(market: &Market) -> Self {
        Self {
            quotes: market.quotes.clone(),
            watched: market.watched_count(),
            ticks: market.ticks,
        }
    }
}

/// The feed the story opens with.
///
/// Running rather than idle on purpose: a board that has to be switched on
/// before it does anything is a board most readers will look at once, in its
/// resting state, and conclude nothing from.
const OPENING_FEED: Feed = Feed::Quotes(50);

/// The choices offered, in the order they answer the question.
const FEEDS: [(&str, Feed); 4] = [
    ("feed-off", Feed::Idle),
    ("feed-quotes-50", Feed::Quotes(50)),
    ("feed-quotes-16", Feed::Quotes(16)),
    ("feed-repaint-16", Feed::Repaint(16)),
];

/// How often the readout re-reads the counters. One second, because the numbers
/// it shows are rates and a rate over a shorter window is mostly noise.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// The entry file the script application directory must contain.
#[cfg(test)]
const ENTRY: &str = "main.js";

/// Where the script lives.
///
/// Resolved against the crate rather than the process working directory, so
/// `cargo run` finds it from anywhere — and so editing the file is enough to
/// change the panel, with no rebuild.
fn script_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js/quotes")
}

/// The independent native-motion laboratory. Keeping this separate from the
/// quote app protects the side-by-side Rust-versus-JavaScript list benchmark.
fn motion_script_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js/motion")
}

/// Grants the script the market module it may reach, and nothing else.
///
/// This is the whole extension surface: a script cannot load native code, so
/// what the host registers here is exactly what it can call (design doc §17.6).
/// Registering an empty set — the default — would leave `import … from
/// "market"` failing with a message saying this host granted none.
fn install_host_modules(market: &Entity<Market>) {
    gpui_shell::export_module(market_module(market))
        .expect("`market` is free, and MARKET_TYPES describes what is registered");
}

fn market_module(market: &Entity<Market>) -> HostModule {
    let read = market.clone();
    let ticks = market.clone();
    let flip = market.clone();
    let bulk = market.clone();
    let summary = market.clone();

    HostModule::new("market")
        .declarations(MARKET_TYPES)
        .function("quotes", move |_| {
            with_app(|cx| read.read(cx).to_host_value())
        })
        // Read separately from `quotes()` so the script can paint how many ticks
        // it has actually seen. When the feed is only asking for repaints this
        // number stops moving on screen, which is the counters' claim made
        // visible in the panel itself.
        .function("ticks", move |_| {
            with_app(|cx| HostValue::from(ticks.read(cx).ticks as f64))
        })
        .function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // The notification is what keeps the two halves in step:
                    // the story observes this entity and re-renders itself and
                    // the script view together. It is delivered after this call
                    // unwinds, so it cannot re-enter the script engine.
                    cx.notify();
                    Ok(HostValue::from(watched))
                })
            })?
        })
        // The one asynchronous function on this module, and the reason it is
        // here: a synchronous `function` would hold the thread that renders for
        // as long as it ran. The feed keeps moving while this is in flight,
        // which is the claim made visible rather than asserted.
        .async_function("summary", move |_| {
            // Synchronous half — on the main thread, so it may read the entity
            // and take the executor the future will need.
            let (movers, executor) =
                with_app(|cx| (summary.read(cx).movers(), cx.background_executor().clone()))?;

            // Asynchronous half — on the background executor, with no `App` in
            // reach. The delay stands in for the slow thing a real one would
            // do; the work after it is ordinary Rust on owned data.
            Ok(async move {
                executor.timer(Duration::from_millis(900)).await;
                Ok(summarise(movers))
            })
        })
        .function("watch_all", move |arguments| {
            let watched = arguments.boolean(0)?;
            with_app(|cx| {
                bulk.update(cx, |market, cx| {
                    let changed = market.watch_all(watched);
                    if changed > 0 {
                        cx.notify();
                    }
                    HostValue::from(changed as f64)
                })
            })
        })
}

/// The TypeScript face of the `market` module.
///
/// Beside the registration rather than in a `.d.ts` next to the script: a
/// `.d.ts` would be a second file, in a second language, with nothing holding
/// it to what this function registers. `export_module` checks this against the
/// registry, so renaming a function on one side fails at start-up rather than
/// completing in an editor and throwing at the call site.
const MARKET_TYPES: &str = r#"
/** One row of the board, as it crosses the boundary. */
export interface Quote {
  symbol: string;
  name: string;
  /** Already formatted by Rust, so both halves round the same way. */
  last: string;
  change: string;
  percent: string;
  volume: string;
  /** 1 up, -1 down, 0 unchanged. */
  direction: number;
  watched: boolean;
}

/** Every row on the board. */
export function quotes(): Quote[];
/** How many feed ticks have landed. */
export function ticks(): number;
/** Flips one row's watched flag and answers the new value. */
export function watch(symbol: string): boolean;
/** Sets every row, and answers how many actually moved. */
export function watch_all(watched: boolean): number;

/** A slow read of the board, computed off the main thread. */
export interface Summary {
  leader: string;
  leader_percent: string;
  laggard: string;
  laggard_percent: string;
  average_percent: string;
}

/**
 * The session's movers.
 *
 * Deliberately slow, and deliberately a promise: the board keeps ticking while
 * this is in flight, which is what a synchronous host function could not do.
 */
export function summary(): Promise<Summary>;
"#;

/// Turns the copied movers into the record the script paints.
///
/// A free function on owned data: it runs on the background executor, where
/// there is no `App`, no window and no script engine.
fn summarise(movers: Vec<(String, f32)>) -> HostValue {
    let best = movers
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .cloned()
        .unwrap_or_default();
    let worst = movers
        .iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .cloned()
        .unwrap_or_default();
    let average = if movers.is_empty() {
        0.
    } else {
        movers.iter().map(|(_, percent)| percent).sum::<f32>() / movers.len() as f32
    };

    HostObject::new()
        .field("leader", best.0)
        .field("leader_percent", format!("{:+.2}%", best.1))
        .field("laggard", worst.0)
        .field("laggard_percent", format!("{:+.2}%", worst.1))
        .field("average_percent", format!("{average:+.2}%"))
        .into()
}

/// Reaches the ambient `App` from inside a host call.
///
/// A host function receives arguments and nothing else; the host context it
/// runs in comes from the shell's call scope, which is live for exactly as long
/// as the script call that is on the stack. Outside one there is no honest
/// answer, so this says so rather than reaching for a stale pointer.
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, HostError> {
    gpui_shell::with_current_app(read).ok_or_else(|| {
        HostError::new("the board is only reachable while a script call is in progress")
    })
}

pub struct ShellStory {
    focus_handle: FocusHandle,
    market: Entity<Market>,
    /// Held for as long as the script view is mounted: the view renders through
    /// it, and dropping it would tear the JavaScript context down underneath.
    runtime: Option<Rc<ShellRuntime>>,
    /// Owns the quote application's policy and generation for as long as its
    /// extracted script view is mounted in this story.
    script_root: Option<Entity<ShellRoot>>,
    script: Option<Entity<ScriptView>>,
    /// A separate ScriptView, so native-motion activity cannot become quote
    /// board work or pollute the performance counters readers compare.
    motion_root: Option<Entity<ShellRoot>>,
    motion: Option<Entity<ScriptView>>,
    /// The hot-reload watcher for the script above.
    ///
    /// Held rather than detached, so the assignment below drops the previous
    /// one: Reload script mounts a new view, and a detached watcher would go on
    /// polling for the view it was started for.
    _script_watch: Option<Watcher>,
    _motion_watch: Option<Watcher>,
    /// The last load failure, kept visible instead of thrown away — a story
    /// that silently shows the previous script after a syntax error is worse
    /// than one that says what broke.
    script_error: Option<SharedString>,
    motion_error: Option<SharedString>,
    feed: Feed,
    /// Bumped whenever the feed changes, so a loop started for an older feed
    /// stops on its next tick instead of racing the new one.
    feed_generation: u64,
    /// The counters as of the last sample, and what the second before it added.
    /// A rate is the difference of two readings; the runtime does not know what
    /// a second is, and should not have to.
    sampled: RuntimeMetrics,
    rate: RuntimeMetrics,
    /// Set while the Rust half is paused: what it was showing when it stopped.
    frozen: Option<Frozen>,
    /// Set while the script half is paused, which simply means it is no longer
    /// told that its description went stale.
    script_paused: bool,
}

impl super::Story for ShellStory {
    fn title() -> &'static str {
        "Shell"
    }

    fn description() -> &'static str {
        "Run a ticking JavaScript quote board beside a Rust one, sharing state \
         through a native module."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ShellStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // `gpui_shell::init` is deliberately not called: it would install the
        // shell's own palette over the Base tokens this gallery projects from
        // its `gpui-component` theme, and the script has no need of them — it
        // reads colors from the render's call-scoped `cx.theme()`. Nothing else in
        // the runtime needs priming; the style reflection table builds itself on
        // first use.
        let market = cx.new(|_| Market::open());
        install_host_modules(&market);

        // The single place a change becomes two re-renders. Whoever moved the
        // board — the feed, a Rust button or a script button — both halves are
        // looking at the same entity, so both are told.
        cx.observe(&market, |this, _, cx| {
            if let Some(script) = &this.script.clone().filter(|_| !this.script_paused) {
                // `refresh`, not `notify`: the board is state the script reads
                // over a native call, so its description is now stale. A bare
                // notify would redraw the panel from the snapshot it already
                // published — correct for a repaint, wrong here.
                script.update(cx, |view, cx| view.refresh(cx));
            }
            cx.notify();
        })
        .detach();

        let mut story = Self {
            focus_handle: cx.focus_handle(),
            market,
            runtime: None,
            script_root: None,
            script: None,
            motion_root: None,
            motion: None,
            _script_watch: None,
            _motion_watch: None,
            script_error: None,
            motion_error: None,
            feed: Feed::Idle,
            feed_generation: 0,
            sampled: RuntimeMetrics::default(),
            rate: RuntimeMetrics::default(),
            frozen: None,
            script_paused: false,
        };

        match ShellRuntime::new(cx) {
            Ok(runtime) => {
                story.runtime = Some(runtime);
                story.reload(window, cx);
                story.reload_motion(window, cx);
            }
            Err(error) => story.script_error = Some(error.to_string().into()),
        }

        story.sample_metrics(cx);
        story.set_feed(OPENING_FEED, cx);
        story
    }

    /// Re-reads the runtime counters once a second, forever.
    ///
    /// Separate from the feed on purpose: the readout has to keep working when
    /// the feed is off, because "zero script renders while the window is busy"
    /// is one of the readings worth seeing.
    fn sample_metrics(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(SAMPLE_INTERVAL).await;
                let alive = this.update(cx, |this, cx| {
                    let Some(runtime) = &this.runtime else {
                        return;
                    };
                    let reading = runtime.read_metrics();
                    this.rate = reading.since(&this.sampled);
                    this.sampled = reading;
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Switches the feed, and starts the loop that drives it.
    ///
    /// A baseline is taken here rather than accumulated, because the readout
    /// answers "what is this feed costing", not "what has this window done since
    /// it opened". A baseline and not a reset: the counters belong to the
    /// runtime, and zeroing them would move them under anything else reading.
    fn set_feed(&mut self, feed: Feed, cx: &mut Context<Self>) {
        self.feed_generation += 1;
        self.feed = feed;

        self.sampled = match &self.runtime {
            Some(runtime) => runtime.read_metrics(),
            None => RuntimeMetrics::default(),
        };
        self.rate = RuntimeMetrics::default();
        cx.notify();

        let Some(interval) = feed.interval() else {
            return;
        };
        let generation = self.feed_generation;

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(interval).await;
                let running = this.update(cx, |this, cx| {
                    if this.feed_generation != generation {
                        return false;
                    }
                    this.tick(cx);
                    true
                });
                if !matches!(running, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// Stops or restarts the Rust half.
    fn toggle_rust(&mut self, cx: &mut Context<Self>) {
        self.frozen = match self.frozen {
            Some(_) => None,
            None => Some(Frozen::capture(self.market.read(cx))),
        };
        cx.notify();
    }

    /// Stops or restarts the script half.
    ///
    /// Resuming refreshes rather than waiting for the next tick, so the board
    /// catches up the moment the button is released rather than showing stale
    /// prices for another interval.
    fn toggle_script(&mut self, cx: &mut Context<Self>) {
        self.script_paused = !self.script_paused;
        if !self.script_paused
            && let Some(script) = &self.script
        {
            script.update(cx, |view, cx| view.refresh(cx));
        }
        cx.notify();
    }

    /// One tick of whichever feed is running.
    fn tick(&mut self, cx: &mut Context<Self>) {
        match self.feed {
            Feed::Idle => {}
            // Moving the board notifies its observers, which invalidates the
            // script view: the script has to run, because what it reads moved.
            Feed::Quotes(_) => self.market.update(cx, |market, cx| {
                market.tick();
                cx.notify();
            }),
            // Nothing the script reads has changed, so this is a repaint and
            // nothing more. The view materializes its existing snapshot and the
            // VM is never entered.
            Feed::Repaint(_) => {
                if let Some(script) = &self.script {
                    // A bare notify on purpose. This is exactly the case
                    // `refresh` exists to be distinguished from.
                    script.update(cx, |_, cx| cx.notify());
                }
            }
        }
    }

    /// Re-reads the script from disk and swaps it into the live view.
    ///
    /// The entity survives, so the panel keeps its place in the window and the
    /// board keeps its state: only what the script produced is replaced.
    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };

        let loaded = runtime
            .try_load(script_directory(), window, cx)
            .and_then(|root| {
                let view = root
                    .read(cx)
                    .content()
                    .clone()
                    .downcast::<ScriptView>()
                    .map_err(|_| {
                        anyhow::anyhow!("the quote application did not mount a script view")
                    })?;

                #[cfg(debug_assertions)]
                {
                    self._script_watch = runtime.watch(&root, window, cx).ok();
                }
                Ok((root, view))
            });

        match loaded {
            Ok((root, view)) => {
                self.script_root = Some(root);
                self.script = Some(view);
                self.script_error = None;
            }
            Err(error) => self.script_error = Some(error.to_string().into()),
        }

        cx.notify();
    }

    /// Loads the standalone motion ScriptView. It shares only the shell
    /// runtime with the quote view; it does not read Market.
    fn reload_motion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };

        let loaded = runtime
            .try_load(motion_script_directory(), window, cx)
            .and_then(|root| {
                let view = root
                    .read(cx)
                    .content()
                    .clone()
                    .downcast::<ScriptView>()
                    .map_err(|_| {
                        anyhow::anyhow!("the motion application did not mount a script view")
                    })?;

                #[cfg(debug_assertions)]
                {
                    self._motion_watch = runtime.watch(&root, window, cx).ok();
                }
                Ok((root, view))
            });

        match loaded {
            Ok((root, view)) => {
                self.motion_root = Some(root);
                self.motion = Some(view);
                self.motion_error = None;
            }
            Err(error) => self.motion_error = Some(error.to_string().into()),
        }

        cx.notify();
    }
}

/// The column widths both halves lay out to.
///
/// Shared as constants because the two panels sit side by side and a reader is
/// comparing them: a column that is 72 wide on the left and 70 on the right
/// would make the comparison about alignment instead of about rendering.
/// Column widths in **rems**, so the board scales with the window's text size
/// instead of pinning itself to a pixel grid that only exists at the default
/// zoom. `ui.js` carries the same numbers as `"…rem"` strings; the two panels
/// have to agree at every size, not only at 100%.
///
/// The values are the exact conversions of what they used to be at a 16px root,
/// so nothing moves for a reader who never changes the setting.
const SYMBOL_COLUMN: f32 = 4.875;
const PRICE_COLUMN: f32 = 4.25;
const PERCENT_COLUMN: f32 = 4.125;
const VOLUME_COLUMN: f32 = 5.125;
/// The watched dot at the end of a row. The header carries an empty cell of the
/// same width, because a trailing column the header does not know about pushes
/// every caption out of line with the numbers under it.
const WATCH_MARKER: f32 = 0.375;

/// Row density, also shared with the script half.
///
/// A quote board is a dense surface — the reader is scanning a column of
/// numbers, not reading paragraphs — so the rows sit close together and the
/// separation comes from the alignment rather than from space. Twenty rows at
/// this pitch is around 120px shorter than the comfortable spacing the rest of
/// the gallery uses, which is the difference between a panel that fits on screen
/// beside its Rust twin and one that does not.
const ROW_PADDING: f32 = 0.125;
const ROW_GAP: f32 = 0.125;

/// The gap between the panel's parts — heading, header, rows, rule, actions.
/// `SPACE.md` on the script side.
const BLOCK_GAP: f32 = 0.75;
/// Horizontal padding inside a row. `SPACE.sm` on the script side.
const ROW_INSET: f32 = 0.5;

/// The type scale, mirrored by `TYPE` in `ui.js`.
///
/// Spelled out in numbers on both sides rather than taken from either
/// framework's named sizes, because `text_xs` here and `text_size(11)` there are
/// not the same thing — and two boards set in different sizes are two boards of
/// different heights, which is the one difference this story must not have.
const TITLE_SIZE: f32 = 0.8125;
const BODY_SIZE: f32 = 0.6875;
const LINE_HEIGHT: f32 = 1.4;

impl ShellStory {
    /// The board, laid out exactly as `main.js` lays out its own.
    ///
    /// Same blocks in the same order at the same gaps, so the two panels are the
    /// same height and a reader comparing them is comparing the rendering rather
    /// than the composition.
    fn rust_panel(
        &self,
        quotes: &[Quote],
        watched: usize,
        ticks: u64,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap(rems(BLOCK_GAP))
            .child(self.rust_heading(quotes.len(), watched, ticks, cx))
            .child(self.rust_header(cx))
            .child(
                v_flex()
                    .w_full()
                    .gap(rems(ROW_GAP))
                    .children(quotes.iter().map(|quote| self.rust_row(quote, cx))),
            )
            .child(rule(cx))
            .child(self.rust_actions(quotes.len(), watched, cx))
    }

    fn rust_heading(
        &self,
        total: usize,
        watched: usize,
        ticks: u64,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_start()
            .justify_between()
            .gap(rems(ROW_INSET))
            .child(
                v_flex()
                    .gap(rems(0.125))
                    .child(title("Live quotes", cx))
                    .child(muted(
                        "Drawn by shell_story.rs · prices read from Entity<Market>",
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .items_end()
                    .gap(rems(0.125))
                    .child(body(format!("{watched} / {total} watched"), cx))
                    .child(muted(format!("tick {ticks}"), cx)),
            )
    }

    fn rust_actions(&self, total: usize, watched: usize, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap(rems(ROW_INSET))
            // The heading already carries "N / M watched"; repeating the count
            // here spent a line on a fact the reader had. The empty case is
            // worth keeping — it is the one state the heading does not explain.
            .child(muted(
                if watched == 0 {
                    "Nothing on the watchlist"
                } else {
                    ""
                },
                cx,
            ))
            .child(
                h_flex()
                    .gap(rems(0.25))
                    // Outline, not primary. Both are ordinary toolbar commands
                    // — neither is the action this panel exists to submit — and
                    // spending the one emphasis a surface has on "Watch all"
                    // leaves nothing to say when something actually is primary.
                    .child(
                        Button::new("watch-all")
                            .xsmall()
                            .outline()
                            .label("Watch all")
                            .disabled(watched == total)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.market.update(cx, |market, cx| {
                                    market.watch_all(true);
                                    cx.notify();
                                });
                            })),
                    )
                    .child(
                        Button::new("watch-none")
                            .xsmall()
                            .outline()
                            .label("Clear")
                            .disabled(watched == 0)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.market.update(cx, |market, cx| {
                                    market.watch_all(false);
                                    cx.notify();
                                });
                            })),
                    ),
            )
    }

    fn rust_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let caption = |value: &'static str, width: f32, right: bool| {
            div()
                .w(rems(width))
                .flex_none()
                .when(right, |this| this.text_right())
                .child(muted(value, cx))
        };

        h_flex()
            .w_full()
            .items_center()
            .gap(rems(ROW_INSET))
            .px(rems(ROW_INSET))
            .pb(rems(0.25))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(caption("Symbol", SYMBOL_COLUMN, false))
            .child(div().flex_1())
            .child(caption("Last", PRICE_COLUMN, true))
            .child(caption("Change", PERCENT_COLUMN, true))
            .child(caption("Volume", VOLUME_COLUMN, true))
            .child(div().w(rems(WATCH_MARKER)).flex_none())
    }

    fn rust_row(&self, quote: &Quote, cx: &Context<Self>) -> impl IntoElement {
        let symbol = quote.symbol.clone();
        let watched = quote.watched;
        let moved = direction_color(quote.direction(), cx);

        // `gpui_kit::base::Button`, which is what the script half gets — and for the
        // same two reasons. A row you can Tab to compared against a row you
        // cannot is not a comparison of two renderers, it is a comparison of one
        // of them against something that is not a control. And Base ships
        // behavior with no appearance, so both halves are drawing the row rather
        // than one of them inheriting a control's height and padding: the
        // component Button imposes `h_8`, which made these rows twice as tall as
        // the script's for no reason a reader could see.
        BaseButton::new(SharedString::from(format!("quote-{symbol}")))
            .accessibility_label(format!("Watch {}", quote.name))
            .hover(|mut style| {
                style.background = Some(cx.theme().muted.into());
                style
            })
            .flex()
            .w_full()
            .items_center()
            .gap(rems(ROW_INSET))
            .px(rems(ROW_INSET))
            .py(rems(ROW_PADDING))
            .rounded(cx.theme().radius)
            .on_click(cx.listener(move |this, _, _, cx| {
                let symbol = symbol.clone();
                this.market.update(cx, |market, cx| {
                    market.set_watched(&symbol, !watched);
                    cx.notify();
                });
            }))
            .child(
                div()
                    .w(rems(SYMBOL_COLUMN))
                    .flex_none()
                    .child(body(quote.symbol.clone(), cx).font_medium()),
            )
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .child(muted(quote.name.clone(), cx)),
            )
            .child(
                div()
                    .w(rems(PRICE_COLUMN))
                    .flex_none()
                    .text_right()
                    .child(body(format!("{:.2}", quote.last), cx).text_color(moved)),
            )
            .child(
                div()
                    .w(rems(PERCENT_COLUMN))
                    .flex_none()
                    .text_right()
                    .child(body(format!("{:+.2}%", quote.change_percent()), cx).text_color(moved)),
            )
            .child(
                div()
                    .w(rems(VOLUME_COLUMN))
                    .flex_none()
                    .text_right()
                    .child(muted(thousands(quote.volume), cx)),
            )
            .child(
                div()
                    .w(rems(WATCH_MARKER))
                    .h(rems(WATCH_MARKER))
                    .flex_none()
                    .rounded_full()
                    .when(watched, |this| this.bg(cx.theme().primary)),
            )
    }

    /// The two counters, side by side, with what they mean underneath.
    ///
    /// Rates rather than totals: a total answers "how much work has this window
    /// ever done", and the question here is "what is this costing right now".
    fn readout(&self, cx: &Context<Self>) -> impl IntoElement {
        let script = self.rate.script_renders();
        let frames = self.rate.materializations();

        v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .gap_6()
                    .child(reading(
                        "Script renders",
                        format!("{script}/s"),
                        // Split, because the whole pass is not JavaScript: the
                        // host answering `quotes()` is in there too, and
                        // charging it to the script would flatter the runtime.
                        format!(
                            "{:.2} ms describing · {:.2} ms in host calls",
                            millis(self.rate.mean_script_only()),
                            millis(self.rate.mean_native()),
                        ),
                        cx,
                    ))
                    .child(reading(
                        "Frames drawn",
                        format!("{frames}/s"),
                        format!("{:.2} ms each", millis(self.rate.mean_materialize())),
                        cx,
                    ))
                    .child(reading("Feed", self.feed.caption(), self.feed.detail(), cx)),
            )
            .when(self.frozen.is_some() || self.script_paused, |this| {
                this.child(muted(
                    match (self.frozen.is_some(), self.script_paused) {
                        (true, true) => {
                            "Both halves are paused. The feed still runs, so the counters show \
                             what the window costs when neither board is following it."
                        }
                        (true, false) => {
                            "The Rust half is paused: it is drawing a copy of what it last read, \
                             because its render has no snapshot of its own to hold."
                        }
                        _ => {
                            "The script half is paused: nobody is telling it its description went \
                             stale, so it keeps drawing the one it already published — and the \
                             script render count is zero while the board next to it moves."
                        }
                    },
                    cx,
                ))
            })
            .child(muted(
                format!(
                    "Slowest single script render this run: {:.2} ms",
                    millis(self.sampled.slowest_script_render()),
                ),
                cx,
            ))
            .child(muted(shape_repeats(&self.rate), cx))
            .child(
                Label::new(match self.feed {
                    Feed::Idle => {
                        "No feed: hovering the script panel draws frames and runs no script."
                    }
                    Feed::Quotes(_) => {
                        "The script reads the prices, so every tick invalidates its snapshot: \
                         script renders track the feed, not the frame rate."
                    }
                    Feed::Repaint(_) => {
                        "Nothing the script reads changed, so every tick repaints the snapshot it \
                         already published: frames climb, script renders stay at zero."
                    }
                })
                .text_xs()
                .text_color(cx.theme().muted_foreground),
            )
    }

    fn script_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_2()
            .when_some(self.script_error.clone(), |this, message| {
                this.child(
                    v_flex()
                        .w_full()
                        .gap_1()
                        .p_2()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().danger)
                        .child(Label::new("The script did not load").text_xs())
                        .child(
                            Label::new(message)
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                        ),
                )
            })
            .children(self.script.clone())
    }

    fn motion_panel(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_2()
            .when_some(self.motion_error.clone(), |this, message| {
                this.child(
                    v_flex()
                        .w_full()
                        .gap_1()
                        .p_2()
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().danger)
                        .child(Label::new("The motion script did not load").text_xs())
                        .child(
                            Label::new(message)
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                        ),
                )
            })
            .children(self.motion.clone())
    }
}

/// Pause or resume one half.
///
/// The label carries the state, so the button does not also wear a selected
/// style: a control that says "Resume" is already telling you it is paused, and
/// styling it as well spends emphasis on something the word has covered.
fn pause_button(
    id: &'static str,
    paused: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    Button::new(id)
        .xsmall()
        .outline()
        .label(if paused { "Resume" } else { "Pause" })
        .on_click(on_click)
}

/// The three type roles, mirroring `title`, `label` and `muted` in `ui.js`.
fn title(value: impl Into<SharedString>, cx: &Context<ShellStory>) -> Label {
    Label::new(value)
        .text_size(rems(TITLE_SIZE))
        .line_height(relative(1.3))
        .font_semibold()
        .text_color(cx.theme().foreground)
}

fn body(value: impl Into<SharedString>, cx: &Context<ShellStory>) -> Label {
    Label::new(value)
        .text_size(rems(BODY_SIZE))
        .line_height(relative(LINE_HEIGHT))
        .text_color(cx.theme().foreground)
}

fn muted(value: impl Into<SharedString>, cx: &Context<ShellStory>) -> Label {
    Label::new(value)
        .text_size(rems(BODY_SIZE))
        .line_height(relative(LINE_HEIGHT))
        .text_color(cx.theme().muted_foreground)
}

/// The hairline between the rows and the actions under them.
fn rule(cx: &Context<ShellStory>) -> impl IntoElement {
    div().w_full().h(px(1.)).flex_none().bg(cx.theme().border)
}

/// Up is `success`, down is `danger`, flat is ordinary text. Both halves ask
/// this question of the same theme, which is why the two panels agree.
fn direction_color(direction: i32, cx: &Context<ShellStory>) -> Hsla {
    match direction {
        1 => cx.theme().success,
        -1 => cx.theme().danger,
        _ => cx.theme().foreground,
    }
}

/// The native modules this story installed capture its `Entity<Market>`, and the
/// registry they live in is process-wide. Leaving them there would keep the
/// entity alive after the story is gone — which GPUI reports as a leaked handle,
/// and which is exactly the shape of leak a plugin host would hit on unload.
impl Drop for ShellStory {
    fn drop(&mut self) {
        gpui_shell::clear_exported_modules();
    }
}

impl Focusable for ShellStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShellStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // What the Rust half draws: the live board, or the copy it kept when it
        // was paused. The script half needs no equivalent here — it is holding
        // its own published description.
        let shown = match &self.frozen {
            Some(frozen) => frozen.clone(),
            None => Frozen::capture(self.market.read(cx)),
        };
        let (quotes, watched, ticks) = (shown.quotes, shown.watched, shown.ticks);

        v_flex()
            .size_full()
            .gap_3()
            .child(
                h_flex()
                    .w_full()
                    .items_start()
                    .gap_4()
                    .child(
                        div().flex_1().child(
                            section("Rust")
                                .sub_title(pause_button(
                                    "pause-rust",
                                    self.frozen.is_some(),
                                    cx.listener(|this, _, _, cx| this.toggle_rust(cx)),
                                ))
                                .v_flex()
                                .child(self.rust_panel(&quotes, watched, ticks, cx)),
                        ),
                    )
                    .child(
                        div().flex_1().child(
                            section("JavaScript · gpui-shell")
                                .sub_title(
                                    h_flex()
                                        .gap(rems(0.25))
                                        .child(pause_button(
                                            "pause-script",
                                            self.script_paused,
                                            cx.listener(|this, _, _, cx| this.toggle_script(cx)),
                                        ))
                                        .child(
                                            Button::new("reload-script")
                                                .xsmall()
                                                .outline()
                                                .label("Reload script")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.reload(window, cx);
                                                })),
                                        ),
                                )
                                .v_flex()
                                .child(self.script_panel(cx)),
                        ),
                    ),
            )
            .child(
                section("Render frequency")
                    .description(
                        "A script render and a GPUI frame are not the same event. Change the feed \
                         and watch the two counters come apart.",
                    )
                    // A segmented control, because this is one setting with
                    // four values rather than four commands. Swapping `primary`
                    // onto whichever button was current said "this is the action
                    // to take" where it meant "this is the one in force" — two
                    // different things wearing one style.
                    .sub_title(
                        div().min_w_0().flex_1().max_w(rems(32.)).child(
                            TabBar::new("feed")
                                .w_full()
                                .segmented()
                                .selected_index(
                                    FEEDS
                                        .iter()
                                        .position(|(_, feed)| *feed == self.feed)
                                        .unwrap_or(0),
                                )
                                .on_click(cx.listener(|this, index: &usize, _, cx| {
                                    if let Some((_, feed)) = FEEDS.get(*index) {
                                        this.set_feed(*feed, cx);
                                    }
                                }))
                                .children(FEEDS.map(|(_, feed)| Tab::new().label(feed.caption()))),
                        ),
                    )
                    .v_flex()
                    .child(self.readout(cx)),
            )
            .child(
                section("Native motion · gpui-shell")
                    .description(
                        "A separate ScriptView with pixel targets. Clicking a control retargets \
                         transition or spring tracks; GPUI owns every in-between frame.",
                    )
                    .sub_title(
                        Button::new("reload-motion-script")
                            .xsmall()
                            .outline()
                            .label("Reload motion")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reload_motion(window, cx);
                            })),
                    )
                    .v_flex()
                    .child(self.motion_panel(cx)),
            )
            .child(
                section("Where the boundary is")
                    .description(
                        "The script holds no host object. Market data crosses the one native \
                         module this story registered; appearance comes from gpui-shell's \
                         call-scoped context.",
                    )
                    .v_flex()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_1()
                            .child(boundary_line(
                                "native(\"market\")",
                                "quotes() · ticks() · watch(symbol) · watch_all(on)",
                                cx,
                            ))
                            .child(boundary_line(
                                "cx.theme()",
                                "read-only semantic colors, spacing, radius and mode",
                                cx,
                            ))
                            .child(boundary_line(
                                "Editing main.js",
                                "needs no rebuild: press Reload script above",
                                cx,
                            )),
                    ),
            )
    }
}

/// One counter: what it is, the number, and what the number means.
///
/// The number is the focal point and gets the size; the label above it and the
/// detail below it are both quiet, because a reader glancing at this is
/// comparing two figures, not reading three lines.
fn reading(
    caption: &'static str,
    value: String,
    detail: String,
    cx: &Context<ShellStory>,
) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            Label::new(caption)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
        .child(Label::new(value).font_semibold())
        .child(
            Label::new(detail)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// How often a rebuild described the shape it replaced, in words.
///
/// The counter behind it is a measurement rather than a feature: nothing skips
/// work when a shape repeats. It is here because this story is the one place a
/// script runs against a live feed, and the rate this line reports is the
/// ceiling a template cache could reach — see §20.7 of `docs/gpui-shell.md`.
fn shape_repeats(rate: &RuntimeMetrics) -> String {
    let compared = rate.structure_repeats() + rate.structure_changes();
    match rate.structure_repeat_rate() {
        Some(share) => format!(
            "Shape repeated on {} of {compared} rebuilds ({:.0}%): the values moved and the \
             structure did not.",
            rate.structure_repeats(),
            share * 100.0,
        ),
        None => "No rebuild has had a previous description to compare with yet.".to_string(),
    }
}

/// One line of the boundary summary: the call on the left, what it does on the
/// right. Two columns rather than a sentence, because the reader is scanning
/// for a name, not reading prose.
fn boundary_line(
    call: &'static str,
    detail: &'static str,
    cx: &Context<ShellStory>,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .items_start()
        .gap_3()
        .child(Label::new(call).text_xs().font_medium().w_48())
        .child(
            Label::new(detail)
                .text_xs()
                .text_color(cx.theme().muted_foreground),
        )
}

#[cfg(test)]
mod tests {
    use std::ops::Deref as _;

    // The glob also carries GPUI's `test` attribute; keep the built-in one for `#[test]`.
    use core::prelude::v1::test;
    use gpui_kit::*;

    use super::*;

    struct ScriptRoot {
        view: Entity<ScriptView>,
    }

    impl Render for ScriptRoot {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.view.clone().into_any_element()
        }
    }

    /// The work an asynchronous host function does after it leaves the main
    /// thread.
    ///
    /// A plain function on owned data, and tested as one — which is the point
    /// of the split: everything the future needs was copied out while the
    /// synchronous half still had the `App`, so what remains has no context to
    /// stand up and can be checked directly.
    #[test]
    fn the_summary_names_the_session_extremes() {
        let value = summarise(vec![
            ("AAPL.US".into(), 1.5),
            ("MSFT.US".into(), -2.25),
            ("NVDA.US".into(), 0.75),
        ]);

        let field = |name: &str| {
            value
                .get(name)
                .and_then(HostValue::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        assert_eq!(field("leader"), "AAPL.US");
        assert_eq!(field("leader_percent"), "+1.50%");
        assert_eq!(field("laggard"), "MSFT.US");
        assert_eq!(field("laggard_percent"), "-2.25%");
        assert_eq!(field("average_percent"), "+0.00%");
    }

    /// An empty board is answered, not divided by zero.
    #[test]
    fn the_summary_of_an_empty_board_is_still_a_record() {
        let value = summarise(Vec::new());
        assert_eq!(
            value.get("average_percent").and_then(HostValue::as_str),
            Some("+0.00%")
        );
    }

    /// Story-specific host access is only for market data. Theme tokens are
    /// supplied by gpui-shell's live call context, so the host must not keep a
    /// second theme projection registered beside it.
    ///
    /// `validate` is part of the assertion: it is what checks `MARKET_TYPES`
    /// against the functions actually registered, so a rename on either side
    /// fails here rather than in an editor.
    #[gpui_kit::test]
    fn story_host_registry_only_grants_market(cx: &mut TestAppContext) {
        let market = cx.new(|_| Market::open());
        let module = market_module(&market);

        assert_eq!(module.name(), "market");
        assert_eq!(
            module.function_names(),
            vec!["quotes", "summary", "ticks", "watch", "watch_all"]
        );
        // Only the slow one answers with a promise. Asserted rather than
        // assumed, because the script `await`s exactly this one and the
        // generated binding differs by it.
        assert!(
            module.is_async("summary"),
            "main.js awaits summary(), and the generated binding differs by this"
        );
        for synchronous in ["quotes", "ticks", "watch", "watch_all"] {
            assert!(!module.is_async(synchronous), "{synchronous}");
        }
        module
            .validate()
            .expect("the declared types match the registered functions");
    }

    /// The claim the counters under the panels make, checked without a person
    /// having to watch two numbers.
    ///
    /// It is worth an end-to-end test rather than a unit one because it spans
    /// every part that has to agree: the entity, the host module, the script's
    /// `ticks()` call, and the difference between `refresh` and `notify`.
    #[gpui_kit::test]
    fn a_quote_tick_re_runs_the_script_and_a_repaint_does_not(cx: &mut TestAppContext) {
        // The story reads shell theme tokens through `cx.theme()`, so
        // the theme has to exist before the script's first render.
        cx.update(gpui_kit::init);
        let window = cx.add_window(|window, cx| ShellStory::new(window, cx));
        let story = cx.update(|cx| window.entity(cx)).expect("the story");
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (runtime, script) = story.read_with(&mut context, |story, _| {
            assert!(
                story.script_error.is_none(),
                "the story's script did not load: {:?}",
                story.script_error
            );
            (story.runtime.clone(), story.script.clone())
        });
        let runtime = runtime.expect("a runtime");
        let script = script.expect("a script view");

        draw(&mut context, &script);
        assert!(
            description(&mut context, &script).contains("tick 0"),
            "the script should be painting the tick count"
        );

        let baseline = runtime.read_metrics().script_renders();

        // A quote tick moves prices the script reads, so the description is
        // stale and has to be rebuilt.
        tick(&mut context, &story, Feed::Quotes(50));
        draw(&mut context, &script);
        assert_eq!(
            runtime.read_metrics().script_renders(),
            baseline + 1,
            "a quote tick must re-run the script"
        );
        assert!(description(&mut context, &script).contains("tick 1"));

        // A repaint tick changes nothing the script can see.
        for _ in 0..8 {
            tick(&mut context, &story, Feed::Repaint(16));
            draw(&mut context, &script);
        }
        assert_eq!(
            runtime.read_metrics().script_renders(),
            baseline + 1,
            "eight repaints must not enter the VM"
        );
        assert!(
            description(&mut context, &script).contains("tick 1"),
            "and the description must be the one already published"
        );
    }

    /// How often a quote tick rebuilds the shape it replaced, on the story's
    /// own script rather than on a panel written to flatter the question.
    ///
    /// This is §20.7's first experiment: a template cache is worth designing
    /// only if a dirty render usually produces the structure the previous one
    /// produced. The board here is twenty rows of six cells fed by a live
    /// market entity, which is the workload the chapter's numbers come from.
    ///
    /// The bound is deliberately loose. The exact rate depends on what the feed
    /// does — a watched flag flipping genuinely is a different shape — and
    /// pinning it would turn an unrelated change to the script into a failure.
    /// What the test defends is the claim: a moving price is a value, not a
    /// structure.
    #[gpui_kit::test]
    fn a_quote_feed_mostly_repeats_the_panel_s_shape(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.add_window(|window, cx| ShellStory::new(window, cx));
        let story = cx.update(|cx| window.entity(cx)).expect("the story");
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (runtime, script) = story.read_with(&mut context, |story, _| {
            (
                story.runtime.clone().expect("a runtime"),
                story.script.clone().expect("a script view"),
            )
        });

        draw(&mut context, &script);
        let baseline = runtime.read_metrics();

        for _ in 0..40 {
            tick(&mut context, &story, Feed::Quotes(50));
            draw(&mut context, &script);
        }

        let reading = runtime.read_metrics().since(&baseline);
        let compared = reading.structure_repeats() + reading.structure_changes();
        println!(
            "\n[F] shape repeats on the story's own board — {} of {compared} rebuilds ({:.0}%)",
            reading.structure_repeats(),
            reading.structure_repeat_rate().unwrap_or_default() * 100.0,
        );

        assert!(compared >= 30, "the feed should have rebuilt the panel");
        assert!(
            reading.structure_repeat_rate().unwrap_or_default() > 0.8,
            "a moving price is a value, not a structure: {} of {compared} rebuilds repeated",
            reading.structure_repeats(),
        );
    }

    /// Pausing one half must not pause the other, and the two mechanisms are
    /// genuinely different: the script half stops because nothing invalidates
    /// it, the Rust half because the story keeps a copy.
    #[gpui_kit::test]
    fn each_half_pauses_on_its_own(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.add_window(|window, cx| ShellStory::new(window, cx));
        let story = cx.update(|cx| window.entity(cx)).expect("the story");
        let mut context = VisualTestContext::from_window(*window.deref(), cx);

        let (runtime, script) = story.read_with(&mut context, |story, _| {
            (
                story.runtime.clone().expect("a runtime"),
                story.script.clone().expect("a script view"),
            )
        });
        draw(&mut context, &script);

        // Paused script: the feed keeps running and the board keeps its
        // description, so the VM is never entered.
        story.update(&mut context, |story, cx| story.toggle_script(cx));
        let baseline = runtime.read_metrics().script_renders();
        for _ in 0..8 {
            tick(&mut context, &story, Feed::Quotes(50));
            draw(&mut context, &script);
        }
        assert_eq!(
            runtime.read_metrics().script_renders(),
            baseline,
            "a paused script half must not run"
        );
        assert!(description(&mut context, &script).contains("tick 0"));

        // The Rust half was never paused, so it is looking at the live board.
        story.read_with(&mut context, |story, cx| {
            assert!(story.frozen.is_none());
            assert_eq!(story.market.read(cx).ticks, 8);
        });

        // Resuming catches the script up on the spot rather than at the next
        // tick.
        story.update(&mut context, |story, cx| story.toggle_script(cx));
        draw(&mut context, &script);
        assert!(
            description(&mut context, &script).contains("tick 8"),
            "resuming must refresh rather than wait for the feed"
        );

        // Paused Rust: the copy stops moving while the board underneath does not.
        story.update(&mut context, |story, cx| story.toggle_rust(cx));
        let frozen_at = story
            .read_with(&mut context, |story, _| story.frozen.clone())
            .expect("a frozen copy")
            .ticks;
        for _ in 0..4 {
            tick(&mut context, &story, Feed::Quotes(50));
        }
        story.read_with(&mut context, |story, cx| {
            assert_eq!(
                story.frozen.as_ref().expect("still frozen").ticks,
                frozen_at,
                "the copy must not follow the feed"
            );
            assert_eq!(story.market.read(cx).ticks, frozen_at + 4);
        });
    }

    /// The board moves the same way twice, so the panel a reader sees on one run
    /// is the panel they saw on the last one.
    #[gpui_kit::test]
    fn the_feed_is_deterministic(_: &mut TestAppContext) {
        let mut first = Market::open();
        let mut second = Market::open();
        for _ in 0..64 {
            first.tick();
            second.tick();
        }

        let prices = |market: &Market| {
            market
                .quotes
                .iter()
                .map(|quote| format!("{:.4}", quote.last))
                .collect::<Vec<_>>()
        };
        assert_eq!(prices(&first), prices(&second));
        assert!(
            prices(&first) != prices(&Market::open()),
            "sixty-four ticks should have moved something"
        );
    }

    /// The performance board must stay a pure quote-list workload, while motion
    /// is its own runnable script resource. Numeric targets are essential:
    /// shell materialization only samples pixel lengths for native motion.
    #[test]
    fn quote_and_motion_scripts_keep_their_contracts_separate() {
        let quote_source = std::fs::read_to_string(script_directory().join(ENTRY))
            .expect("the checked-in shell story script");
        let quote_ui_source = std::fs::read_to_string(script_directory().join("ui.js"))
            .expect("the checked-in shell story presentation helpers");
        let motion_source = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("js/motion/main.js"),
        )
        .expect("the checked-in motion story script");

        for forbidden in ["motionLab", "motion-transition", ".transition(", ".spring("] {
            assert!(
                !quote_source.contains(forbidden),
                "the performance quote script must not contain `{forbidden}`"
            );
        }

        for required in [
            "motion-transition",
            "motion-spring",
            "motion-trigger",
            "motion-policy-segment",
            "Run motion",
            "AAPL",
            "$228.26",
            "+1.84%",
            "Live",
            "✓",
            ".selected(active)",
            ".transition(\"left\"",
            ".transition(\"opacity\"",
            ".spring(\"left\"",
            ".spring(\"opacity\"",
        ] {
            assert!(
                motion_source.contains(required),
                "the independent motion script must demonstrate `{required}`"
            );
        }

        for rem_target in [
            ".left(active ? \"",
            ".w(active ? \"",
            ".top(active ? \"",
            ".left(\"",
            ".w(\"",
            ".top(\"",
        ] {
            assert!(
                !motion_source.contains(rem_target),
                "motion target `{rem_target}` must be numeric pixels"
            );
        }
        assert!(
            !motion_source.contains("rem"),
            "the motion script must not reintroduce relative-length targets"
        );
        for forbidden in ["const colors = cx.theme()", "palette()", "refreshPalette"] {
            assert!(
                !motion_source.contains(forbidden),
                "the motion example must read theme values directly through cx.theme(); found `{forbidden}`"
            );
        }
        assert!(
            motion_source.contains("cx.theme().colors.foreground"),
            "the motion example must demonstrate explicit nested theme-token access"
        );
        for source in [&quote_source, &quote_ui_source] {
            for forbidden in ["palette()", "refreshPalette", "const colors = cx.theme()"] {
                assert!(
                    !source.contains(forbidden),
                    "the quote example must not cache or alias its theme; found `{forbidden}`"
                );
            }
        }
        assert!(
            quote_ui_source.contains("cx.theme().colors.foreground"),
            "the quote helpers must read semantic colors from the current context"
        );
        assert!(
            motion_source.contains(".left(active ? REST_LEFT + TRAVEL : REST_LEFT)"),
            "the motion card must travel between the two stations the track marks"
        );
        assert!(
            motion_source.contains(".overflow_hidden()"),
            "the stage must clip: its children are absolute, and the panel's width is not ours"
        );
        for forbidden in ["colors.primary", "colors.primary_foreground"] {
            assert!(
                !motion_source.contains(forbidden),
                "the motion demo must use quiet semantic surfaces, not `{forbidden}`"
            );
        }
        assert!(
            !motion_source.contains(".disabled(active)"),
            "the selected motion policy must remain focusable and clickable"
        );
    }

    /// The motion lab has its own loaded object, rather than being extra work
    /// inside the quote board. A pointer event retargets it, then GPUI samples
    /// native animation frames without returning to QuickJS.
    #[gpui_kit::test]
    fn standalone_motion_view_retargets_native_frames(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.add_window(|window, cx| ShellStory::new(window, cx));
        let story = cx.update(|cx| window.entity(cx)).expect("the story");

        let (runtime, motion) = cx.update(|cx| {
            let story = story.read(cx);
            assert!(
                story.motion_error.is_none(),
                "the independent motion script did not load: {:?}",
                story.motion_error
            );
            assert!(story.script.is_some(), "the separate quote view");
            (
                story.runtime.clone().expect("a runtime"),
                story.motion.clone().expect("the independent motion view"),
            )
        });

        // Mount this view as a real window root. `VisualTestContext::draw` is
        // sufficient for snapshots, but intentionally does not install mouse
        // hitboxes; this path proves the actual GPUI event dispatch instead.
        let motion_root = motion.clone();
        let (_, context) = cx.add_window_view(move |_, _| ScriptRoot { view: motion_root });
        context.update(|window, cx| window.draw(cx).clear(cx));

        assert!(description(context, &motion).contains("Native motion"));
        assert!(description(context, &motion).contains("AAPL"));
        assert!(description(context, &motion).contains("$228.26"));
        assert!(description(context, &motion).contains("Live"));

        let baseline = runtime.read_metrics().script_renders();
        // This lands on the independent 32px Run action after the segmented
        // policy choice, through GPUI hit testing rather than callback access.
        context.simulate_click(point(px(270.), px(64.)), Modifiers::default());
        context.update(|window, cx| window.draw(cx).clear(cx));
        assert!(description(context, &motion).contains("Send back"));
        assert_eq!(
            runtime.read_metrics().script_renders(),
            baseline + 1,
            "a motion retarget rebuilds only the independent motion script"
        );

        let pending = context.update(|window, cx| window.simulate_next_frame(cx));
        assert!(
            pending >= 1,
            "pixel motion targets must schedule native frames"
        );
        assert_eq!(
            runtime.read_metrics().script_renders(),
            baseline + 1,
            "sampling native frames must not re-enter QuickJS"
        );
    }

    /// Choosing the interpolation policy is not itself an animation command.
    /// The selected segment remains an ordinary interactive control, while the
    /// independent Run action is the only thing that changes the card target.
    #[gpui_kit::test]
    fn selecting_a_motion_policy_does_not_run_the_motion(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let window = cx.add_window(|window, cx| ShellStory::new(window, cx));
        let story = cx.update(|cx| window.entity(cx)).expect("the story");
        let (runtime, motion) = cx.update(|cx| {
            let story = story.read(cx);
            (
                story.runtime.clone().expect("a runtime"),
                story.motion.clone().expect("the independent motion view"),
            )
        });

        let motion_root = motion.clone();
        let (_, context) = cx.add_window_view(move |_, _| ScriptRoot { view: motion_root });
        context.update(|window, cx| window.draw(cx).clear(cx));
        assert!(description(context, &motion).contains("Transition ✓"));
        assert!(description(context, &motion).contains("Run motion"));

        let baseline = runtime.read_metrics().script_renders();
        context.simulate_click(point(px(145.), px(64.)), Modifiers::default());
        context.update(|window, cx| window.draw(cx).clear(cx));

        assert!(description(context, &motion).contains("Spring ✓"));
        assert!(description(context, &motion).contains("Run motion"));
        assert_eq!(runtime.read_metrics().script_renders(), baseline + 1);
        assert_eq!(
            context.update(|window, cx| window.simulate_next_frame(cx)),
            0,
            "selecting a policy must not retarget the card"
        );
    }

    fn tick(context: &mut VisualTestContext, story: &Entity<ShellStory>, feed: Feed) {
        story.update(context, |story, cx| {
            story.feed = feed;
            story.tick(cx);
        });
    }

    fn draw(context: &mut VisualTestContext, script: &Entity<ScriptView>) {
        let script = script.clone();
        context.draw(
            gpui_kit::Point::default(),
            gpui_kit::size(gpui_kit::px(520.), gpui_kit::px(420.)),
            move |_, _| script.into_any_element(),
        );
    }

    /// The published description, read without entering the VM.
    fn description(context: &mut VisualTestContext, script: &Entity<ScriptView>) -> String {
        context.update(|_, cx| {
            script
                .read(cx)
                .snapshot()
                .map(gpui_shell::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    }
}
