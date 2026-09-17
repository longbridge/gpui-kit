# Styling and Motion

## Scope

This document defines how GPUI runtime interaction styles, base semantic-state
styles, application styles, and motion compose. It describes the current public
contract rather than a future milestone plan.

## Ownership

```text
GPUI
  detects hover, active, focus, and focus-visible state

gpui-base
  defines semantic component states
  resolves semantic-state style precedence
  provides generic value-transition and spring lifecycle

application or gpui-component
  owns all target styles
  owns variants and visual slots
  chooses animated properties and timing
```

The phrase “base owns state” is intentionally avoided. Applications commonly
own controlled values such as `checked`, `selected`, and `open`. Base defines
how those values affect behavior and how their optional semantic styles are
resolved.

## GPUI Interaction Styles

Use GPUI's native modifiers for runtime pseudo-states:

```rust,ignore
element
    .hover(|style| style.bg(hover))
    .active(|style| style.bg(active))
    .focus(|style| style.border_color(focus))
    .focus_visible(|style| style.border_color(ring))
```

GPUI resolves these states at runtime in its own fixed order. Base does not
provide a second hover, active, or focus selector API.

A GPUI pseudo-state has one owner. Repeated registration of modifiers such as
`hover` may assert in debug builds, and base cannot read or merge GPUI's private
interaction refinements from another crate.

## Semantic-State Styles

Semantic states describe component values rather than pointer conditions:

- checked and indeterminate;
- pressed;
- selected;
- focused when it is a component value contract;
- disabled.

Controls expose only states that they can actually enter. For example, a Button
does not expose a checked style and a Slider does not expose an open style.

```rust,ignore
Checkbox::new("terms")
    .checked(checked)
    .disabled(disabled)
    .border_1()
    .styles(|styles| {
        styles
            .checked(|style| style.bg(primary))
            .indeterminate(|style| style.bg(primary))
            .disabled(|style| style.opacity(0.5))
    })
```

`StateStyle` implements GPUI's `Styled` interface and `FluentBuilder`, so state
closures can use normal style methods and helpers such as `when`, `when_some`,
and `when_none`. It is not a separate styling language.

## Style Precedence

Every base control resolves static and semantic styles in this order:

```text
instance style
→ active value states in the component's documented order
→ disabled
→ GPUI runtime interaction refinements
```

Later layers override only the fields they set. Unrelated fields from earlier
layers remain intact.

For a Checkbox, a typical value-state order is:

```text
instance → checked → indeterminate → disabled
```

Normalized checkbox state makes checked and indeterminate mutually exclusive,
but the fixed ordering still keeps resolution deterministic.

The order in which closures are written inside `.styles(...)` does not change
precedence. The component defines the state order and routes it through the
shared resolver.

### Preserving an application style in an active state

If a compatibility component requires a caller-provided style to win over one
semantic state, replay that refinement inside the state closure:

```rust,ignore
Button::new("save")
    .bg(brand)
    .styles(|styles| {
        styles.disabled(|style| style.opacity(0.5).bg(brand))
    })
```

Base cannot infer which fields in one `StyleRefinement` came from component
defaults and which came from a final caller override.

### Disabled interaction appearance

Base controls suppress activation while disabled. GPUI does not expose native
hover and active refinements for base to remove, and those refinements run after
semantic styles. Guard interaction modifiers at the call site when disabled
controls must not react visually:

```rust,ignore
.when(!disabled, |element| {
    element
        .hover(|style| style.bg(hover))
        .active(|style| style.bg(active))
})
```

## Root and Part Styling

Semantic root styles do not automatically traverse into children. Compound
parts are explicit application-owned styling boundaries:

```text
Checkbox / CheckboxIndicator
Switch / SwitchTrack / SwitchThumb
Slider / SliderTrack / SliderIndicator / SliderThumb
```

Each part exposes the semantic state needed to style itself. Applications
should construct and style the parts directly instead of expecting a root style
to mutate arbitrary descendants.

Keep state-independent geometry in the part's ordinary builder chain and put
state-dependent color, border, fill, or opacity in its semantic style context.

## Corner Radius

Every corner an application-owned component draws comes from the theme. An
application that sets `Theme::radius` to zero gets square corners everywhere,
including the elements that read as circles or pills — avatar, badge dot, radio
mark, slider thumb, stepper indicator, pill tab, progress bar.

```text
Theme::radius            the general radius
Theme::radius_lg         dialogs, notifications, large surfaces
Theme::radius_full()     circle or pill; zero when `radius` is zero
ThemeStyled::rounded_full_style(cx)   the same, applied to an element
```

Do not write a literal radius. `rounded_full()`, `rounded_md()`,
`rounded(px(6.))` and friends survive `radius` being set to zero, which leaves
a handful of permanently round elements in a UI that is square everywhere else.
Derive from the theme instead, scaling with `radius.half()` or `radius * 2.`
where a component needs a tighter or looser curve than the base.

The Base layer keeps its own copy of the theme, because it paints the scrollbar
and the resize handles without going through `gpui-component`. `Theme::change`
refreshes that copy; writing to the theme's public fields does not. After
mutating the theme directly, call `Theme::sync_base(cx)` or the scrollbar thumb
keeps the radius it was last given.

Two deliberate exceptions:

- A radius the caller passes in explicitly, such as `Tag::rounded_full()` or
  `Avatar::rounded(px(20.))`. The application asked for that shape.
- Plotted geometry — a scatter marker, a pie slice, a chart data dot. Those are
  data, not chrome, and do not follow the interface's radius.

## Motion Ownership

Ordinary semantic controls do not install default fade, slide, spring, or size
animations. Product motion is presentation and therefore belongs to the
application or the styled component layer.

The base crate provides a generic target-value transition:

```rust,ignore
let opacity = gpui_base::transition(
    ("dialog", "opacity"),
    if open { 1.0 } else { 0.0 },
    gpui_base::Transition::new(Duration::from_millis(160)),
    window,
    cx,
);
```

The transition owns lifecycle mechanics only:

- keyed retained state;
- duration and delay;
- easing;
- animation-frame requests;
- smooth reversal from the currently sampled value;
- reduced-motion handling, against the operating system's preference that
  `gpui_base::init` reads into `App::set_reduce_motion` (see `reduce_motion`).

The caller chooses what the value means and applies it to opacity, color,
geometry, or another interpolatable property.

For a value that can be retargeted while it is still moving, base provides a
spring instead:

```rust,ignore
let left = gpui_base::spring(
    ("tab-indicator", "left"),
    selected_tab_left,
    gpui_base::Spring::new(Duration::from_millis(250))
        .with_damping(0.85)
        .with_epsilon(0.1),
    window,
    cx,
);
```

A spring is keyed, reduced-motion aware, and frame-rate independent in the same
way a transition is. It differs in what it carries across a target change: a
transition restarts its easing from the value sampled at that instant, which is
continuous in position but not in velocity, while a spring preserves velocity
and turns the value around. Prefer a spring where the target changes faster than
the motion completes — a toast stack that reflows as toasts arrive, an indicator
chasing rapid selection changes, a panel toggled again mid-slide — and a
transition where the target is set once and runs to completion.

A value that the pointer is already moving must not be sprung while the drag
lasts, or it trails the cursor. `Spring::with_travel(false)` suspends travel and
keeps the retained state pinned to the target, so the same call site can hand the
value straight through during a drag and resume springing from where the drag
released it:

```rust,ignore
let position = spring(
    id,
    target,
    POSITION_SPRING.with_travel(!dragging),
    window,
    cx,
);
```

Suspending travel says so where the reader is, and says it without disturbing the
response, damping or tolerance the spring is configured with. A zero response
resolves the same way — as a zero duration does for a transition — but it is the
degenerate case rather than the way to express this, and a policy swapped out for
the length of a drag has to restate or discard everything else the original one
carried.

`Spring::new(response)` builds one that reaches its target in about that long
without overshooting it, which is what almost every value wants: a spring
driving an opacity, a measured height, or anything bounded by the geometry
around it has nowhere to overshoot to. `with_damping` opts a value out where
passing the target and coming back is the intended effect.

A response is not a duration in the sense `Transition::new` means one. A spring
has no end to schedule, so this is the scale the motion is felt at rather than
the moment it stops: the last fraction of a percent keeps settling past it,
until it is inside the tolerance below.

The settling tolerance is expressed in the target's own units and defaults to a
normalized `0..1` range; a spring over pixels should coarsen it so the animation
ends when the remaining travel is sub-pixel rather than running frames that
change nothing visible.

Deep behavior modules may own configurable motion when it is required to keep
their internal layout lifecycle coherent. `ToastStack`, for example, combines
measurement, overlap, expansion, and collapse through `ToastMotion`. This does
not give base ownership of toast colors, typography, borders, or content.

`Scrollbar` follows the same rule through `ScrollbarMotion`. A scrollbar is a
custom element that paints its own track and thumb, so no caller can apply an
opacity or offset to it from outside. Base therefore plays the transition, but
owns none of its timing: `ScrollbarMotion::default()` has zero enter, exit, and
expand durations, so an unstyled scrollbar appears and disappears without
motion. The styled layer projects product timing and the entrance choreography
through `ScrollbarTheme::motion`. A zero duration always means "adopt the
target now", which is also how reduced motion and always-visible scrollbars
reach the same code path.

`TextView` follows it through `TextViewMotion`. Streamed text is painted glyph
by glyph inside the view, so only the view can fade the words an update
appended. Base tracks which rendered text is new and samples the fade, but
`TextViewMotion::default()` has zero `stream_fade` and `stream_fade_stagger`
durations; the styled `TextView::stream_fade(true)` projects its own timing
(350 ms per chunk, measured from claude.ai) the way `ScrollbarTheme` carries
the scrollbar's, because a reveal that must overlap a model's chunk cadence
is not one of the four UI transition tiers.

## Transition Identity

A transition ID identifies one independently animated value. Use a stable
element-like ID and a named channel when one component animates multiple values:

```rust,ignore
("checkbox", "indicator-opacity")
("checkbox", "indicator-scale")
```

Do not reuse one ID for different value types or unrelated component instances.
State is keyed within the current GPUI element-state scope.

## Target Changes and Reversal

When a target changes during an active transition, the next transition begins
from the value sampled at that instant. It does not restart from the previous
endpoint. This prevents discontinuities during rapid toggles.

On the first render, the target is adopted immediately. When reduced motion is
enabled or duration is zero, the target is returned immediately and retained
transition state is synchronized with it.

A spring resolves the same three moments differently. A target change keeps both
the current position and the current velocity, so the value decelerates through
the reversal instead of restarting. The first render adopts the target at rest.
Reduced motion snaps to the target and clears the stored velocity.

## Supported Values

`transition` accepts values implementing `Interpolate`, `Clone`, and
`PartialEq`. Types implementing the legacy `animation::Lerp` trait receive an
`Interpolate` implementation automatically.

Applications may implement `Interpolate` for their own value types when the
interpolation is meaningful and deterministic.

Base also implements composite interpolation for `Size<Pixels>`,
`Bounds<Pixels>`, and `MotionTransform`. The transform bundle coordinates
translation, scale, rotation, and opacity while leaving the actual paint
strategy to its caller.

`spring` accepts values implementing GPUI's `SpringTarget`, which projects a
value onto the single scalar coordinate the spring integrates and back again.
GPUI implements it for `f32`, `Pixels`, `Rems`, and `bool`, the last resolving
to an `AnimationPhase` that interpolates between two endpoint values. A value
that needs more than one coordinate — a position, a pair of bounds — uses one
spring per channel rather than one spring over the composite.

## CSS-Aligned Timing and Keyframes

`Easing` provides the CSS keyword curves, typed cubic Bézier curves, all CSS
step positions, and piecewise-linear stops. `Timing` adds signed delay,
finite/infinite iteration counts, and normal, reverse, alternate, and
alternate-reverse playback directions. Both are pure samplers.

`Keyframes<T>` validates endpoint and ordering invariants once, stores its
track behind shared ownership, and binary-searches the active segment when
sampled. Each segment may carry its own easing. `animate_keyframes` combines a
track with `Timing` and keyed GPUI lifecycle state. `Discrete<T>` explicitly
models values that switch rather than interpolate, and `Stagger` calculates
per-item delay without allocating a schedule.

All timing is derived from absolute elapsed time. Frame rate affects how many
samples are painted, not the value reached at a particular time.

## Presence and Measured Reveal

`Presence` retains the enter/present/exit/absent lifecycle independently from
the caller's logical boolean. Its sample reports progress and whether content
must remain mounted. Reentry during exit reverses from the current sample.

`MotionReveal` is a lower-level custom element for vertical measured reveals.
It lays its child out at natural height, reports the progress-scaled height to
its parent, and clips paint and hit testing to the visible region. The styled
`Collapsible::motion_id` facade combines it with the theme's control spring.
The opt-in ID preserves the legacy immediate mount/unmount contract for callers
that do not request motion.

## Sequencing

`Sequence` chains value transitions so that each step starts when the previous
one ends. It is the mechanism for "run B when A completes" — a toast that fades
in, holds, then fades out; a control that overshoots and settles — without a
timer or a second keyed state per step:

```rust,ignore
let opacity = gpui_base::Sequence::new(("toast", "opacity"), 0.0)
    .with_step(1.0, Transition::new(Duration::from_millis(160)))
    .with_step(0.0, Transition::new(Duration::from_millis(200)).delay(Duration::from_secs(3)))
    .sample(window, cx);

if opacity.is_finished() {
    // dismiss
}
```

A sequence carries the lifecycle the single transition has and nothing a
transition does not decide:

- it begins at `from` on the frame it is first sampled and plays once per key;
  to replay it, put an application-owned generation in the ID;
- a step ends at an absolute instant, its start plus its delay and duration, and
  the next step starts at that instant rather than on the frame that noticed
  it, so frame rate changes how many samples are painted, not where a step is
  at a given time. Zero-duration steps complete within the frame that reaches
  them;
- the sample reports the value, the index of the step being played, and a
  `MotionStatus`. `Finished` is reported only once the last step completes; a
  frame that crosses a boundary reports the next step's `Delayed` or `Running`;
- frames are requested only while a step is delayed or running;
- under reduced motion the last target is adopted at once, retained state is
  synchronized with it, and no frame is requested.

A step's target and transition are captured when the step starts. Handing the
step being played a different target — or fewer steps than the one it is on —
restarts the sequence from its first step, from the value sampled at that
instant, which is how a retargeted transition continues from its current value.
Steps the sequence has not reached are read when it reaches them; a change to
an earlier step alone has no effect. A sequence does not reverse: play a
second sequence back to the start under its own key when that is wanted.

`Stagger` composes with it as a delay on the first step, because a stagger is
nothing more than a per-index delay:

```rust,ignore
let stagger = Stagger::new(Duration::from_millis(40), StaggerOrigin::First);
let offset = Sequence::new(("row", index), px(12.))
    .with_step(px(-2.), Transition::new(Duration::from_millis(120)).delay(stagger.delay(index, count)))
    .with_step(px(0.), Transition::new(Duration::from_millis(80)))
    .sample(window, cx);
```

## Product Motion Tokens

`gpui-component::MotionTokens` centralizes styled policy. It contains four
semantic duration tiers, enter/exit/move easing, control/movement springs, and
short/medium travel distances. Styled controls read these tokens instead of
defining local constants. Base remains presentation-neutral and can be used by
another design system with different policy.

## Sampling Budget

The steady timing/easing/keyframe sampling paths allocate nothing. The release
benchmark samples batches of 1,000 values and enforces a `0.10 ms` median
ceiling for scalar timing/easing on the reference machine. It is run with:

```bash
cargo bench -p gpui-base --bench motion
```

At 120 Hz the full application has about 8.33 ms per frame. Motion sampling is
kept well below one tenth of a millisecond so layout, paint, text, and
application work retain nearly the whole budget. This does not make arbitrary
layout animation free: prefer opacity and paint transforms when they express
the same relationship.

## Legacy Element Animation

`motion::Transition` is the preferred API for application-owned target values.
It is distinct from `animation::Transition`, the legacy element-animation API
that applies concrete fade, slide, or size effects.

New component code should prefer value transitions because the presentation
owner explicitly selects the animated property. Existing legacy animation code
may continue to use its module-qualified API.

## Design Invariants

1. Static semantic styling must not require retained animation state.
2. Base controls must remain usable without motion.
3. Base must not choose visual target values for application components.
4. GPUI native pseudo-states remain the only hover, active, and focus styling
   mechanism.
5. Semantic state precedence is fixed by the component, not builder call order.
6. Disabled is the last semantic layer.
7. Part styling is explicit and typed; base does not traverse arbitrary child
   trees to apply styles.
8. Reduced-motion preferences are honored by generic transitions and springs,
   and the operating system's preference is read into GPUI's flag at `init`;
   an application that sets the flag itself is never overridden.
9. Corner radius is derived from the theme, never written as a literal.
