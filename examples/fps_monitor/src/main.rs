//! Overlays the `gpui-fps` HUD on a port of three.js' `webgl_lines_colors`
//! demo: 3D Hilbert curves smoothed with a Catmull-Rom spline, colored by the
//! same three HSL schemes as the original, rotating over a black background.
//!
//! The number of curves is adjustable, which makes it a load knob for watching
//! the frame time trace react.
//!
//! The example deliberately depends only on `gpui` and `gpui-fps`, not on
//! `gpui-component`, to show that the HUD stands on its own.

use std::time::Instant;

use gpui_fps::fps_monitor;
use gpui_kit::*;

/// Matches the original demo: a one-iteration Hilbert curve of 64 control
/// points, resampled at six points each.
const HILBERT_SIZE: f32 = 200.;
const HILBERT_ITERATIONS: u32 = 1;
const SUBDIVISIONS: usize = 6;

/// How far the curve actually reaches from the origin.
///
/// Not `HILBERT_SIZE / 2`: each recursion centers a sub-cell *on a corner* of
/// the cell above it and then extends half a sub-cell further out, so one
/// iteration reaches 1.5x the half size. Scaling against the nominal size
/// instead of this would draw every curve larger than its grid cell.
const HILBERT_EXTENT: f32 = HILBERT_SIZE / 2. * 1.5;

/// Fraction of a grid cell the curve is allowed to fill.
///
/// Two effects push past the nominal size and the margin has to cover both, or
/// neighbouring curves overlap: the spline overshoots its control polygon by
/// about 8%, and the perspective divide magnifies the near face by
/// `EYE_DISTANCE / (EYE_DISTANCE - HILBERT_EXTENT)`, roughly 1.32. Together
/// that is a factor of ~1.43.
const CELL_FILL: f32 = 0.68;

/// Points per drawn path. A path carries one color, so the gradient is built
/// from short runs rather than per-vertex colors, which GPUI has no equivalent
/// for.
const SEGMENT_POINTS: usize = 6;

const CURVE_STEP: usize = 1;
const MAX_CURVES: usize = 48;

/// Distance from the eye to the origin, for the perspective divide.
const EYE_DISTANCE: f32 = 620.;
/// How quickly the view catches up with the cursor.
const CURSOR_EASING: f32 = 0.08;

#[derive(Clone, Copy, Default)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

struct Example {
    /// The spline, shared by every curve on screen.
    points: Vec<Vec3>,
    /// Vertex colors for the demo's three schemes, indexed by scheme.
    palettes: [Vec<Hsla>; 3],
    curves: usize,
    /// Where the view is being pulled to, and where it currently is.
    cursor_tilt: Point<f32>,
    tilt: Point<f32>,
    started: Instant,
}

impl Example {
    fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let points = hilbert_spline();
        let palettes = [
            color_scheme(&points, Scheme::CyanByX),
            color_scheme(&points, Scheme::MagentaByY),
            color_scheme(&points, Scheme::Rainbow),
        ];

        Self {
            points,
            palettes,
            // The original lays out six curves in a 2x3 grid.
            curves: 6,
            cursor_tilt: point(0., 0.),
            tilt: point(0., 0.),
            started: Instant::now(),
        }
    }

    fn render_curves(&self) -> impl IntoElement {
        let points = self.points.clone();
        let palettes = self.palettes.clone();
        let curves = self.curves;
        let tilt = self.tilt;
        let spin = self.started.elapsed().as_secs_f32() * 0.35;

        canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window: &mut Window, _| {
                // Lay the curves out on the squarest grid that fits them.
                let columns = (curves as f32).sqrt().ceil().max(1.) as usize;
                let rows = curves.div_ceil(columns);
                let cell = size(
                    bounds.size.width / columns as f32,
                    bounds.size.height / rows as f32,
                );
                let scale =
                    (cell.width.min(cell.height).as_f32() / (HILBERT_EXTENT * 2.)) * CELL_FILL;

                for index in 0..curves {
                    let column = index % columns;
                    let row = index / columns;
                    let center = point(
                        (bounds.origin.x + cell.width * (column as f32 + 0.5)).as_f32(),
                        (bounds.origin.y + cell.height * (row as f32 + 0.5)).as_f32(),
                    );
                    // Alternating spin direction, as in the original.
                    let direction = if index % 2 == 0 { 1. } else { -1. };
                    let yaw = spin * direction + index as f32 * 0.4 + tilt.x;
                    let pitch = tilt.y;

                    let projected: Vec<Point<f32>> = points
                        .iter()
                        .map(|vertex| project(*vertex, yaw, pitch, center, scale))
                        .collect();
                    paint_gradient_curve(window, &projected, &palettes[index % palettes.len()]);
                }
            },
        )
        .absolute()
        .size_full()
    }

    fn render_load_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let button = |id: &'static str, label: &'static str, delta: isize| {
            div()
                .id(id)
                .px_3()
                .py_1()
                .rounded(px(6.))
                .bg(hsla(0., 0., 1., 0.08))
                .border_1()
                .border_color(hsla(0., 0., 1., 0.16))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.curves = this
                        .curves
                        .saturating_add_signed(delta * CURVE_STEP as isize)
                        .clamp(1, MAX_CURVES);
                    cx.notify();
                }))
        };

        div()
            .absolute()
            .bottom(px(16.))
            .left(px(16.))
            .flex()
            .items_center()
            .gap_2()
            .text_size(px(12.))
            .text_color(hsla(0., 0., 0.75, 1.))
            .child(button("fewer", "− load", -1))
            .child(format!("{} curves", self.curves))
            .child(button("more", "+ load", 1))
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The HUD only drives redraws of its own subtree, so the scene has to
        // keep asking for frames to stay animated.
        window.request_animation_frame();

        // Ease toward the cursor rather than snapping, the way the original
        // demo drifts its camera.
        self.tilt.x += (self.cursor_tilt.x - self.tilt.x) * CURSOR_EASING;
        self.tilt.y += (self.cursor_tilt.y - self.tilt.y) * CURSOR_EASING;

        div()
            .relative()
            .size_full()
            .bg(black())
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, _| {
                let viewport = window.viewport_size();
                // A zero-sized viewport would divide to infinity here, and the
                // easing below would then keep the tilt non-finite forever.
                if viewport.width <= px(0.) || viewport.height <= px(0.) {
                    return;
                }
                // No notify: the scene already redraws every frame.
                this.cursor_tilt = point(
                    (event.position.x / viewport.width - 0.5) * 2.4,
                    (event.position.y / viewport.height - 0.5) * 1.2,
                );
            }))
            .child(self.render_curves())
            .child(self.render_load_controls(cx))
            .child(fps_monitor(window, cx))
    }
}

/// Paints one curve as short runs of constant color, approximating the
/// per-vertex gradient of the original.
fn paint_gradient_curve(window: &mut Window, projected: &[Point<f32>], colors: &[Hsla]) {
    let mut start = 0;
    while start + 1 < projected.len() {
        let end = (start + SEGMENT_POINTS).min(projected.len() - 1);
        let run = &projected[start..=end];
        // The tessellator asserts on non-finite coordinates, so a run that
        // picked one up is dropped rather than aborting the process.
        if run.iter().any(|v| !v.x.is_finite() || !v.y.is_finite()) {
            start = end;
            continue;
        }

        let mut path = PathBuilder::stroke(px(1.));
        path.move_to(point(px(run[0].x), px(run[0].y)));
        for vertex in &run[1..] {
            path.line_to(point(px(vertex.x), px(vertex.y)));
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, colors[(start + end) / 2]);
        }
        // Share the boundary vertex so runs join without a gap.
        start = end;
    }
}

/// Rotates around Y then X and applies a perspective divide.
fn project(vertex: Vec3, yaw: f32, pitch: f32, center: Point<f32>, scale: f32) -> Point<f32> {
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let x = vertex.x * cos_yaw + vertex.z * sin_yaw;
    let z = vertex.z * cos_yaw - vertex.x * sin_yaw;

    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let y = vertex.y * cos_pitch - z * sin_pitch;
    let z = z * cos_pitch + vertex.y * sin_pitch;

    // Guard the divide: a vertex level with the eye would blow up.
    let depth = (EYE_DISTANCE + z).max(1.);
    let perspective = EYE_DISTANCE / depth;
    point(
        center.x + x * perspective * scale,
        center.y + y * perspective * scale,
    )
}

#[derive(Clone, Copy)]
enum Scheme {
    CyanByX,
    MagentaByY,
    Rainbow,
}

/// The three vertex color schemes from the original demo.
fn color_scheme(points: &[Vec3], scheme: Scheme) -> Vec<Hsla> {
    points
        .iter()
        .enumerate()
        .map(|(index, vertex)| match scheme {
            Scheme::CyanByX => hsla(0.6, 1., (-vertex.x / 200.).max(0.) + 0.5, 1.),
            Scheme::MagentaByY => hsla(0.9, 1., (-vertex.y / 200.).max(0.) + 0.5, 1.),
            Scheme::Rainbow => hsla(index as f32 / points.len() as f32, 1., 0.5, 1.),
        })
        .collect()
}

/// A Hilbert curve resampled through a Catmull-Rom spline, matching the
/// original demo's geometry.
fn hilbert_spline() -> Vec<Vec3> {
    let mut control = Vec::new();
    hilbert3d(
        Vec3::default(),
        HILBERT_SIZE,
        HILBERT_ITERATIONS,
        [0, 1, 2, 3, 4, 5, 6, 7],
        &mut control,
    );

    let samples = control.len() * SUBDIVISIONS;
    (0..=samples)
        .map(|index| catmull_rom(&control, index as f32 / samples as f32))
        .collect()
}

/// Port of three.js' `hilbert3D`.
fn hilbert3d(center: Vec3, size: f32, iterations: u32, v: [usize; 8], out: &mut Vec<Vec3>) {
    let half = size / 2.;
    let corners = [
        Vec3::new(center.x - half, center.y + half, center.z - half),
        Vec3::new(center.x - half, center.y + half, center.z + half),
        Vec3::new(center.x - half, center.y - half, center.z + half),
        Vec3::new(center.x - half, center.y - half, center.z - half),
        Vec3::new(center.x + half, center.y - half, center.z - half),
        Vec3::new(center.x + half, center.y - half, center.z + half),
        Vec3::new(center.x + half, center.y + half, center.z + half),
        Vec3::new(center.x + half, center.y + half, center.z - half),
    ];
    let vec = v.map(|index| corners[index]);

    let Some(iterations) = iterations.checked_sub(1) else {
        out.extend_from_slice(&vec);
        return;
    };

    let [v0, v1, v2, v3, v4, v5, v6, v7] = v;
    let children = [
        [v0, v3, v4, v7, v6, v5, v2, v1],
        [v0, v7, v6, v1, v2, v5, v4, v3],
        [v0, v7, v6, v1, v2, v5, v4, v3],
        [v2, v3, v0, v1, v6, v7, v4, v5],
        [v2, v3, v0, v1, v6, v7, v4, v5],
        [v4, v3, v2, v5, v6, v1, v0, v7],
        [v4, v3, v2, v5, v6, v1, v0, v7],
        [v6, v5, v2, v1, v0, v3, v4, v7],
    ];
    for (child, order) in vec.iter().zip(children) {
        hilbert3d(*child, half, iterations, order, out);
    }
}

/// Centripetal Catmull-Rom evaluation over the whole control polygon, with the
/// endpoints clamped.
///
/// Centripetal (the `alpha = 0.5` knot parameterization) rather than uniform,
/// matching `CatmullRomCurve3`'s default, because a Hilbert curve turns a
/// corner at nearly every control point. Uniform parameterization overshoots
/// hard at those turns — the curve shoots away from its control polygon and
/// loops back — which reads as long straight spikes across the scene.
/// Centripetal is provably free of both overshoot and cusps.
fn catmull_rom(control: &[Vec3], t: f32) -> Vec3 {
    if control.is_empty() {
        return Vec3::default();
    }
    if control.len() == 1 {
        return control[0];
    }

    let spans = control.len() - 1;
    let scaled = t.clamp(0., 1.) * spans as f32;
    let span = (scaled as usize).min(spans - 1);
    let local = scaled - span as f32;

    let at = |index: isize| control[(index.clamp(0, spans as isize)) as usize];
    let (p0, p1, p2, p3) = (
        at(span as isize - 1),
        at(span as isize),
        at(span as isize + 1),
        at(span as isize + 2),
    );

    // Knots spaced by the square root of the chord length. Coincident control
    // points — which the clamped endpoints always produce — would collapse a
    // span to zero width, so each step is floored. The floor is well above
    // `f32::EPSILON`: a span that small makes the divisions below overflow into
    // the millions and lose all precision.
    let knot = |from: Vec3, to: Vec3| distance3(from, to).sqrt().max(1e-3);
    let t0 = 0.;
    let t1 = t0 + knot(p0, p1);
    let t2 = t1 + knot(p1, p2);
    let t3 = t2 + knot(p2, p3);
    let t = t1 + (t2 - t1) * local;

    // Barry-Goldman pyramid: three lerps, then two, then one.
    let a1 = lerp3(p0, p1, (t - t0) / (t1 - t0));
    let a2 = lerp3(p1, p2, (t - t1) / (t2 - t1));
    let a3 = lerp3(p2, p3, (t - t2) / (t3 - t2));
    let b1 = lerp3(a1, a2, (t - t0) / (t2 - t0));
    let b2 = lerp3(a2, a3, (t - t1) / (t3 - t1));
    lerp3(b1, b2, (t - t1) / (t2 - t1))
}

fn distance3(a: Vec3, b: Vec3) -> f32 {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn lerp3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

actions!(fps_monitor, [Quit]);

fn main() {
    gpui_kit::application().run(move |cx: &mut App| {
        cx.bind_keys([
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-q", Quit, None),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("alt-f4", Quit, None),
        ]);
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

        // This is a single-window demo, so closing the window ends it rather
        // than leaving a headless process behind.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        cx.activate(true);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                window.activate_window();
                window.set_window_title("FPS Monitor");
                cx.new(|cx| Example::new(window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
