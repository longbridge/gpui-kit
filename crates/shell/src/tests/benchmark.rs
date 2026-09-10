//! What a script view costs, split into the three costs that are not the same.
//!
//! The design doc (§20.3) asked one question — how long does it take script code
//! to describe a realistic interface — and that number still decides whether the
//! approach is viable. But it is no longer the frame cost, because a description
//! is built once and replayed by every frame that follows it. So the measurement
//! is split three ways:
//!
//! ```text
//! A  script ──▶ snapshot      cost of one application invalidation
//! B  snapshot ──▶ elements    cost that GPUI actually pays per frame
//! C  snapshot ──▶ N frames    proof that A is not paid per frame at all
//! ```
//!
//! A and B are timings. C is an architectural assertion with a timing attached:
//! if the script render count moves while a clean view repaints, the runtime has
//! regressed to the coupling this design exists to remove.
//!
//! Run with output:
//!
//! ```text
//! cargo test -p gpui-shell --release tests::benchmark -- --nocapture
//! ```

use std::{ops::Deref, time::Instant};

use crate::{RenderSnapshot, ScriptView, ShellRuntime, materialize::materialize};
use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};
#[cfg(not(debug_assertions))]
use sha2::{Digest as _, Sha256};

/// Rows and columns chosen to land near the doc's "typical panel" figure:
/// 40 rows x 5 cells plus wrappers is ~250 nodes, each carrying 8-12 ops.
const ROWS: usize = 40;
const COLUMNS: usize = 5;
const ITERATIONS: usize = 50;
#[cfg(not(debug_assertions))]
const ACCEPTANCE_ITERATIONS: usize = 200;
#[cfg(not(debug_assertions))]
const JIT_WARMUP_RENDERS: usize = 64;
#[cfg(not(debug_assertions))]
const RELOAD_OBSERVATIONS: usize = 5;
/// How many batches of [`ITERATIONS`] a timing takes before believing the
/// fastest one.
const ROUNDS: usize = 7;

/// Panel sizes the scaling test walks, from the typical panel above to one no
/// single view should hold. The last two exist to show where the description
/// stops fitting a frame — and that the frame still never enters the VM.
const SIZES: [(usize, usize); 4] = [(40, 5), (100, 10), (200, 10), (400, 10)];

const TEMPLATE: &str = r#"
import { View, div } from "gpui-kit";
import { v_flex, h_flex, Button } from "gpui-base";

export default class Grid extends View {
  init() {
    this.rows = __ROWS__;
    this.columns = __COLUMNS__;
  }

  cell(row, column) {
    return div()
      .flex()
      .items_center()
      .justify_center()
      .w(90)
      .h(24)
      .px(6)
      .rounded(4)
      .bg(`#f8f8f8`)
      .text_color(`#111111`)
      .text_sm()
      .child(`${row}:${column}`);
  }

  row(row) {
    const cells = [];
    for (let column = 0; column < this.columns; column += 1) {
      cells.push(this.cell(row, column));
    }
    return h_flex().gap(6).py(2).children(cells);
  }

  render(cx) {
    const rows = [];
    for (let row = 0; row < this.rows; row += 1) {
      rows.push(this.row(row));
    }
    return v_flex()
      .size_full()
      .p(12)
      .gap(4)
      .bg(`#ffffff`)
      .children(rows)
      .child(Button.new("refresh").px(10).py(4).rounded(6).bg(`#2563eb`).child("Refresh"));
  }
}
"#;

#[cfg(not(debug_assertions))]
const COMPUTE_TEMPLATE: &str = r#"
import { View, div } from "gpui-kit";

function layoutKernel(batches, seed) {
  let checksum = seed;
  for (let batch = 0; batch < batches; batch += 1) {
    let a = 0;
    let b = 1;
    for (let i = 0; i < 40; i += 1) {
      const next = a + b;
      a = b;
      b = next;
    }
    checksum = b;
  }
  return checksum;
}

export default class NumericLayout extends View {
  render(cx) {
    return div().child(`layout:${layoutKernel(100, 0)}`);
  }
}
"#;

#[cfg(not(debug_assertions))]
const MIXED_MARKET_TEMPLATE: &str = r#"
import { View, div } from "gpui-kit";

function quoteScore(seed, index) {
  let previous = seed;
  let current = seed + index + 1;
  let aggregate = 0;
  for (let sample = 0; sample < 32; sample += 1) {
    const next = previous + current;
    previous = current;
    current = next;
    aggregate += current & 2047;
  }
  return aggregate;
}

function compareQuotes(left, right) {
  return right.score - left.score;
}

export default class MarketPanel extends View {
  render(cx) {
    const quotes = [];
    let total = 0;
    for (let index = 0; index < 96; index += 1) {
      const score = quoteScore(17, index);
      total += score;
      quotes.push({ index, score });
    }
    quotes.sort(compareQuotes);

    const visible = [];
    for (let rank = 0; rank < 12; rank += 1) {
      const quote = quotes[rank];
      visible.push(
        div()
          .h_flex()
          .justify_between()
          .child(`SYM${quote.index}`)
          .child(`${quote.score}`),
      );
    }
    return div().v_flex().gap(2).child(`total:${total}`).children(visible);
  }
}
"#;

const _: () = assert!(ROWS > 0 && COLUMNS > 0);

#[test]
fn p99_excludes_one_outlier_from_two_hundred_observations() {
    let mut samples = vec![1_u64; 199];
    samples.push(1_000);

    assert_eq!(p99(samples), 1);
}

#[test]
fn reload_median_retains_normal_observations_when_one_is_interrupted() {
    assert_eq!(median_ns(vec![700, 710, 720, 730, 4_000]), 720);
}

fn p99(mut samples: Vec<u64>) -> u64 {
    assert!(!samples.is_empty(), "P99 needs at least one sample");
    let rank = (samples.len() * 99).div_ceil(100);
    *samples.select_nth_unstable(rank - 1).1
}

fn median_ns(mut samples: Vec<u64>) -> u64 {
    assert!(
        samples.len() % 2 == 1,
        "reload observations must have an odd count"
    );
    let middle = samples.len() / 2;
    *samples.select_nth_unstable(middle).1
}

fn source(rows: usize, columns: usize) -> String {
    TEMPLATE
        .replace("__ROWS__", &rows.to_string())
        .replace("__COLUMNS__", &columns.to_string())
}

#[gpui::test]
fn describing_a_panel_stays_inside_the_frame_budget(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = grid(cx, ROWS, COLUMNS);

    // Warm up: the style table and the module both initialize lazily.
    let nodes = context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .expect("render")
            .len()
    });

    // Best of several batches rather than one average. Every source of noise
    // available to a benchmark on a shared machine adds time; none removes it,
    // so the fastest batch is the closest reading of what the work costs — and
    // averaging one batch made changes worth a few per cent unreadable.
    let mut per_build = std::time::Duration::MAX;
    context.update(|window, cx| {
        for _ in 0..ROUNDS {
            let started = Instant::now();
            for _ in 0..ITERATIONS {
                runtime
                    .build_snapshot(&object, None, crate::policy::default(), window, cx)
                    .expect("render");
            }
            per_build = per_build.min(started.elapsed() / ITERATIONS as u32);
        }
    });

    let ops = nodes * 10; // roughly ten recorded calls per node
    println!(
        "\n[A] script → snapshot: {nodes} nodes ({ROWS}x{COLUMNS}) — {:.3} ms per build, \
         ~{} ns per recorded op ({ITERATIONS} iterations)",
        per_build.as_secs_f64() * 1000.0,
        per_build.as_nanos() as usize / ops.max(1),
    );

    let automatic_tree = context
        .update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("automatic parity render")
        })
        .debug_tree();
    let automatic_renders = runtime.read_metrics().script_renders();
    let (interpreter, mut interpreter_context, interpreter_object) =
        runtime_with_source(cx, &source(ROWS, COLUMNS), true);
    let mut interpreted_tree = String::new();
    for _ in 0..automatic_renders {
        interpreted_tree = interpreter_context
            .update(|window, cx| {
                interpreter
                    .build_snapshot(
                        &interpreter_object,
                        None,
                        crate::policy::default(),
                        window,
                        cx,
                    )
                    .expect("interpreter parity render")
            })
            .debug_tree();
    }
    assert_eq!(automatic_tree, interpreted_tree);
    assert_eq!(
        automatic_renders,
        interpreter.read_metrics().script_renders()
    );

    let jit = runtime.jit_metrics_for_benchmark();
    println!(
        "[A/JIT] enabled={} queued={} failures={} unsupported={} tier1_rejections={} \
         resource_limits={} cancelled={} panics={} invalid_artifacts={} install_failures={} \
         installed={} native_entries={} pending_jobs={}",
        jit.native_enabled(),
        jit.queued,
        jit.compile_failures,
        jit.unsupported_opcode_failures,
        jit.tier1_rejections,
        jit.resource_limit_failures,
        jit.cancelled_compilations,
        jit.compiler_panics,
        jit.invalid_artifacts,
        jit.install_failures,
        jit.installed,
        jit.native_entries,
        jit.pending_worker_jobs,
    );
    let categorized_failures = jit
        .unsupported_opcode_failures
        .saturating_add(jit.tier1_rejections)
        .saturating_add(jit.resource_limit_failures)
        .saturating_add(jit.cancelled_compilations)
        .saturating_add(jit.compiler_panics)
        .saturating_add(jit.invalid_artifacts)
        .saturating_add(jit.install_failures);
    assert!(
        jit.compile_failures == 0 || categorized_failures > 0,
        "aggregate JIT failures had no diagnostic category: {jit:?}"
    );

    // A smoke bound, not the real gate: the doc's 1.5 ms budget is for a
    // release build, and this assertion has to hold in debug too.
    assert!(
        per_build.as_millis() < 200,
        "describing {nodes} nodes took {per_build:?}, which is far outside any budget"
    );
}

#[gpui::test]
#[cfg(not(debug_assertions))]
fn numeric_layout_installs_and_enters_native_code(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = runtime_with_source(cx, COMPUTE_TEMPLATE, false);
    let mut tree = String::new();
    let mut automatic_renders = 0;

    for _ in 0..1_000 {
        let snapshot = context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("numeric render")
        });
        tree = snapshot.debug_tree();
        automatic_renders += 1;
        if runtime.jit_metrics_for_benchmark().native_entries > 0 {
            break;
        }
    }

    let (interpreter, mut interpreter_context, interpreter_object) =
        runtime_with_source(cx, COMPUTE_TEMPLATE, true);
    let mut interpreted_tree = String::new();
    for _ in 0..automatic_renders {
        interpreted_tree = interpreter_context
            .update(|window, cx| {
                interpreter
                    .build_snapshot(
                        &interpreter_object,
                        None,
                        crate::policy::default(),
                        window,
                        cx,
                    )
                    .expect("interpreter numeric render")
            })
            .debug_tree();
    }

    let jit = runtime.jit_metrics_for_benchmark();
    println!("\n[N/JIT] {jit:#?}");
    assert_eq!(tree, interpreted_tree);
    assert_eq!(
        runtime.read_metrics().script_renders(),
        interpreter.read_metrics().script_renders()
    );
    assert!(tree.contains("layout:165580141"), "{tree}");
    assert!(
        jit.installed > 0,
        "numeric layout installed no artifact: {jit:?}"
    );
    assert!(
        jit.native_entries > 0,
        "numeric layout never entered native code: {jit:?}"
    );
}

#[gpui::test]
#[cfg(not(debug_assertions))]
fn mixed_market_panel_matches_interpreter_and_enters_native_code(cx: &mut TestAppContext) {
    let (interpreter, mut interpreter_context, interpreter_object) =
        runtime_with_source(cx, MIXED_MARKET_TEMPLATE, true);
    let interpreter_tree = interpreter_context.update(|window, cx| {
        interpreter
            .build_snapshot(
                &interpreter_object,
                None,
                crate::policy::default(),
                window,
                cx,
            )
            .expect("interpreter mixed render")
            .debug_tree()
    });

    let (automatic, mut automatic_context, automatic_object) =
        runtime_with_source(cx, MIXED_MARKET_TEMPLATE, false);
    let mut automatic_tree = String::new();
    for _ in 0..JIT_WARMUP_RENDERS {
        automatic_tree = automatic_context.update(|window, cx| {
            automatic
                .build_snapshot(
                    &automatic_object,
                    None,
                    crate::policy::default(),
                    window,
                    cx,
                )
                .expect("JIT mixed render")
                .debug_tree()
        });
        cx.run_until_parked();
    }

    assert_eq!(automatic_tree, interpreter_tree);
    let metrics = automatic.jit_metrics_for_benchmark();
    assert!(metrics.native_entries > 0, "{metrics:?}");
    assert_eq!(metrics.native_fallbacks, 0, "{metrics:?}");
    assert_eq!(metrics.deopts, 0, "{metrics:?}");
}

#[gpui::test]
#[cfg(not(debug_assertions))]
fn first_render_defers_jit_profiling_until_the_view_is_warm(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = runtime_with_source(cx, COMPUTE_TEMPLATE, false);

    context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .expect("first numeric render")
    });
    let first = runtime.jit_metrics_for_benchmark();
    assert_eq!(first.snapshot_requests, 0, "{first:?}");

    for _ in 0..1_000 {
        context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("warm numeric render")
        });
        if runtime.jit_metrics_for_benchmark().native_entries > 0 {
            break;
        }
    }
    let warm = runtime.jit_metrics_for_benchmark();
    assert!(warm.snapshot_requests > 0, "{warm:?}");
    assert!(warm.native_entries > 0, "{warm:?}");
}

#[gpui::test]
fn materializing_a_snapshot_stays_inside_the_frame_budget(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = grid(cx, ROWS, COLUMNS);

    let snapshot = context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .expect("render")
    });
    let nodes = snapshot.len();

    // This is the number that belongs to the frame budget: no VM runs here, and
    // it is what a repaint of an unchanged view actually costs.
    let per_materialize = time_materializations(&mut context, &runtime, &snapshot, ITERATIONS);

    println!(
        "\n[B] snapshot → elements: {nodes} nodes — {:.3} ms per materialization \
         ({ITERATIONS} iterations)",
        per_materialize.as_secs_f64() * 1000.0,
    );

    assert!(
        per_materialize.as_millis() < 200,
        "materializing {nodes} nodes took {per_materialize:?}, which is far outside any budget"
    );
}

#[gpui::test]
fn repainting_a_clean_view_never_enters_the_vm(cx: &mut TestAppContext) {
    let (runtime, mut context, object) = grid(cx, ROWS, COLUMNS);
    let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));

    draw(&mut context, &view);
    let after_first_frame = runtime.metrics().read().script_renders();

    let started = Instant::now();
    for _ in 0..ITERATIONS {
        draw(&mut context, &view);
    }
    let per_frame = started.elapsed() / ITERATIONS as u32;

    println!(
        "\n[C] cached frames: {ITERATIONS} repaints — {:.3} ms per frame, \
         {} script renders\n",
        per_frame.as_secs_f64() * 1000.0,
        runtime.metrics().read().script_renders() - after_first_frame,
    );

    assert_eq!(
        runtime.metrics().read().script_renders(),
        after_first_frame,
        "{ITERATIONS} repaints of a clean view entered the VM; script cost is back on the \
         frame budget"
    );
}

/// What all three costs do as a panel grows.
///
/// A single size cannot answer the question the design actually asks, because
/// two of these costs scale and one assertion does not. A, B and C all grow
/// roughly linearly with the node count — but the script render count stays at
/// zero at every size, which is the property the snapshot exists to buy. It is
/// ignored by default because the largest size costs seconds in a debug build:
///
/// ```text
/// cargo test -p gpui-shell --release tests::benchmark -- --ignored --nocapture
/// ```
///
/// Two runs out of thirteen have seen the largest size report one script render
/// rather than zero, and it has not reproduced since — not under CPU load, not
/// with per-frame instrumentation, which is itself a hint that it is a timing
/// window. If it returns, the thing to instrument is `invalidate`: the deferred
/// drain in `scheduler::drain_after_render` is the only path that reaches a view
/// between frames, and the default-size test above has never failed.
#[gpui::test]
#[ignore = "walks panel sizes up to 8403 nodes; run explicitly in release"]
fn every_size_pays_the_script_only_when_it_changes(cx: &mut TestAppContext) {
    println!(
        "\n{:>6} | {:>12} | {:>12} | {:>12} | {}",
        "nodes", "A build", "B materialize", "C frame", "script renders"
    );

    for (rows, columns) in SIZES {
        let (runtime, mut context, object) = grid(cx, rows, columns);

        let nodes = context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render")
                .len()
        });

        let started = Instant::now();
        let snapshot = context.update(|window, cx| {
            let mut last = None;
            for _ in 0..ITERATIONS {
                last = Some(
                    runtime
                        .build_snapshot(&object, None, crate::policy::default(), window, cx)
                        .expect("render"),
                );
            }
            last.expect("snapshot")
        });
        let per_build = started.elapsed() / ITERATIONS as u32;

        let per_materialize = time_materializations(&mut context, &runtime, &snapshot, ITERATIONS);

        let view = context.update(|_, cx| cx.new(|_| ScriptView::new(runtime.clone(), object)));
        draw(&mut context, &view);
        let after_first_frame = runtime.metrics().read().script_renders();

        let started = Instant::now();
        for _ in 0..ITERATIONS {
            draw(&mut context, &view);
        }
        let per_frame = started.elapsed() / ITERATIONS as u32;
        let renders = runtime.metrics().read().script_renders() - after_first_frame;

        println!(
            "{nodes:>6} | {:>9.2} ms | {:>9.2} ms | {:>9.2} ms | {renders}",
            per_build.as_secs_f64() * 1000.0,
            per_materialize.as_secs_f64() * 1000.0,
            per_frame.as_secs_f64() * 1000.0,
        );

        assert_eq!(
            renders, 0,
            "{ITERATIONS} repaints of a clean {nodes}-node view entered the VM"
        );
    }
    println!();
}

fn time_materializations(
    context: &mut VisualTestContext,
    runtime: &std::rc::Rc<ShellRuntime>,
    snapshot: &RenderSnapshot,
    iterations: usize,
) -> std::time::Duration {
    // Elements are arena-allocated and only live inside a draw, so the timing
    // runs inside one — measuring materialization alone, with layout and paint
    // outside the clock.
    let mut elapsed = std::time::Duration::MAX;
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        |window, cx| {
            // Best of several batches, for the reason given in [A].
            for _ in 0..ROUNDS {
                let started = Instant::now();
                for _ in 0..iterations {
                    let element = materialize(runtime, snapshot, window, cx);
                    std::hint::black_box(&element);
                }
                elapsed = elapsed.min(started.elapsed());
            }
            gpui::div().into_any_element()
        },
    );
    elapsed / iterations as u32
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(800.), gpui::px(600.)),
        move |_, _| view.into_any_element(),
    );
}

/// The grid application, instantiated in a window.
fn grid(
    cx: &mut TestAppContext,
    rows: usize,
    columns: usize,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    crate::engine::ViewObject,
) {
    runtime_with_source(cx, &source(rows, columns), false)
}

fn runtime_with_source(
    cx: &mut TestAppContext,
    source: &str,
    interpreter: bool,
) -> (
    std::rc::Rc<ShellRuntime>,
    VisualTestContext,
    crate::engine::ViewObject,
) {
    cx.update(|cx| crate::init(cx));

    let runtime = if interpreter {
        ShellRuntime::new_isolated_interpreter().expect("interpreter runtime")
    } else {
        ShellRuntime::new_isolated().expect("automatic JIT runtime")
    };
    cx.update(|cx| runtime.set_global(cx));

    let view_type = runtime.load_source("benchmark-view", source).expect("load");

    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let object = context
        .update(|window, cx| runtime.instantiate(&view_type, window, cx))
        .expect("instantiate");

    (runtime, context, object)
}

/// Emits one independent process sample for the Issue #3 acceptance runner.
/// The runner launches this test in interleaved interpreter/automatic pairs;
/// repeated timings inside this process are observations, not independent
/// benchmark samples.
#[gpui::test]
#[cfg(not(debug_assertions))]
fn emit_one_jit_acceptance_sample(cx: &mut TestAppContext) {
    let Ok(path) = std::env::var("GPUI_SHELL_JIT_SAMPLE") else {
        return;
    };
    let interpreter = match std::env::var("GPUI_SHELL_JIT_MODE").as_deref() {
        Ok("interpreter") => true,
        Ok("automatic") => false,
        _ => panic!("GPUI_SHELL_JIT_MODE must be interpreter or automatic"),
    };
    let pair_index: usize = std::env::var("GPUI_SHELL_JIT_PAIR")
        .expect("GPUI_SHELL_JIT_PAIR")
        .parse()
        .expect("numeric pair index");
    let workload = std::env::var("GPUI_SHELL_JIT_WORKLOAD").unwrap_or_else(|_| "panel".into());
    let benchmark_source = match workload.as_str() {
        "panel" => source(ROWS, COLUMNS),
        "compute" => COMPUTE_TEMPLATE.to_owned(),
        "mixed" => MIXED_MARKET_TEMPLATE.to_owned(),
        _ => panic!("GPUI_SHELL_JIT_WORKLOAD must be panel, compute, or mixed"),
    };

    let first_started = Instant::now();
    let (runtime, mut context, object) = runtime_with_source(cx, &benchmark_source, interpreter);
    let first = context.update(|window, cx| {
        runtime
            .build_snapshot(&object, None, crate::policy::default(), window, cx)
            .expect("first render")
    });
    let first_window_ns = first_started.elapsed().as_nanos() as u64;
    let expected_tree = first.debug_tree();
    cx.run_until_parked();

    for _ in 0..JIT_WARMUP_RENDERS {
        let snapshot = context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("warmup render")
        });
        assert_eq!(snapshot.debug_tree(), expected_tree);
    }

    let mut render_ns = Vec::with_capacity(ACCEPTANCE_ITERATIONS);
    let steady_started = Instant::now();
    for _ in 0..ACCEPTANCE_ITERATIONS {
        let started = Instant::now();
        let snapshot = context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("measured render")
        });
        assert_eq!(snapshot.debug_tree(), expected_tree);
        render_ns.push(started.elapsed().as_nanos() as u64);
    }
    let steady_state_ns = steady_started.elapsed().as_nanos() as u64 / ACCEPTANCE_ITERATIONS as u64;
    let p99_script_render_ns = p99(render_ns);
    let metrics = runtime.jit_metrics_for_benchmark();
    assert_eq!(
        metrics.native_enabled(),
        !interpreter,
        "interpreter samples must not attach a JIT backend: {metrics:?}"
    );

    let mut reload_totals = Vec::with_capacity(RELOAD_OBSERVATIONS);
    let mut reload_observations = Vec::with_capacity(RELOAD_OBSERVATIONS);
    let mut reloaded_len = 0;
    for observation in 0..RELOAD_OBSERVATIONS {
        let reload_started = Instant::now();
        let source_started = Instant::now();
        let reload_type = runtime
            .load_source(
                &format!("benchmark-view-reload-{observation}"),
                &benchmark_source,
            )
            .expect("reload source");
        let source_eval_ns = source_started.elapsed().as_nanos() as u64;
        let instantiate_started = Instant::now();
        let reload_object = context
            .update(|window, cx| runtime.instantiate(&reload_type, window, cx))
            .expect("reload instantiate");
        let instantiate_ns = instantiate_started.elapsed().as_nanos() as u64;
        let render_started = Instant::now();
        let reloaded = context.update(|window, cx| {
            runtime
                .build_snapshot(&reload_object, None, crate::policy::default(), window, cx)
                .expect("reload render")
        });
        let render_ns = render_started.elapsed().as_nanos() as u64;
        let total_ns = reload_started.elapsed().as_nanos() as u64;
        assert_eq!(reloaded.debug_tree(), expected_tree);
        reloaded_len = reloaded.len();
        reload_totals.push(total_ns);
        reload_observations.push(serde_json::json!({
            "source_eval_ns": source_eval_ns,
            "instantiate_ns": instantiate_ns,
            "render_ns": render_ns,
            "total_ns": total_ns,
        }));
    }
    let hot_reload_ns = median_ns(reload_totals);

    let digest = format!("{:x}", Sha256::digest(expected_tree.as_bytes()));
    let sample = serde_json::json!({
        "mode": if interpreter { "interpreter" } else { "automatic" },
        "workload": workload,
        "pair_index": pair_index,
        "steady_state_ns": steady_state_ns,
        "p99_script_render_ns": p99_script_render_ns,
        "first_window_ns": first_window_ns,
        "hot_reload_ns": hot_reload_ns,
        "checksum": format!("{}:{}", reloaded_len, digest),
        "snapshot_sha256": digest,
        "script_renders": runtime.read_metrics().script_renders(),
        "native_enabled": metrics.native_enabled(),
        "native_entries": metrics.native_entries,
        "fallback_count": metrics.native_fallbacks,
        "installed": metrics.installed,
        "compile_failures": metrics.compile_failures,
        "unsupported_opcode_failures": metrics.unsupported_opcode_failures,
        "tier1_rejections": metrics.tier1_rejections,
        "resource_limit_failures": metrics.resource_limit_failures,
        "cancelled_compilations": metrics.cancelled_compilations,
        "compiler_panics": metrics.compiler_panics,
        "invalid_artifacts": metrics.invalid_artifacts,
        "install_failures": metrics.install_failures,
        "native_exits": metrics.native_exits,
        "osr_entries": metrics.osr_entries,
        "deopts": metrics.deopts,
        "reload_observations": reload_observations,
    });
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&sample).expect("serialize sample"),
    )
    .expect("write sample");
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}

// --- What one recorded call costs, and where the cost sits ------------------
//
// [A] prices a whole description. It does not say what to change to make one
// cheaper, because a description is a loop in script around a call that crosses
// into Rust, and those are two costs with two different remedies. [D] separates
// them by walking one call through progressively more of the generic path, then
// prints what the same call costs on the path the prelude actually binds.
//
// Each row is the whole path up to that point, so the difference between two
// adjacent rows is the cost of exactly the piece that row adds. The last row is
// not part of the walk: it is the shipped call, and the distance between it and
// `recorded` is what the dedicated entry points removed.

/// 20,000 recorded calls per round: a little over four times what the [A] panel
/// describes, and long enough that a round is milliseconds rather than
/// microseconds.
const BENCH_ELEMENTS: usize = 2_000;
const BENCH_PER_ELEMENT: usize = 10;

/// One prototype per stage, built over the same `function (...args)` shape the
/// prelude uses for behaviours — so what is timed is the real builder method
/// and not a hand-written approximation of it.
const BENCH_SETUP: &str = r#"
globalThis.__bench = (() => {
  const nothing = () => {};
  const method = (body) => {
    const methods = {};
    methods.m = body;
    return methods;
  };
  const generic = (target, name) =>
    method(function (...args) {
      target(this.__id, name, args);
      return this;
    });

  const nullaryIndex = __nullaryStyleIndexes[__nullaryStyles.indexOf("items_center")];
  const paramIndex = __paramStyles.indexOf("bg");

  const stages = {
    nullary: {
      js: generic(nothing, "items_center"),
      crossing: generic(__benchId, "items_center"),
      name: generic(__benchName, "items_center"),
      arguments: generic(__benchArgs, "items_center"),
      recorded: generic(__apply, "items_center"),
      shipped: method(function () {
        __applyNullaryStyle(this.__id, nullaryIndex);
        return this;
      }),
    },
    parametric: {
      js: generic(nothing, "bg"),
      crossing: generic(__benchId, "bg"),
      name: generic(__benchName, "bg"),
      arguments: generic(__benchArgs, "bg"),
      recorded: generic(__apply, "bg"),
      shipped: method(function (value) {
        __applyParamStyle(this.__id, paramIndex, value);
        return this;
      }),
    },
  };

  // The floor: the loop and the element, with no method call in it at all.
  const floor = (elements, per) => {
    for (let e = 0; e < elements; e += 1) {
      const object = Object.create(stages.nullary.js);
      object.__id = __div();
      for (let i = 0; i < per; i += 1) {
      }
    }
  };
  const bare = (methods, elements, per) => {
    for (let e = 0; e < elements; e += 1) {
      const object = Object.create(methods);
      object.__id = __div();
      for (let i = 0; i < per; i += 1) object.m();
    }
  };
  const valued = (methods, elements, per, value) => {
    for (let e = 0; e < elements; e += 1) {
      const object = Object.create(methods);
      object.__id = __div();
      for (let i = 0; i < per; i += 1) object.m(value);
    }
  };

  return { stages, floor, bare, valued };
})();
"#;

#[gpui::test]
fn one_recorded_call_is_priced_stage_by_stage(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx);
        // What a script render does on its way in. These stages run outside
        // every call scope, matching normal application render setup.
        crate::theme_tokens::sync(cx);
    });
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    runtime.eval_for_benchmark(BENCH_SETUP).expect("setup");

    let calls = BENCH_ELEMENTS * BENCH_PER_ELEMENT;
    let floor = time_stage(
        &runtime,
        &format!("__bench.floor({BENCH_ELEMENTS}, {BENCH_PER_ELEMENT})"),
    );
    let per_call = |elapsed: std::time::Duration| {
        elapsed.saturating_sub(floor).as_nanos() as f64 / calls as f64
    };

    println!(
        "\n[D] one recorded call, {calls} calls per round (best of {ROUNDS})\n\
         {:>11} | {:>12} | {:>12} | {}",
        "stage", "ns per call", "added", "what the step adds"
    );

    let mut shipped = Vec::new();
    for (family, argument, label) in [
        ("nullary", None, "items_center()"),
        ("parametric", Some("\"#f8f8f8\""), "bg(\"#f8f8f8\")"),
    ] {
        println!("{label}:");
        let mut previous = floor;
        for (stage, adds) in [
            ("js", "QuickJS interpreting the builder"),
            ("crossing", "the bare crossing into Rust"),
            ("name", "the method name as a Rust String"),
            (
                "arguments",
                "the argument list, in a JS array, as `Bridged`",
            ),
            ("recorded", "dispatch and the arena write"),
            ("shipped", "— what the prelude binds instead"),
        ] {
            let call = match argument {
                None => format!(
                    "__bench.bare(__bench.stages.{family}.{stage}, {BENCH_ELEMENTS}, \
                     {BENCH_PER_ELEMENT})"
                ),
                Some(value) => format!(
                    "__bench.valued(__bench.stages.{family}.{stage}, {BENCH_ELEMENTS}, \
                     {BENCH_PER_ELEMENT}, {value})"
                ),
            };
            let elapsed = time_stage(&runtime, &call);
            println!(
                "{stage:>11} | {:>9.0} ns | {:>9.0} ns | {adds}",
                per_call(elapsed),
                per_call(elapsed) - per_call(previous),
            );
            if stage == "shipped" {
                shipped.push((label, per_call(previous), per_call(elapsed)));
            }
            previous = elapsed;
        }
    }
    println!();

    // Not a threshold on any of the numbers — those are hardware — but on the
    // shape, which is the claim the table is making: the entry point the
    // prelude binds is the reason a style call is worth what it is worth, and
    // a change that undid it would leave the table saying so while every other
    // test still passed.
    for (label, generic, shipped) in shipped {
        assert!(
            shipped < generic,
            "`{label}` cost {shipped:.0} ns through its own entry point and {generic:.0} ns \
             through the generic one; the dedicated path has stopped paying for itself"
        );
    }
}

fn time_stage(runtime: &std::rc::Rc<ShellRuntime>, call: &str) -> std::time::Duration {
    // The first round pays for the arena's first growth and for QuickJS's
    // inline caches, neither of which a steady-state description pays.
    runtime.eval_for_benchmark(call).expect("stage");
    runtime.reset_arena_for_benchmark();

    let mut best = std::time::Duration::MAX;
    for _ in 0..ROUNDS {
        let started = Instant::now();
        runtime.eval_for_benchmark(call).expect("stage");
        best = best.min(started.elapsed());
        runtime.reset_arena_for_benchmark();
    }
    best
}
