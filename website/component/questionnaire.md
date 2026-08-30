---
title: Questionnaire
description: A composable multi-step questionnaire with choice, freeform, validation, and navigation support.
---

# Questionnaire

`Questionnaire` guides a user through an ordered set of questions. It owns the
active item, answer state, validation, progress, and navigation. A containing
page, `GroupBox`, `Dialog`, or `Sheet` remains responsible for closing,
cancelling, persistence, transport, and application-specific branching.

## Import

```rust
use gpui_component::questionnaire::{
    Questionnaire, QuestionnaireActions, QuestionnaireChoice,
    QuestionnaireChoiceDescription, QuestionnaireChoices, QuestionnaireDescription,
    QuestionnaireError, QuestionnaireInput, QuestionnaireItem, QuestionnaireNext,
    QuestionnairePrevious, QuestionnaireProgress, QuestionnaireSkip, QuestionnaireState,
    QuestionnaireSubmit, QuestionnaireTitle,
};
```

## Usage

Create the item collection once and use one `QuestionnaireState` entity as the
source of truth for all parts.

```rust
use gpui_component::input::InputState;
use gpui_component::questionnaire::{
    QuestionnaireChoiceDefinition, QuestionnaireInputDefinition,
    QuestionnaireItemDefinition, QuestionnaireState,
};

let direction_input = cx.new(|cx| {
    InputState::new(window, cx).placeholder("Type another answer…")
});

let items = vec![
    QuestionnaireItemDefinition::new("direction", "What should we prototype next?")
        .with_required(true)
        .with_description("Choose a direction or write your own.")
        .with_choices([
            QuestionnaireChoiceDefinition::new("delegation", "Delegation")
                .with_description("Show how work moves to a specialist."),
            QuestionnaireChoiceDefinition::new("questions", "Question prompts"),
            QuestionnaireChoiceDefinition::new("both", "Both together"),
        ])
        .with_input(QuestionnaireInputDefinition::new(
            direction_input,
            "Another answer",
        )),
    QuestionnaireItemDefinition::new("detail", "How much detail should it include?")
        .with_description("You can skip this question if you are not sure yet.")
        .with_choices([
            QuestionnaireChoiceDefinition::new("focused", "Focused"),
            QuestionnaireChoiceDefinition::new("complete", "Complete flow"),
        ]),
];

let state = cx.new(|cx| {
    QuestionnaireState::new(items, cx)
        .expect("valid questionnaire schema")
});
```

Map every definition in the collection into the compound parts. The active
`QuestionnaireItem` is the only item rendered, so omitting an item from this
composition leaves the UI empty when navigation reaches that item.

```rust
Questionnaire::new(&state)
    .child(QuestionnaireProgress::new(&state))
    .child(
        QuestionnaireItem::new(&state, "direction")
            .child(QuestionnaireTitle::new(&state, "direction"))
            .child(QuestionnaireDescription::new(&state, "direction"))
            .child(
                QuestionnaireChoices::new(&state, "direction")
                    .child(QuestionnaireChoice::new(&state, "direction", "delegation"))
                    .child(QuestionnaireChoice::new(&state, "direction", "questions"))
                    .child(QuestionnaireChoice::new(&state, "direction", "both"))
                    .child(QuestionnaireInput::new(&state, "direction")),
            )
            .child(QuestionnaireError::new(&state, "direction")),
    )
    .child(
        QuestionnaireItem::new(&state, "detail")
            .child(QuestionnaireTitle::new(&state, "detail"))
            .child(QuestionnaireDescription::new(&state, "detail"))
            .child(
                QuestionnaireChoices::new(&state, "detail")
                    .child(QuestionnaireChoice::new(&state, "detail", "focused"))
                    .child(QuestionnaireChoice::new(&state, "detail", "complete")),
            )
            .child(QuestionnaireError::new(&state, "detail")),
    )
    .child(
        QuestionnaireActions::new(&state)
            .child(QuestionnairePrevious::new(&state))
            .child(QuestionnaireSkip::new(&state))
            .child(QuestionnaireNext::new(&state))
            .child(QuestionnaireSubmit::new(&state)),
    )
```

## Composition

```text
Questionnaire
├── QuestionnaireProgress
├── QuestionnaireItem
│   ├── QuestionnaireTitle
│   ├── QuestionnaireDescription
│   ├── QuestionnaireChoices
│   │   ├── QuestionnaireChoice
│   │   │   └── QuestionnaireChoiceDescription (custom child)
│   │   └── QuestionnaireInput
│   └── QuestionnaireError
└── QuestionnaireActions
    ├── QuestionnairePrevious
    ├── QuestionnaireSkip
    ├── QuestionnaireNext
    └── QuestionnaireSubmit
```

Every part accepts ordinary GPUI styling and can be composed with existing
`Button`, `Input`, `Radio`, `Checkbox`, `Progress`, `Stepper`, `GroupBox`, and
`Dialog` elements. Pass the same state entity to each part. A custom part
should read its corresponding state and call state methods for user actions;
it should not create a second answer store.

`QuestionnaireChoice` supplies the default indicator, content, and shortcut.
Adding children replaces the fallback label and description while preserving
choice activation, focus, state, and accessibility behavior. Use
`QuestionnaireChoiceDescription::new()` for secondary text in a custom choice
body. The following seams customize only the corresponding region:

```rust
use gpui::{IntoElement as _, ParentElement as _, StyleRefinement, Styled as _, div};
use gpui_component::{ActiveTheme as _, StyledExt as _};
use gpui_component::questionnaire::{
    QuestionnaireChoice, QuestionnaireChoiceDescription,
};

let _styled_choice = QuestionnaireChoice::new(&state, "direction", "questions")
    .indicator_style(StyleRefinement::default().opacity(0.9))
    .content_style(StyleRefinement::default().opacity(0.95))
    .shortcut_style(StyleRefinement::default().opacity(0.8));

let _rendered_choice = QuestionnaireChoice::new(&state, "direction", "delegation")
    .render_indicator(|choice, _, cx| {
        div()
            .size_4()
            .rounded_full()
            .bg(if choice.is_selected() {
                cx.theme().primary
            } else {
                cx.theme().muted
            })
            .into_any_element()
    })
    .child(
        div()
            .child("Delegation")
            .child(QuestionnaireChoiceDescription::new().child(
                "Show how work moves to a specialist.",
            )),
    );
```

`render_shortcut` has the same renderer signature and receives the
`QuestionnaireChoiceState`; use it when an application wants to replace the
default `Kbd` hint. A renderer replaces that region completely, so its matching
style seam is not applied; style the custom renderer directly. The state
snapshot exposes `is_selected`, `is_disabled`, `is_invalid`, and `shortcut` for
custom rendering.

## Single selection

An item uses single selection by default. Activating a choice answers the item
and makes `Next` available. A single-choice item may also provide a freeform
input. The fixed choice and freeform answer are mutually exclusive, while the
input draft remains available when the user changes their mind.

```rust
let plan_input = cx.new(|cx| InputState::new(window, cx));
let item = QuestionnaireItemDefinition::new("plan", "Which plan fits your team?")
    .with_choices([
        QuestionnaireChoiceDefinition::new("plus", "Plus"),
        QuestionnaireChoiceDefinition::new("pro", "Pro"),
    ])
    .with_input(QuestionnaireInputDefinition::new(plan_input, "Another plan"));
```

## Multiple selection

Set `multiple` on an item when more than one fixed answer is valid. A non-empty
freeform input can be included with the selected fixed choices.

```rust
let tools_input = cx.new(|cx| InputState::new(window, cx));
let item = QuestionnaireItemDefinition::new("tools", "Which tools do you use?")
    .with_multiple(true)
    .with_choices([
        QuestionnaireChoiceDefinition::new("editor", "Editor"),
        QuestionnaireChoiceDefinition::new("terminal", "Terminal"),
        QuestionnaireChoiceDefinition::new("browser", "Browser"),
    ])
    .with_input(QuestionnaireInputDefinition::new(tools_input, "Something else"));
```

The answer reader preserves schema order. If a selected choice is disabled
later, it is excluded from the effective answer.

## Freeform answer

Add `QuestionnaireInputDefinition` to allow a user to enter an answer that is
not in the fixed choices. Give the input an accessible label; a placeholder is
not a label.

```rust
let feedback_input = cx.new(|cx| {
    InputState::new(window, cx).placeholder("Tell us what would help…")
});
let item = QuestionnaireItemDefinition::new("feedback", "What should we improve?")
    .with_input(QuestionnaireInputDefinition::new(
        feedback_input,
        "Your suggestion",
    ));
```

Whitespace-only input is unanswered. The input draft is kept when a fixed
choice is selected, but it is submitted only when the freeform answer is
active. In a multiple item, a non-empty freeform answer can accompany fixed
choices.

## Explicit skip

Optional items can expose `QuestionnaireSkip`. A skip is an intentional valid
state, clears the item answer, and allows `Next` to continue. Required items do
not allow skipping. Re-entering an item and choosing an answer clears its
skipped state. Skipping the final enabled item requests submission after the
skip has been recorded.

```rust
let optional = QuestionnaireItemDefinition::new("tone", "What tone should we use?")
    .with_required(false)
    .with_choices([
        QuestionnaireChoiceDefinition::new("direct", "Direct"),
        QuestionnaireChoiceDefinition::new("warm", "Warm"),
    ]);
```

## Defaults and disabled controls

Use definition builders for the initial snapshot. A choice can start selected,
an item or choice can start disabled, and an input can start disabled. A
single-choice item may contain at most one default selected choice.

```rust
let saved_input = cx.new(|cx| InputState::new(window, cx).default_value("Saved draft"));
let item = QuestionnaireItemDefinition::new("workspace", "Which workspaces?")
    .with_multiple(true)
    .with_choices([
        QuestionnaireChoiceDefinition::new("personal", "Personal")
            .with_default_selected(true),
        QuestionnaireChoiceDefinition::new("team", "Team")
            .with_disabled(true),
    ])
    .with_input(
        QuestionnaireInputDefinition::new(saved_input, "Another workspace")
            .with_disabled(false),
    );
let disabled_item = QuestionnaireItemDefinition::new(
    "advanced",
    "Advanced preferences",
)
    .with_disabled(true);
let disabled_input = cx.new(|cx| InputState::new(window, cx));
let disabled_input_definition = QuestionnaireInputDefinition::new(
    disabled_input,
    "Disabled answer",
)
    .with_disabled(true);
```

`with_default_selected` belongs to `QuestionnaireChoiceDefinition`; an item
uses `with_disabled`, and an input uses
`QuestionnaireInputDefinition::with_disabled`. For an initially disabled item,
use `QuestionnaireItemDefinition::with_disabled(true)`.

`QuestionnaireState::new` rejects duplicate item names, duplicate choice
values within an item, and multiple defaults on a single-choice item. Setters
for unknown items or choices return `QuestionnaireStateError`.

## Navigation and status

`QuestionnaireState` exposes the current item, ordered item states, and
navigation state for custom action layouts.

```rust
let current = state.read(cx).current_item();
let current_ix = state.read(cx).current_ix();
let progress = state.read(cx).progress();
let status = state
    .read(cx)
    .item_state("direction")
    .map(|item| item.status());
let navigation = state.read(cx).navigation_state();
let can_confirm = navigation.is_confirmable();
let show_previous = navigation.is_previous_visible();
let show_next = navigation.is_next_visible();
let show_skip = navigation.is_skip_visible();
let show_submit = navigation.is_submit_visible();
```

The default action layout shows `Previous` at the beginning, `Next` between
items, `Skip` only for the active optional item, and `Submit` at the end.
Hidden actions are not rendered and do not enter keyboard navigation. Disabled
items are removed from the navigation and progress totals. The three item
statuses are `Unanswered`, `Answered`, and `Skipped`.

## Validation

Required status validation is built in. Add a synchronous validator to an item
for domain-specific checks. The validator receives the current item, its
answer, and the complete enabled answer snapshot through
`QuestionnaireValidationContext`. `Next` validates the current item; `Submit`
validates all enabled items and focuses the first invalid item.

```rust
let item = QuestionnaireItemDefinition::new("handle", "Choose a handle")
    .with_required(true)
    .with_validator(|context| {
        if context
            .answer()
            .freeform()
            .is_some_and(|value| value.as_ref().len() >= 3)
        {
            Ok(())
        } else {
            Err("Use at least three characters.".into())
        }
    });
```

An optional unanswered item is invalid until the user explicitly skips it;
`Skipped` is intentionally valid. Disabled items and disabled controls do not
participate in validation. The first invalid item is selected on submit, and
focus goes to its filled input or selected choice before falling back to the
first enabled control.

Use external errors for schema or server responses. External errors belong to
the host and remain until the host clears them.

```rust
state.update(cx, |state, cx| {
    state
        .set_external_error("handle", "This handle is already taken.", cx)
        .expect("known questionnaire item");
});

// After the owner accepts a corrected answer or a new server response:
state.update(cx, |state, cx| {
    state
        .clear_external_error("handle", cx)
        .expect("known questionnaire item");
});
```

`reset` clears internal validation attempts and errors, but preserves
owner-managed external errors. Questionnaire semantic validation and
synchronous validators are supported; native HTML constraint validation is
not part of this GPUI component.

## Controlled state

When a page owns the active item or needs to apply a saved answer after state
creation, use the silent setters. They update the UI and focus as needed but do
not emit user-interaction events.

```rust
use gpui_component::questionnaire::QuestionnaireAnswer;

state.update(cx, |state, cx| {
    state
        .set_current_item("detail", window, cx)
        .expect("known enabled questionnaire item");
    state
        .set_answer(
            "direction",
            QuestionnaireAnswer::new().with_choices(["delegation"]),
            window,
            cx,
        )
        .expect("known questionnaire item");
    state
        .set_input_value("direction", "A controlled draft", window, cx)
        .expect("item has an input");
});
```

Use `activate_choice`, `confirm_current`, `go_previous`, `go_next`,
`skip_current`, and `submit` for user intent. Those paths emit the relevant
`QuestionnaireEvent` values. A host can also use `set_item_disabled` and
`set_choice_disabled`; disabling the current item moves focus to the next
enabled item, or to the previous one when there is no next item.

## Resume

To make `reset` return to a saved draft, establish the saved draft as the
initial snapshot before constructing `QuestionnaireState`. Use
`InputState::default_value`, `with_default_selected`, and
`with_current_item` for the input, choice, and current-item baselines.

```rust
let saved_input = cx.new(|cx| {
    InputState::new(window, cx).default_value("Saved description")
});
let saved_items = vec![
    QuestionnaireItemDefinition::new("plan", "Which plan?")
        .with_choices([
            QuestionnaireChoiceDefinition::new("plus", "Plus")
                .with_default_selected(true),
            QuestionnaireChoiceDefinition::new("pro", "Pro"),
        ]),
    QuestionnaireItemDefinition::new("detail", "How much detail?")
        .with_input(QuestionnaireInputDefinition::new(saved_input, "More detail")),
];
let state = cx.new(|cx| {
    QuestionnaireState::new(saved_items, cx)
        .expect("valid saved questionnaire")
        .with_current_item("detail")
        .expect("known enabled questionnaire item")
});
```

If the saved values arrive after construction, apply
`set_answer`, `set_input_value`, and `set_current_item` instead. Those setters
change the current state; they do not replace the reset baseline.

## Reset

Reset restores the initial choices and input drafts, clears intentional skips,
validation attempts, and completion, and returns to the initial current item.
It also focuses the restored current item.

```rust
state.update(cx, |state, cx| {
    state.reset(window, cx);
});
```

External errors remain owner-managed across reset. If a reset should also
remove a server error, clear it explicitly with `clear_external_error`.

## Conditional items

Questionnaire does not contain a branching engine. The host can derive an
item's disabled state from an earlier answer and synchronize it with
`set_item_disabled`. This keeps conditional policy in the page while the
Questionnaire continues to own ordering, focus, progress, validation, and
submission.

```rust
fn sync_advanced_item(
    state: &Entity<QuestionnaireState>,
    window: &mut Window,
    cx: &mut App,
) {
    let enabled = state.read(cx).answer("direction").is_some_and(|answer| {
        answer
            .choices()
            .iter()
            .any(|choice| choice.as_ref() == "delegation")
    });

    state.update(cx, |state, cx| {
        let _ = state.set_item_disabled("advanced", !enabled, window, cx);
    });
}
```

Call this helper from the host's answer-change handling or from the UI action
that changes the earlier answer. A disabled conditional item is excluded from
progress, navigation, validation, focus, shortcuts, and submission.

## Keyboard shortcuts

Enable letter or number shortcuts on the state. Shortcuts apply only to the
active item's enabled choices. Repeated key events, text input, IME composition,
and modified key presses are left untouched.

```rust
use gpui_component::questionnaire::QuestionnaireShortcutMode;

let state = cx.new(|cx| {
    QuestionnaireState::new(items, cx)
        .expect("valid questionnaire schema")
        .with_shortcuts(QuestionnaireShortcutMode::Letters)
});
```

Questionnaire handles radio movement according to the native single-choice
interaction. Up and Down otherwise move through enabled choices and the
freeform input in schema order; the input remains in that order when present.
When a non-empty text input has focus, its normal text-editing behavior is
preserved. Left and Right move between items only outside text inputs and
single-choice radio controls; Right requires a confirmable current item.

Enter confirms a filled answer. Command/Ctrl+Enter confirms the current item.
An empty answer does not implicitly submit. Shortcut labels are assigned in
enabled-choice order (`A`–`Z` or `1`–`9`), and disabled choices receive no label.

## Progress and custom rendering

`QuestionnaireProgress` follows the default presentation: “Question 2 of 4”.
Its state can also be used to compose a custom indicator from the existing
`Progress` or `Stepper` components.

```rust
QuestionnaireProgress::new(&state)
    .with_size(Size::Small);

let progress = state.read(cx).progress();
let percent = if progress.total() == 0 {
    0.
} else {
    progress.current() as f32 / progress.total() as f32 * 100.
};
Progress::new("questionnaire-progress")
    .value(percent);

Stepper::new("questionnaire-steps")
    .selected_index(progress.current().saturating_sub(1));
```

## Sizes and theming

Questionnaire parts implement the same `Sizable` contract as the rest of the
library. `Medium` is the default and follows the shadcn/ui `base-nova` docs
appearance. The supported named sizes are `XSmall`, `Small`, `Medium`, and
`Large`; `Size::Size(value)` is available for a custom scale.

Compound parts do not inherit the root's size automatically. Pass the same
size to the root, progress, item, title, description, choices, choice, choice
description, input, error, actions, and navigation parts that should share one
scale.

```rust
use gpui_component::{Sizable as _, Size};

let size = Size::Small;
Questionnaire::new(&state)
    .with_size(size)
    .child(QuestionnaireProgress::new(&state).with_size(size))
    .child(
        QuestionnaireItem::new(&state, "direction")
            .with_size(size)
            .child(QuestionnaireTitle::new(&state, "direction").with_size(size))
            .child(QuestionnaireDescription::new(&state, "direction").with_size(size))
            .child(
                QuestionnaireChoices::new(&state, "direction")
                    .with_size(size)
                    .child(QuestionnaireChoice::new(&state, "direction", "delegation").with_size(size))
                    .child(QuestionnaireInput::new(&state, "direction").with_size(size)),
            )
            .child(QuestionnaireError::new(&state, "direction").with_size(size)),
    )
    .child(
        QuestionnaireActions::new(&state)
            .with_size(size)
            .child(QuestionnairePrevious::new(&state).with_size(size))
            .child(QuestionnaireSkip::new(&state).with_size(size))
            .child(QuestionnaireNext::new(&state).with_size(size))
            .child(QuestionnaireSubmit::new(&state).with_size(size)),
    );
```

The default skin derives spacing, typography, radius, border, input, primary,
muted, destructive, and focus-ring values from the active theme's semantic
tokens. Use `Styled` methods or `StyleRefinement` for local adjustments; local
style refinement is applied after the component defaults.

## Card and Dialog composition

The questionnaire owns the complete question flow. A card or dialog owns its
container layout and close/cancel behavior. Both examples below include every
item in the collection, so moving to the second question remains visible.

```rust
use gpui::{Entity, IntoElement, ParentElement as _};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dialog::{Dialog, DialogClose, DialogFooter, DialogHeader, DialogTitle},
    group_box::{GroupBox, GroupBoxVariants as _},
};

fn questionnaire_content(
    state: &Entity<QuestionnaireState>,
    actions: impl IntoElement,
) -> Questionnaire {
    Questionnaire::new(state)
        .child(QuestionnaireProgress::new(state))
        .child(
            QuestionnaireItem::new(state, "direction")
                .child(QuestionnaireTitle::new(state, "direction"))
                .child(QuestionnaireDescription::new(state, "direction"))
                .child(
                    QuestionnaireChoices::new(state, "direction")
                        .child(QuestionnaireChoice::new(state, "direction", "delegation"))
                        .child(QuestionnaireChoice::new(state, "direction", "questions"))
                        .child(QuestionnaireChoice::new(state, "direction", "both"))
                        .child(QuestionnaireInput::new(state, "direction")),
                )
                .child(QuestionnaireError::new(state, "direction")),
        )
        .child(
            QuestionnaireItem::new(state, "detail")
                .child(QuestionnaireTitle::new(state, "detail"))
                .child(QuestionnaireDescription::new(state, "detail"))
                .child(
                    QuestionnaireChoices::new(state, "detail")
                        .child(QuestionnaireChoice::new(state, "detail", "focused"))
                        .child(QuestionnaireChoice::new(state, "detail", "complete")),
                )
                .child(QuestionnaireError::new(state, "detail")),
        )
        .child(actions)
}

GroupBox::new()
    .outline()
    .title("Set up your workspace")
    .child(questionnaire_content(
        &state,
        QuestionnaireActions::new(&state)
            .child(QuestionnairePrevious::new(&state))
            .child(QuestionnaireSkip::new(&state))
            .child(QuestionnaireNext::new(&state))
            .child(QuestionnaireSubmit::new(&state)),
    ));
```

For a dialog, put the same complete composition inside the dialog content and
let the host handle dismissal and cancellation.

```rust
use gpui_component::{WindowExt as _, questionnaire::QuestionnaireEvent};

let dialog_state = state.clone();
cx.subscribe_in(
    &dialog_state,
    window,
    |_, _, event: &QuestionnaireEvent, window, cx| {
        if matches!(event, QuestionnaireEvent::Submit(_)) {
            window.close_dialog(cx);
        }
    },
)
.detach();

Dialog::new(cx)
    .trigger(
        Button::new("open-questionnaire")
            .outline()
            .label("Open questionnaire"),
    )
    .content(move |content, _, _| {
        content
            .child(DialogHeader::new().child(DialogTitle::new().child("Workspace setup")))
            .child(questionnaire_content(
                &dialog_state,
                DialogFooter::new()
                    .child(
                        DialogClose::new().child(
                            Button::new("cancel-questionnaire")
                                .outline()
                                .label("Cancel"),
                        ),
                    )
                    .child(
                        QuestionnaireActions::new(&dialog_state)
                            .child(QuestionnairePrevious::new(&dialog_state))
                            .child(QuestionnaireNext::new(&dialog_state))
                            .child(QuestionnaireSubmit::new(&dialog_state)),
                    ),
            ))
    });
```

The host subscription closes the Dialog only after a successful `Submit`.
The same event is the place to hand a validated `QuestionnaireSubmission` to
application transport. Persistence and network success remain outside
Questionnaire.

## Events and submission

Subscribe to `QuestionnaireEvent` for active-item changes, answer changes,
completion, and successful submit. `Completed` is emitted on the transition
into a complete state; `Submit` is emitted for each successful explicit submit.
On the first successful submit, the order is `Completed` followed by `Submit`.
Changing answers or enabled conditions clears completion, so the next successful
submit can emit `Completed` again.

```rust
use gpui_component::questionnaire::QuestionnaireEvent;

cx.subscribe(&state, |_, _, event, _| match event {
    QuestionnaireEvent::CurrentItemChanged { current, .. } => {
        println!("Current item: {:?}", current);
    }
    QuestionnaireEvent::AnswerChanged(change) => {
        println!("Changed: {:?} ({:?})", change.item(), change.status());
    }
    QuestionnaireEvent::Completed(submission)
    | QuestionnaireEvent::Submit(submission) => {
        println!("Answers: {:?}", submission.items());
    }
    _ => {}
})
.detach();
```

Detaching keeps the callback alive until the subscribed entities are dropped.
Store the returned `Subscription` in the host instead when it needs to cancel
the listener earlier.

The submission is ordered by the item schema and contains only enabled items.
Each item includes its name, `Unanswered`/`Answered`/`Skipped` status, and
effective answer. It represents a validated local submission request; saving
it remotely remains the host application's responsibility.

## Accessibility

`Questionnaire` uses the GPUI `Form` role for the root. `QuestionnaireItem` is
an accessible group with its item label and description. The definition's
`accessibility_label` and `description` remain the semantic source for the
item and choice, even when a custom child replaces the visible fallback
content. `QuestionnaireError` is announced as an alert only while the item is
invalid. Choice parts preserve radio and checkbox semantics, progress exposes
current and total values, and navigation uses real buttons.

Inactive items and hidden actions are removed from keyboard navigation. On a
successful transition focus moves to the new item; on validation failure focus
moves to the selected or filled answer control, then to the first available
control.

Always provide an accessible label for a freeform input with its definition's
`accessibility_label`; a visible label or equivalent custom composition can
supplement it. The GPUI accessibility layer does not expose a direct
`aria-invalid` builder. Questionnaire still exposes invalid state through its
error alert, semantic group state, focus behavior, and destructive styling.

## Current scope

This GPUI port covers state, navigation, validation, focus, accessibility,
compound rendering, and local submission events. The following web-specific or
future behaviors are not currently provided: SSR/hydration collection
diagnostics, `FormData`, native HTML validation, DOM mutation registration,
async validation, runtime insertion/reordering of definitions, and built-in
animation or persistence/transport.

## API reference

### Compound parts

- [Questionnaire]
- [QuestionnaireProgress]
- [QuestionnaireItem]
- [QuestionnaireTitle]
- [QuestionnaireDescription]
- [QuestionnaireChoices]
- [QuestionnaireChoice]
- [QuestionnaireChoiceDescription]
- [QuestionnaireInput]
- [QuestionnaireError]
- [QuestionnaireActions]
- [QuestionnairePrevious]
- [QuestionnaireSkip]
- [QuestionnaireNext]
- [QuestionnaireSubmit]

### State, answers, and events

- [QuestionnaireState]
- [QuestionnaireItemDefinition]
- [QuestionnaireChoiceDefinition]
- [QuestionnaireInputDefinition]
- [QuestionnaireAnswer]
- [QuestionnaireAnswers]
- [QuestionnaireItemStatus]
- [QuestionnaireShortcutMode]
- [QuestionnaireProgressState]
- [QuestionnaireItemState]
- [QuestionnaireChoiceState]
- [QuestionnaireNavigationState]
- [QuestionnaireValidationContext]
- [QuestionnaireValidator]
- [QuestionnaireAnswerChange]
- [QuestionnaireSubmission]
- [QuestionnaireSubmissionItem]
- [QuestionnaireEvent]
- [QuestionnaireStateError]
- [Sizable]

[Questionnaire]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.Questionnaire.html
[QuestionnaireProgress]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireProgress.html
[QuestionnaireItem]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItem.html
[QuestionnaireTitle]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireTitle.html
[QuestionnaireDescription]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireDescription.html
[QuestionnaireChoices]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoices.html
[QuestionnaireChoice]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoice.html
[QuestionnaireChoiceDescription]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceDescription.html
[QuestionnaireInput]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInput.html
[QuestionnaireError]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireError.html
[QuestionnaireActions]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireActions.html
[QuestionnairePrevious]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnairePrevious.html
[QuestionnaireSkip]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSkip.html
[QuestionnaireNext]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireNext.html
[QuestionnaireSubmit]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmit.html
[QuestionnaireState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireState.html
[QuestionnaireItemDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItemDefinition.html
[QuestionnaireChoiceDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceDefinition.html
[QuestionnaireInputDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInputDefinition.html
[QuestionnaireAnswer]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireAnswer.html
[QuestionnaireAnswers]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireAnswers.html
[QuestionnaireItemStatus]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireItemStatus.html
[QuestionnaireShortcutMode]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireShortcutMode.html
[QuestionnaireProgressState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireProgressState.html
[QuestionnaireItemState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItemState.html
[QuestionnaireChoiceState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceState.html
[QuestionnaireNavigationState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireNavigationState.html
[QuestionnaireValidationContext]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireValidationContext.html
[QuestionnaireValidator]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/type.QuestionnaireValidator.html
[QuestionnaireAnswerChange]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireAnswerChange.html
[QuestionnaireSubmission]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmission.html
[QuestionnaireSubmissionItem]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmissionItem.html
[QuestionnaireEvent]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireEvent.html
[QuestionnaireStateError]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireStateError.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
