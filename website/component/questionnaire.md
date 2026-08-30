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
    QuestionnaireChoices, QuestionnaireDescription, QuestionnaireError,
    QuestionnaireInput, QuestionnaireItem, QuestionnaireNext,
    QuestionnairePrevious, QuestionnaireProgress, QuestionnaireSkip,
    QuestionnaireState, QuestionnaireSubmit, QuestionnaireTitle,
};
```

## Usage

Create the item collection once and use one `QuestionnaireState` entity as the
source of truth for all parts.

```rust
use gpui_component::questionnaire::{
    QuestionnaireItemDefinition, QuestionnaireChoiceDefinition,
    QuestionnaireInputDefinition, QuestionnaireState, QuestionnaireAnswer,
    QuestionnaireEvent,
};
use gpui_component::input::InputState;

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

Map the same collection into the compound parts. The parts are intentionally
small, so an application can replace a title, choice body, progress indicator,
or action without taking ownership of questionnaire state.

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
        QuestionnaireActions::new(&state)
            .child(QuestionnairePrevious::new(&state))
            .child(QuestionnaireSkip::new(&state))
            .child(QuestionnaireNext::new(&state))
            .child(QuestionnaireSubmit::new(&state)),
    )
```

`Questionnaire` renders the active item. The other items remain in the ordered
schema and are available to navigation and final validation.

## Composition

```text
Questionnaire
├── QuestionnaireProgress
├── QuestionnaireItem
│   ├── QuestionnaireTitle
│   ├── QuestionnaireDescription
│   ├── QuestionnaireChoices
│   │   ├── QuestionnaireChoice
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
`Dialog` elements. A custom part should read the corresponding state and call
the state methods for user actions; it should not duplicate answer state.

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

The answer reader preserves schema order. Disabled choices are excluded from
answers even if a previously restored answer contains their value.

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
choice is selected, but it is submitted only when it is the active freeform
answer.

## Explicit skip

Optional items can expose `QuestionnaireSkip`. A skip is an intentional valid
state, clears the item answer, and allows `Next` to continue. Required items do
not allow skipping. Re-entering an item and choosing an answer clears its
skipped state.

```rust
let optional = QuestionnaireItemDefinition::new("tone", "What tone should we use?")
    .with_required(false)
    .with_choices([
        QuestionnaireChoiceDefinition::new("direct", "Direct"),
        QuestionnaireChoiceDefinition::new("warm", "Warm"),
    ]);
```

## Navigation and status

`QuestionnaireState` exposes the current item, ordered item states, and
navigation state for custom action layouts.

```rust
let current = state.read(cx).current_item();
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

state.update(cx, |state, cx| {
    state.go_previous(window, cx);
    state.go_next(window, cx);
});
```

The default action layout shows `Previous` at the beginning, `Next` between
items, `Skip` only for the active optional item, and `Submit` at the end.
Hidden actions are inert and do not enter keyboard navigation. Disabled items
are removed from the navigation and progress totals.

## Validation

Required status validation is built in. Add a synchronous validator to an item
for domain-specific checks. `Next` validates the current item; `Submit`
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

An application can show an external schema or server error with
`set_external_error`, then clear it after the owner has corrected the data.
Reset restores defaults and clears internal validation state while leaving
owner-managed external errors under application control.

```rust
state.update(cx, |state, cx| {
    state
        .set_external_error("handle", "This handle is already taken.", cx)
        .expect("known questionnaire item");
});
```

## Controlled state, resume, and reset

Use the state readers and silent setters when a page owns the active item or
restores a saved draft. Silent setters update the UI without emitting user
interaction events.

```rust
state.update(cx, |state, cx| {
    state
        .set_current_item("detail", window, cx)
        .expect("known enabled questionnaire item");
    state.set_answer(
        "direction",
        QuestionnaireAnswer::new().with_choices(["delegation"]),
        window,
        cx,
    ).expect("known questionnaire item");
    state.reset(window, cx);
});
```

Set disabled state when an earlier answer makes an item irrelevant. Disabled
items do not count toward progress, validation, focus, or submission.

```rust
state.update(cx, |state, cx| {
    state
        .set_item_disabled("advanced", true, window, cx)
        .expect("known questionnaire item");
});
```

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

Questionnaire handles all four arrow directions inside a single-choice radio
group, moving focus and selecting the next enabled choice. Up and Down otherwise
move between checkbox, choice, and freeform controls in schema order; Left and
Right move between items only when focus is not in a text input or radio control.
Enter confirms a filled answer, and Command/Ctrl+Enter confirms the current
item. Empty answers do not implicitly submit the questionnaire.

## Progress and custom rendering

`QuestionnaireProgress` follows the docs default presentation: “Question 2 of
4”. Its state can also be used to compose a custom indicator from the existing
`Progress` or `Stepper` components.

```rust
QuestionnaireProgress::new(&state)
    .with_size(Size::Small)

let progress = state.read(cx).progress();
let percent = if progress.total() == 0 {
    0.
} else {
    progress.current() as f32 / progress.total() as f32 * 100.
};
Progress::new("questionnaire-progress").value(percent)

Stepper::new("questionnaire-steps")
    .selected_index(progress.current().saturating_sub(1))
```

## Sizes and theming

Questionnaire parts implement the same `Sizable` contract as the rest of the
library. `Medium` is the default and follows the shadcn/ui `base-nova` docs
appearance.

```rust
use gpui_component::{Sizable as _, Size};

Questionnaire::new(&state).with_size(Size::Small)
Questionnaire::new(&state).with_size(Size::Medium)
Questionnaire::new(&state).with_size(Size::Large)
```

The default skin derives spacing, typography, radius, border, input, primary,
muted, destructive, and focus-ring values from the active theme's semantic
tokens. Use `Styled` methods or `StyleRefinement` for local adjustments; no
Questionnaire-specific color constants are required.

## Card and Dialog composition

The questionnaire owns its question flow. A card or dialog owns its container
layout and close/cancel behavior.

```rust
GroupBox::new()
    .outline()
    .title("Set up your workspace")
    .child(Questionnaire::new(&state))
```

For a dialog, create the Questionnaire inside the existing Dialog content and
let the host handle dismissal. The Questionnaire `Submit` event is the place
to hand a validated `QuestionnaireSubmission` to application transport.

## Events and submission

Subscribe to `QuestionnaireEvent` for active-item changes, answer changes,
completion, and successful submit. `Completed` is emitted on the transition
into a complete state; `Submit` is emitted for each successful explicit submit.

```rust
cx.subscribe(&state, |_, _, event, _| match event {
    QuestionnaireEvent::CurrentItemChanged { current, .. } => {
        println!("Current item: {:?}", current);
    }
    QuestionnaireEvent::AnswerChanged(change) => {
        println!("Changed: {:?}", change.item());
    }
    QuestionnaireEvent::Completed(submission)
    | QuestionnaireEvent::Submit(submission) => {
        println!("Answers: {:?}", submission.items());
    }
    _ => {}
});
```

The submission is ordered by the item schema and contains only enabled items.
It represents a validated local submission request; saving it remotely remains
the host application's responsibility.

## Accessibility

`Questionnaire` uses the GPUI `Form` role for the root. `QuestionnaireItem` is
an accessible group with its item label and description. The definition's
`accessibility_label` and `description` remain the semantic source for the
item and choice, even when a custom child replaces the visible fallback
content. Custom children control visible presentation and keep the state,
roles, focus behavior, and semantics supplied by the Questionnaire parts.
`QuestionnaireError` is announced as an alert only while the item is invalid.
Choice parts preserve radio and checkbox semantics, progress exposes current
and total values, and navigation uses real buttons.

Inactive items and hidden actions are removed from keyboard navigation. On a
successful transition focus moves to the new item; on validation failure focus
moves to the selected or filled answer control, then to the first available
control.

Always provide an accessible label for a freeform input with its definition's
`accessibility_label`; a visible label or equivalent custom composition can
supplement it. The GPUI accessibility layer does not expose a direct
`aria-invalid` builder. Questionnaire still exposes invalid state through its
error alert, semantic group state, focus behavior, and destructive styling.

## API reference

- [Questionnaire]
- [QuestionnaireState]
- [QuestionnaireItemDefinition]
- [QuestionnaireChoiceDefinition]
- [QuestionnaireInputDefinition]
- [QuestionnaireProgress]
- [QuestionnaireItem]
- [QuestionnaireChoice]
- [QuestionnaireInput]
- [QuestionnaireActions]
- [QuestionnaireEvent]
- [QuestionnaireSubmission]
- [Sizable]

[Questionnaire]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.Questionnaire.html
[QuestionnaireState]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireState.html
[QuestionnaireItemDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItemDefinition.html
[QuestionnaireChoiceDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoiceDefinition.html
[QuestionnaireInputDefinition]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInputDefinition.html
[QuestionnaireProgress]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireProgress.html
[QuestionnaireItem]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireItem.html
[QuestionnaireChoice]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireChoice.html
[QuestionnaireInput]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireInput.html
[QuestionnaireActions]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireActions.html
[QuestionnaireEvent]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/enum.QuestionnaireEvent.html
[QuestionnaireSubmission]: https://docs.rs/gpui-component/latest/gpui_component/questionnaire/struct.QuestionnaireSubmission.html
[Sizable]: https://docs.rs/gpui-component/latest/gpui_component/trait.Sizable.html
