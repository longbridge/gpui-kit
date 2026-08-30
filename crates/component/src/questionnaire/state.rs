use std::collections::HashSet;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable as _, SharedString, Subscription,
    Window,
};
use rust_i18n::t;

use crate::input::{InputEvent, InputState};

use super::types::*;

struct ItemRuntime {
    disabled: bool,
    choice_disabled: Vec<bool>,
    input_disabled: bool,
    answer: QuestionnaireAnswer,
    initial_answer: QuestionnaireAnswer,
    initial_input_value: Option<SharedString>,
    skipped: bool,
    validation_attempted: bool,
    internal_error: Option<SharedString>,
    external_error: Option<SharedString>,
    focus_handle: FocusHandle,
    choice_focus_handles: Vec<FocusHandle>,
    input_focus_handle: Option<FocusHandle>,
}

/// Owns questionnaire answers, validation, navigation and focus state.
pub struct QuestionnaireState {
    items: Vec<QuestionnaireItemDefinition>,
    runtime: Vec<ItemRuntime>,
    current: Option<usize>,
    initial_current: Option<SharedString>,
    shortcut_mode: Option<QuestionnaireShortcutMode>,
    complete: bool,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl QuestionnaireState {
    pub fn new(
        items: Vec<QuestionnaireItemDefinition>,
        cx: &mut Context<Self>,
    ) -> Result<Self, QuestionnaireStateError> {
        Self::validate_schema(&items)?;

        let mut runtime = Vec::with_capacity(items.len());
        let mut subscriptions = Vec::new();

        for item in &items {
            let mut answer = QuestionnaireAnswer::new().with_choices(
                item.choices()
                    .iter()
                    .filter(|choice| choice.is_default_selected() && !choice.is_disabled())
                    .map(|choice| choice.value().clone()),
            );

            let mut initial_input_value = None;
            let mut input_focus_handle = None;
            if let Some(input) = item.input() {
                let state = input.state();
                state.update(cx, |state, cx| {
                    state.set_disabled(item.is_disabled() || input.is_disabled(), cx);
                });

                let value = state.read(cx).value();
                initial_input_value = Some(value.clone());
                input_focus_handle = Some(state.focus_handle(cx));
                if !value.trim().is_empty() && !input.is_disabled() {
                    if !item.is_multiple() {
                        answer = QuestionnaireAnswer::new();
                    }
                    answer.freeform = Some(value);
                }

                subscriptions.push(cx.subscribe(state, |this, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.on_input_change(&input, cx);
                    }
                }));
            }

            runtime.push(ItemRuntime {
                disabled: item.is_disabled(),
                choice_disabled: item
                    .choices()
                    .iter()
                    .map(QuestionnaireChoiceDefinition::is_disabled)
                    .collect(),
                input_disabled: item
                    .input()
                    .is_none_or(QuestionnaireInputDefinition::is_disabled),
                initial_answer: answer.clone(),
                initial_input_value,
                answer,
                skipped: false,
                validation_attempted: false,
                internal_error: None,
                external_error: None,
                focus_handle: cx.focus_handle(),
                choice_focus_handles: item.choices().iter().map(|_| cx.focus_handle()).collect(),
                input_focus_handle,
            });
        }

        let current = runtime.iter().position(|item| !item.disabled);
        let initial_current = current.map(|ix| items[ix].name().clone());

        Ok(Self {
            items,
            runtime,
            current,
            initial_current,
            shortcut_mode: None,
            complete: false,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
        })
    }

    fn validate_schema(
        items: &[QuestionnaireItemDefinition],
    ) -> Result<(), QuestionnaireStateError> {
        let mut item_names = HashSet::new();
        for item in items {
            if !item_names.insert(item.name().to_string()) {
                return Err(QuestionnaireStateError::DuplicateItem(item.name().clone()));
            }

            let mut choices = HashSet::new();
            let mut defaults = 0;
            for choice in item.choices() {
                if !choices.insert(choice.value().to_string()) {
                    return Err(QuestionnaireStateError::DuplicateChoice {
                        item: item.name().clone(),
                        choice: choice.value().clone(),
                    });
                }
                defaults += usize::from(choice.is_default_selected());
            }
            if !item.is_multiple() && defaults > 1 {
                return Err(QuestionnaireStateError::MultipleDefaultsForSingleItem(
                    item.name().clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn with_current_item(
        mut self,
        name: impl Into<SharedString>,
    ) -> Result<Self, QuestionnaireStateError> {
        let name = name.into();
        let ix = self.item_ix(&name)?;
        if !self.runtime[ix].disabled {
            self.current = Some(ix);
            self.initial_current = Some(name);
        }
        Ok(self)
    }

    pub fn with_shortcuts(mut self, mode: QuestionnaireShortcutMode) -> Self {
        self.shortcut_mode = Some(mode);
        self
    }

    pub fn current_item(&self) -> Option<&SharedString> {
        self.current.map(|ix| self.items[ix].name())
    }

    pub fn current_ix(&self) -> Option<usize> {
        let current = self.current?;
        self.enabled_indices().position(|ix| ix == current)
    }

    pub fn total(&self) -> usize {
        self.enabled_indices().count()
    }

    pub fn progress(&self) -> QuestionnaireProgressState {
        QuestionnaireProgressState::new(self.current_ix().map_or(0, |ix| ix + 1), self.total())
    }

    pub fn item_definition(&self, name: &str) -> Option<&QuestionnaireItemDefinition> {
        self.items.iter().find(|item| item.name().as_ref() == name)
    }

    pub fn choice_definition(
        &self,
        item: &str,
        value: &str,
    ) -> Option<&QuestionnaireChoiceDefinition> {
        self.item_definition(item)?
            .choices()
            .iter()
            .find(|choice| choice.value().as_ref() == value)
    }

    pub fn item_state(&self, name: &str) -> Option<QuestionnaireItemState> {
        let ix = self.item_ix_opt(name)?;
        let definition = &self.items[ix];
        Some(QuestionnaireItemState::new(
            definition.name().clone(),
            self.status(ix),
            definition.is_required(),
            definition.is_multiple(),
            self.runtime[ix].disabled,
            self.error_at(ix).is_some(),
            definition.input().is_some(),
        ))
    }

    pub fn choice_state(&self, item: &str, value: &str) -> Option<QuestionnaireChoiceState> {
        let item_ix = self.item_ix_opt(item)?;
        let choice_ix = self.choice_ix_opt(item_ix, value)?;
        let runtime = &self.runtime[item_ix];
        let definition = &self.items[item_ix].choices()[choice_ix];
        Some(QuestionnaireChoiceState::new(
            definition.value().clone(),
            runtime.answer.choices.contains(definition.value()),
            runtime.disabled || runtime.choice_disabled[choice_ix],
            self.error_at(item_ix).is_some(),
            self.shortcut_for_choice(item, value),
        ))
    }

    pub fn navigation_state(&self) -> QuestionnaireNavigationState {
        let Some(ix) = self.current_ix() else {
            return QuestionnaireNavigationState::default();
        };
        let total = self.total();
        let item_ix = self
            .current
            .expect("current item exists when enabled index exists");
        QuestionnaireNavigationState::new(
            ix > 0,
            ix + 1 < total,
            !self.items[item_ix].is_required(),
            ix + 1 == total,
            self.status(item_ix) != QuestionnaireItemStatus::Unanswered,
        )
    }

    pub fn answer(&self, name: &str) -> Option<QuestionnaireAnswer> {
        let ix = self.item_ix_opt(name)?;
        Some(self.effective_answer(ix))
    }

    pub fn answers(&self) -> QuestionnaireAnswers {
        QuestionnaireAnswers::from_entries(
            self.enabled_indices()
                .map(|ix| (self.items[ix].name().clone(), self.effective_answer(ix)))
                .collect(),
        )
    }

    pub fn error(&self, name: &str) -> Option<&SharedString> {
        self.item_ix_opt(name).and_then(|ix| self.error_at(ix))
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn input_state(&self, name: &str) -> Option<Entity<InputState>> {
        self.item_definition(name)?
            .input()
            .map(|input| input.state().clone())
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    pub fn item_focus_handle(&self, name: &str) -> Option<&FocusHandle> {
        self.item_ix_opt(name)
            .map(|ix| &self.runtime[ix].focus_handle)
    }

    pub fn choice_focus_handle(&self, item: &str, value: &str) -> Option<&FocusHandle> {
        let item_ix = self.item_ix_opt(item)?;
        let choice_ix = self.choice_ix_opt(item_ix, value)?;
        Some(&self.runtime[item_ix].choice_focus_handles[choice_ix])
    }

    pub fn is_current_input_focused(&self, window: &Window) -> bool {
        let Some(ix) = self.current else { return false };
        self.runtime[ix]
            .input_focus_handle
            .as_ref()
            .is_some_and(|handle| handle.is_focused(window))
    }

    pub(crate) fn current_input_has_text(&self, cx: &App) -> bool {
        let Some(item_ix) = self.current else {
            return false;
        };
        self.items[item_ix]
            .input()
            .is_some_and(|input| !input.state().read(cx).value().trim().is_empty())
    }

    pub fn focused_current_choice(&self, window: &Window) -> Option<&SharedString> {
        let ix = self.current?;
        self.runtime[ix]
            .choice_focus_handles
            .iter()
            .position(|handle| handle.is_focused(window))
            .map(|choice_ix| self.items[ix].choices()[choice_ix].value())
    }

    pub fn shortcut_mode(&self) -> Option<QuestionnaireShortcutMode> {
        self.shortcut_mode
    }

    pub fn shortcut_for_choice(&self, item: &str, value: &str) -> Option<SharedString> {
        let mode = self.shortcut_mode?;
        let item_ix = self.item_ix_opt(item)?;
        let choice_ix = self.choice_ix_opt(item_ix, value)?;
        if self.runtime[item_ix].disabled || self.runtime[item_ix].choice_disabled[choice_ix] {
            return None;
        }
        let enabled_position = (0..=choice_ix)
            .filter(|ix| !self.runtime[item_ix].choice_disabled[*ix])
            .count()
            .checked_sub(1)?;
        match mode {
            QuestionnaireShortcutMode::Letters if enabled_position < 26 => {
                Some(char::from(b'A' + enabled_position as u8).to_string().into())
            }
            QuestionnaireShortcutMode::Numbers if enabled_position < 9 => {
                Some((enabled_position + 1).to_string().into())
            }
            _ => None,
        }
    }

    pub fn choice_for_shortcut(&self, item: &str, key: &str) -> Option<&SharedString> {
        let item_ix = self.item_ix_opt(item)?;
        self.items[item_ix].choices().iter().find_map(|choice| {
            self.shortcut_for_choice(item, choice.value())
                .is_some_and(|shortcut| shortcut.as_ref().eq_ignore_ascii_case(key))
                .then_some(choice.value())
        })
    }

    pub fn activate_shortcut(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(item_ix) = self.current else {
            return false;
        };
        let item = self.items[item_ix].name().clone();
        let Some(choice) = self.choice_for_shortcut(&item, key).cloned() else {
            return false;
        };
        if self.activate_choice(&item, &choice, cx).is_err() {
            return false;
        }
        self.focus_choice(&item, &choice, window, cx)
    }

    pub fn set_current_item(
        &mut self,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let ix = self.item_ix(name)?;
        if !self.runtime[ix].disabled {
            self.current = Some(ix);
            self.focus_current_item(window, cx);
            cx.notify();
        }
        Ok(())
    }

    pub fn set_answer(
        &mut self,
        item: &str,
        mut answer: QuestionnaireAnswer,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let item_ix = self.item_ix(item)?;
        let before = self.effective_answer(item_ix);
        let before_status = self.status(item_ix);
        if answer
            .freeform
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            answer.freeform = None;
        }
        self.check_answer(item_ix, &answer)?;
        answer.choices = self.items[item_ix]
            .choices()
            .iter()
            .filter(|choice| answer.choices.contains(choice.value()))
            .map(|choice| choice.value().clone())
            .collect();
        self.runtime[item_ix].answer = answer.clone();
        self.runtime[item_ix].skipped = false;
        if let (Some(input), Some(value)) =
            (self.items[item_ix].input(), answer.freeform().cloned())
        {
            input
                .state()
                .update(cx, |input, cx| input.set_value(value, window, cx));
        }
        if before != self.effective_answer(item_ix) || before_status != self.status(item_ix) {
            self.answer_did_change(item_ix, false, cx);
        }
        Ok(())
    }

    pub fn set_input_value(
        &mut self,
        item: &str,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let item_ix = self.item_ix(item)?;
        let Some(input) = self.items[item_ix]
            .input()
            .map(|input| input.state().clone())
        else {
            return Err(QuestionnaireStateError::AnswerDoesNotMatchItem(
                self.items[item_ix].name().clone(),
            ));
        };
        input.update(cx, |input, cx| input.set_value(value, window, cx));
        self.sync_input_answer(item_ix, false, cx);
        Ok(())
    }

    pub fn set_item_disabled(
        &mut self,
        name: &str,
        disabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let ix = self.item_ix(name)?;
        if self.runtime[ix].disabled == disabled {
            return Ok(());
        }
        self.runtime[ix].disabled = disabled;
        if let Some(input) = self.items[ix].input() {
            let input_disabled = disabled || self.runtime[ix].input_disabled;
            input
                .state()
                .update(cx, |input, cx| input.set_disabled(input_disabled, cx));
        }
        self.complete = false;
        if self.current == Some(ix) && disabled {
            let next = self
                .enabled_indices()
                .find(|candidate| *candidate > ix)
                .or_else(|| {
                    self.enabled_indices()
                        .rev()
                        .find(|candidate| *candidate < ix)
                });
            self.current = next;
            self.focus_current_item(window, cx);
        } else if self.current.is_none() && !disabled {
            self.current = Some(ix);
            self.focus_current_item(window, cx);
        }
        cx.notify();
        Ok(())
    }

    pub fn set_choice_disabled(
        &mut self,
        item: &str,
        value: &str,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let item_ix = self.item_ix(item)?;
        let choice_ix = self.choice_ix(item_ix, value)?;
        if self.runtime[item_ix].choice_disabled[choice_ix] == disabled {
            return Ok(());
        }
        self.runtime[item_ix].choice_disabled[choice_ix] = disabled;
        self.answer_did_change(item_ix, false, cx);
        Ok(())
    }

    pub fn set_external_error(
        &mut self,
        item: &str,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let ix = self.item_ix(item)?;
        self.runtime[ix].external_error = Some(error.into());
        self.complete = false;
        cx.notify();
        Ok(())
    }

    pub fn clear_external_error(
        &mut self,
        item: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let ix = self.item_ix(item)?;
        self.runtime[ix].external_error = None;
        cx.notify();
        Ok(())
    }

    pub fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for ix in 0..self.items.len() {
            let initial = self.runtime[ix].initial_answer.clone();
            self.runtime[ix].answer = initial;
            self.runtime[ix].skipped = false;
            self.runtime[ix].validation_attempted = false;
            self.runtime[ix].internal_error = None;
            if let Some(input) = self.items[ix].input() {
                let value = self.runtime[ix]
                    .initial_input_value
                    .as_ref()
                    .cloned()
                    .unwrap_or_default();
                input
                    .state()
                    .update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
        self.complete = false;
        self.current = self
            .initial_current
            .as_ref()
            .and_then(|name| self.item_ix_opt(name));
        if self.current.is_some_and(|ix| self.runtime[ix].disabled) {
            let next = self.enabled_indices().next();
            self.current = next;
        }
        self.focus_current_item(window, cx);
        cx.notify();
    }

    pub fn activate_choice(
        &mut self,
        item: &str,
        value: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), QuestionnaireStateError> {
        let item_ix = self.item_ix(item)?;
        let choice_ix = self.choice_ix(item_ix, value)?;
        if self.runtime[item_ix].disabled || self.runtime[item_ix].choice_disabled[choice_ix] {
            return Ok(());
        }
        let before = self.effective_answer(item_ix);
        let before_status = self.status(item_ix);

        if self.items[item_ix].is_multiple() {
            let selected = self.runtime[item_ix]
                .answer
                .choices
                .iter()
                .position(|choice| choice.as_ref() == value);
            if let Some(ix) = selected {
                self.runtime[item_ix].answer.choices.remove(ix);
            } else {
                self.runtime[item_ix].answer.choices.push(value.into());
            }
        } else {
            self.runtime[item_ix].answer.choices.clear();
            self.runtime[item_ix].answer.choices.push(value.into());
            self.runtime[item_ix].answer.freeform = None;
        }
        self.runtime[item_ix].skipped = false;
        if before != self.effective_answer(item_ix) || before_status != self.status(item_ix) {
            self.answer_did_change(item_ix, true, cx);
        }
        Ok(())
    }

    pub fn confirm_current(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(current_ix) = self.current_ix() else {
            return false;
        };
        if current_ix + 1 == self.total() {
            self.submit(window, cx)
        } else {
            self.go_next(window, cx)
        }
    }

    pub fn go_previous(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(ix) = self.current_ix() else {
            return false;
        };
        let enabled: Vec<_> = self.enabled_indices().collect();
        if ix == 0 {
            return false;
        }
        self.change_current(Some(enabled[ix - 1]), true, window, cx);
        true
    }

    pub fn go_next(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(current) = self.current else {
            return false;
        };
        if !self.validate_item(current) {
            self.focus_invalid_item(self.items[current].name(), window, cx);
            cx.notify();
            return false;
        }
        let Some(ix) = self.current_ix() else {
            return false;
        };
        let enabled: Vec<_> = self.enabled_indices().collect();
        if ix + 1 >= enabled.len() {
            return false;
        }
        self.change_current(Some(enabled[ix + 1]), true, window, cx);
        true
    }

    pub fn skip_current(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(ix) = self.current else { return false };
        if self.items[ix].is_required() {
            return false;
        }
        self.runtime[ix].answer = QuestionnaireAnswer::new();
        self.runtime[ix].skipped = true;
        self.complete = false;
        self.emit_answer_changed(ix, cx);
        if self
            .current_ix()
            .is_some_and(|current| current + 1 == self.total())
        {
            self.submit(window, cx)
        } else {
            self.go_next(window, cx)
        }
    }

    pub fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let enabled: Vec<_> = self.enabled_indices().collect();
        let mut first_invalid = None;
        for ix in enabled {
            if !self.validate_item(ix) && first_invalid.is_none() {
                first_invalid = Some(ix);
            }
        }
        if let Some(ix) = first_invalid {
            self.change_current(Some(ix), true, window, cx);
            self.focus_invalid_item(self.items[ix].name(), window, cx);
            cx.notify();
            return false;
        }

        let submission = self.submission();
        if !self.complete {
            self.complete = true;
            cx.emit(QuestionnaireEvent::Completed(submission.clone()));
        }
        cx.emit(QuestionnaireEvent::Submit(submission));
        cx.notify();
        true
    }

    pub fn focus_current_item(&self, window: &mut Window, cx: &mut App) -> bool {
        let Some(ix) = self.current else { return false };
        self.runtime[ix].focus_handle.focus(window, cx);
        true
    }

    pub fn focus_invalid_item(&self, item: &str, window: &mut Window, cx: &mut App) -> bool {
        let Some(item_ix) = self.item_ix_opt(item) else {
            return false;
        };
        if self.runtime[item_ix].answer.freeform().is_some()
            && !self.runtime[item_ix].input_disabled
            && let Some(focus_handle) = &self.runtime[item_ix].input_focus_handle
        {
            focus_handle.focus(window, cx);
            return true;
        }
        for (choice_ix, choice) in self.items[item_ix].choices().iter().enumerate() {
            if self.runtime[item_ix]
                .answer
                .choices
                .contains(choice.value())
                && !self.runtime[item_ix].choice_disabled[choice_ix]
            {
                self.runtime[item_ix].choice_focus_handles[choice_ix].focus(window, cx);
                return true;
            }
        }
        if let Some(choice_ix) = self.runtime[item_ix]
            .choice_disabled
            .iter()
            .position(|disabled| !disabled)
        {
            self.runtime[item_ix].choice_focus_handles[choice_ix].focus(window, cx);
            return true;
        }
        if !self.runtime[item_ix].input_disabled
            && let Some(focus_handle) = &self.runtime[item_ix].input_focus_handle
        {
            focus_handle.focus(window, cx);
            return true;
        }
        self.runtime[item_ix].focus_handle.focus(window, cx);
        true
    }

    pub fn focus_choice(&self, item: &str, value: &str, window: &mut Window, cx: &mut App) -> bool {
        let Some(item_ix) = self.item_ix_opt(item) else {
            return false;
        };
        let Some(choice_ix) = self.choice_ix_opt(item_ix, value) else {
            return false;
        };
        self.runtime[item_ix].choice_focus_handles[choice_ix].focus(window, cx);
        true
    }

    pub fn focus_input(&self, item: &str, window: &mut Window, cx: &mut App) -> bool {
        let Some(item_ix) = self.item_ix_opt(item) else {
            return false;
        };
        let Some(focus_handle) = &self.runtime[item_ix].input_focus_handle else {
            return false;
        };
        focus_handle.focus(window, cx);
        true
    }

    pub fn focus_previous_answer(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.focus_adjacent_answer(-1, window, cx)
    }

    pub fn focus_next_answer(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.focus_adjacent_answer(1, window, cx)
    }

    pub fn move_current_radio(
        &mut self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(item_ix) = self.current else {
            return false;
        };
        if self.items[item_ix].is_multiple() || self.is_current_input_focused(window) {
            return false;
        }
        let enabled: Vec<_> = self.runtime[item_ix]
            .choice_disabled
            .iter()
            .enumerate()
            .filter_map(|(ix, disabled)| (!disabled).then_some(ix))
            .collect();
        if enabled.is_empty() {
            return false;
        }
        let current = enabled
            .iter()
            .position(|ix| self.runtime[item_ix].choice_focus_handles[*ix].is_focused(window))
            .or_else(|| {
                enabled.iter().position(|ix| {
                    self.runtime[item_ix]
                        .answer
                        .choices
                        .contains(self.items[item_ix].choices()[*ix].value())
                })
            });
        let target = match (current, direction.is_negative()) {
            (Some(ix), true) => ix.checked_sub(1).unwrap_or(enabled.len() - 1),
            (Some(ix), false) => (ix + 1) % enabled.len(),
            (None, true) => enabled.len() - 1,
            (None, false) => 0,
        };
        let choice_ix = enabled[target];
        let item = self.items[item_ix].name().clone();
        let choice = self.items[item_ix].choices()[choice_ix].value().clone();
        let _ = self.activate_choice(&item, &choice, cx);
        self.runtime[item_ix].choice_focus_handles[choice_ix].focus(window, cx);
        true
    }

    fn focus_adjacent_answer(
        &mut self,
        direction: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(item_ix) = self.current else {
            return false;
        };
        if (self.is_current_input_focused(window) && self.current_input_has_text(cx))
            || (!self.items[item_ix].is_multiple() && self.focused_current_choice(window).is_some())
        {
            return false;
        }

        #[derive(Clone, Copy)]
        enum Target {
            Choice(usize),
            Input,
        }
        let mut targets = Vec::new();
        for choice_ix in 0..self.items[item_ix].choices().len() {
            if !self.runtime[item_ix].choice_disabled[choice_ix] {
                targets.push(Target::Choice(choice_ix));
            }
        }
        if self.items[item_ix].input().is_some() && !self.runtime[item_ix].input_disabled {
            targets.push(Target::Input);
        }
        if targets.is_empty() {
            return false;
        }

        let focused = targets.iter().position(|target| match target {
            Target::Choice(ix) => {
                self.runtime[item_ix].choice_focus_handles[*ix].is_focused(window)
            }
            Target::Input => self.is_current_input_focused(window),
        });

        if focused.is_none() && self.runtime[item_ix].focus_handle.is_focused(window) {
            let filled: Vec<_> = targets
                .iter()
                .enumerate()
                .filter_map(|(ix, target)| match target {
                    Target::Choice(choice_ix)
                        if self.runtime[item_ix]
                            .answer
                            .choices
                            .contains(self.items[item_ix].choices()[*choice_ix].value()) =>
                    {
                        Some(ix)
                    }
                    Target::Input if self.runtime[item_ix].answer.freeform().is_some() => Some(ix),
                    _ => None,
                })
                .collect();
            let filled_ix = if direction.is_negative() {
                filled.last().copied()
            } else {
                filled.first().copied()
            };
            if let Some(filled_ix) = filled_ix {
                match targets[filled_ix] {
                    Target::Choice(choice_ix) => {
                        self.runtime[item_ix].choice_focus_handles[choice_ix].focus(window, cx);
                    }
                    Target::Input => {
                        if let Some(focus_handle) = &self.runtime[item_ix].input_focus_handle {
                            focus_handle.focus(window, cx);
                        }
                    }
                }
                return true;
            }
        }

        let target_ix = match (focused, direction.is_negative()) {
            (Some(ix), true) => ix.checked_sub(1).unwrap_or(targets.len() - 1),
            (Some(ix), false) => (ix + 1) % targets.len(),
            (None, true) => targets.len() - 1,
            (None, false) => 0,
        };
        match targets[target_ix] {
            Target::Choice(choice_ix) => {
                if !self.items[item_ix].is_multiple() {
                    let item = self.items[item_ix].name().clone();
                    let choice = self.items[item_ix].choices()[choice_ix].value().clone();
                    let _ = self.activate_choice(&item, &choice, cx);
                }
                self.runtime[item_ix].choice_focus_handles[choice_ix].focus(window, cx);
            }
            Target::Input => {
                if let Some(focus_handle) = &self.runtime[item_ix].input_focus_handle {
                    focus_handle.focus(window, cx);
                }
            }
        }
        true
    }

    fn on_input_change(&mut self, input: &Entity<InputState>, cx: &mut Context<Self>) {
        if let Some(ix) = self.input_item_ix(input) {
            self.sync_input_answer(ix, true, cx);
        }
    }

    fn sync_input_answer(&mut self, item_ix: usize, emit: bool, cx: &mut Context<Self>) {
        if self.runtime[item_ix].disabled || self.runtime[item_ix].input_disabled {
            return;
        }
        let before = self.effective_answer(item_ix);
        let before_status = self.status(item_ix);
        let Some(input) = self.items[item_ix].input() else {
            return;
        };
        let value = input.state().read(cx).value();
        if value.trim().is_empty() {
            self.runtime[item_ix].answer.freeform = None;
        } else {
            if !self.items[item_ix].is_multiple() {
                self.runtime[item_ix].answer.choices.clear();
            }
            self.runtime[item_ix].answer.freeform = Some(value);
            self.runtime[item_ix].skipped = false;
        }
        if before != self.effective_answer(item_ix) || before_status != self.status(item_ix) {
            self.answer_did_change(item_ix, emit, cx);
        }
    }

    fn answer_did_change(&mut self, item_ix: usize, emit: bool, cx: &mut Context<Self>) {
        if self.runtime[item_ix].validation_attempted {
            self.validate_item(item_ix);
        } else {
            self.runtime[item_ix].internal_error = None;
        }
        self.complete = false;
        if emit {
            self.emit_answer_changed(item_ix, cx);
        }
        cx.notify();
    }

    fn emit_answer_changed(&self, item_ix: usize, cx: &mut Context<Self>) {
        cx.emit(QuestionnaireEvent::AnswerChanged(
            QuestionnaireAnswerChange::new(
                self.items[item_ix].name().clone(),
                self.effective_answer(item_ix),
                self.status(item_ix),
            ),
        ));
    }

    fn validate_item(&mut self, item_ix: usize) -> bool {
        if self.runtime[item_ix].disabled || self.runtime[item_ix].skipped {
            return true;
        }
        self.runtime[item_ix].validation_attempted = true;
        let answer = self.effective_answer(item_ix);
        let error = if answer.is_empty() {
            Some(if self.items[item_ix].is_required() {
                t!("Questionnaire.error.required").into()
            } else {
                t!("Questionnaire.error.optional").into()
            })
        } else if let Some(validator) = self.items[item_ix].validator().cloned() {
            validator(&QuestionnaireValidationContext::new(
                self.items[item_ix].name().clone(),
                answer,
                self.answers(),
            ))
            .err()
        } else {
            None
        };
        self.runtime[item_ix].internal_error = error;
        self.error_at(item_ix).is_none()
    }

    fn submission(&self) -> QuestionnaireSubmission {
        QuestionnaireSubmission::new(
            self.enabled_indices()
                .map(|ix| {
                    QuestionnaireSubmissionItem::new(
                        self.items[ix].name().clone(),
                        self.status(ix),
                        self.effective_answer(ix),
                    )
                })
                .collect(),
        )
    }

    fn status(&self, item_ix: usize) -> QuestionnaireItemStatus {
        if self.runtime[item_ix].skipped {
            QuestionnaireItemStatus::Skipped
        } else if self.effective_answer(item_ix).is_empty() {
            QuestionnaireItemStatus::Unanswered
        } else {
            QuestionnaireItemStatus::Answered
        }
    }

    fn effective_answer(&self, item_ix: usize) -> QuestionnaireAnswer {
        if self.runtime[item_ix].disabled {
            return QuestionnaireAnswer::new();
        }
        let runtime = &self.runtime[item_ix];
        QuestionnaireAnswer {
            choices: self.items[item_ix]
                .choices()
                .iter()
                .filter(|choice| {
                    runtime.answer.choices.contains(choice.value())
                        && self
                            .choice_ix_opt(item_ix, choice.value())
                            .is_some_and(|ix| !runtime.choice_disabled[ix])
                })
                .map(|choice| choice.value().clone())
                .collect(),
            freeform: (!runtime.input_disabled)
                .then(|| runtime.answer.freeform.clone())
                .flatten(),
        }
    }

    fn error_at(&self, item_ix: usize) -> Option<&SharedString> {
        if self.runtime[item_ix].skipped || self.runtime[item_ix].disabled {
            return None;
        }
        self.runtime[item_ix].external_error.as_ref().or_else(|| {
            self.runtime[item_ix]
                .validation_attempted
                .then_some(self.runtime[item_ix].internal_error.as_ref())
                .flatten()
        })
    }

    fn check_answer(
        &self,
        item_ix: usize,
        answer: &QuestionnaireAnswer,
    ) -> Result<(), QuestionnaireStateError> {
        let item = &self.items[item_ix];
        let sources = answer.choices.len() + usize::from(answer.freeform.is_some());
        if (!item.is_multiple() && sources > 1)
            || (answer.freeform.is_some() && item.input().is_none())
        {
            return Err(QuestionnaireStateError::AnswerDoesNotMatchItem(
                item.name().clone(),
            ));
        }
        for choice in &answer.choices {
            let choice_ix = self.choice_ix(item_ix, choice)?;
            if self.runtime[item_ix].choice_disabled[choice_ix] {
                return Err(QuestionnaireStateError::AnswerDoesNotMatchItem(
                    item.name().clone(),
                ));
            }
        }
        Ok(())
    }

    fn change_current(
        &mut self,
        next: Option<usize>,
        emit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.current == next {
            return;
        }
        let previous = self.current.map(|ix| self.items[ix].name().clone());
        self.current = next;
        self.focus_current_item(window, cx);
        if emit {
            cx.emit(QuestionnaireEvent::CurrentItemChanged {
                previous,
                current: next.map(|ix| self.items[ix].name().clone()),
            });
        }
        cx.notify();
    }

    fn enabled_indices(&self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.runtime
            .iter()
            .enumerate()
            .filter_map(|(ix, runtime)| (!runtime.disabled).then_some(ix))
    }

    fn item_ix(&self, name: &str) -> Result<usize, QuestionnaireStateError> {
        self.item_ix_opt(name)
            .ok_or_else(|| QuestionnaireStateError::UnknownItem(name.into()))
    }

    fn item_ix_opt(&self, name: &str) -> Option<usize> {
        self.items
            .iter()
            .position(|item| item.name().as_ref() == name)
    }

    fn choice_ix(&self, item_ix: usize, value: &str) -> Result<usize, QuestionnaireStateError> {
        self.choice_ix_opt(item_ix, value)
            .ok_or_else(|| QuestionnaireStateError::UnknownChoice {
                item: self.items[item_ix].name().clone(),
                choice: value.into(),
            })
    }

    fn choice_ix_opt(&self, item_ix: usize, value: &str) -> Option<usize> {
        self.items[item_ix]
            .choices()
            .iter()
            .position(|choice| choice.value().as_ref() == value)
    }

    fn input_item_ix(&self, input: &Entity<InputState>) -> Option<usize> {
        self.items.iter().position(|item| {
            item.input()
                .is_some_and(|definition| definition.state().entity_id() == input.entity_id())
        })
    }
}

impl EventEmitter<QuestionnaireEvent> for QuestionnaireState {}

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext as _, Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext,
        div,
    };

    use super::*;

    struct Harness {
        state: Entity<QuestionnaireState>,
        first_input: Entity<InputState>,
        second_input: Entity<InputState>,
        events: Vec<&'static str>,
        _subscription: Subscription,
    }

    impl Harness {
        fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
            let first_input = cx.new(|cx| InputState::new(window, cx));
            let second_input =
                cx.new(|cx| InputState::new(window, cx).default_value("initial draft"));
            let items = vec![
                QuestionnaireItemDefinition::new("first", "First question")
                    .with_required(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("a", "A"),
                        QuestionnaireChoiceDefinition::new("b", "B"),
                    ])
                    .with_input(QuestionnaireInputDefinition::new(
                        first_input.clone(),
                        "Another answer",
                    )),
                QuestionnaireItemDefinition::new("second", "Second question")
                    .with_multiple(true)
                    .with_choices([
                        QuestionnaireChoiceDefinition::new("x", "X"),
                        QuestionnaireChoiceDefinition::new("y", "Y"),
                    ])
                    .with_input(QuestionnaireInputDefinition::new(
                        second_input.clone(),
                        "Another answer",
                    ))
                    .with_validator(|context| {
                        (context.answer().freeform().map(SharedString::as_ref) == Some("valid"))
                            .then_some(())
                            .ok_or_else(|| SharedString::from("Use the valid answer"))
                    }),
                QuestionnaireItemDefinition::new("disabled", "Disabled").with_disabled(true),
            ];
            let state = cx.new(|cx| {
                QuestionnaireState::new(items, cx)
                    .unwrap()
                    .with_shortcuts(QuestionnaireShortcutMode::Letters)
            });
            let subscription = cx.subscribe(&state, |this, _, event, _| {
                this.events.push(match event {
                    QuestionnaireEvent::CurrentItemChanged { .. } => "current",
                    QuestionnaireEvent::AnswerChanged(_) => "answer",
                    QuestionnaireEvent::Completed(_) => "completed",
                    QuestionnaireEvent::Submit(_) => "submit",
                });
            });
            Self {
                state,
                first_input,
                second_input,
                events: Vec::new(),
                _subscription: subscription,
            }
        }
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn harness(
        cx: &mut TestAppContext,
    ) -> (
        Entity<Harness>,
        Entity<QuestionnaireState>,
        Entity<InputState>,
        Entity<InputState>,
        &mut VisualTestContext,
    ) {
        cx.update(crate::init);
        let (harness, cx) = cx.add_window_view(Harness::new);
        let (state, first_input, second_input) = harness.read_with(cx, |harness, _| {
            (
                harness.state.clone(),
                harness.first_input.clone(),
                harness.second_input.clone(),
            )
        });
        (harness, state, first_input, second_input, cx)
    }

    #[test]
    fn schema_rejects_duplicate_names_and_invalid_single_defaults() {
        let duplicate_items = vec![
            QuestionnaireItemDefinition::new("same", "One"),
            QuestionnaireItemDefinition::new("same", "Two"),
        ];
        assert_eq!(
            QuestionnaireState::validate_schema(&duplicate_items),
            Err(QuestionnaireStateError::DuplicateItem("same".into()))
        );

        let invalid_default = vec![
            QuestionnaireItemDefinition::new("single", "Single").with_choices([
                QuestionnaireChoiceDefinition::new("a", "A").with_default_selected(true),
                QuestionnaireChoiceDefinition::new("b", "B").with_default_selected(true),
            ]),
        ];
        assert_eq!(
            QuestionnaireState::validate_schema(&invalid_default),
            Err(QuestionnaireStateError::MultipleDefaultsForSingleItem(
                "single".into()
            ))
        );

        let duplicate_choice = vec![
            QuestionnaireItemDefinition::new("item", "Item").with_choices([
                QuestionnaireChoiceDefinition::new("same", "One"),
                QuestionnaireChoiceDefinition::new("same", "Two"),
            ]),
        ];
        assert_eq!(
            QuestionnaireState::validate_schema(&duplicate_choice),
            Err(QuestionnaireStateError::DuplicateChoice {
                item: "item".into(),
                choice: "same".into(),
            })
        );
    }

    #[gpui::test]
    fn validates_navigates_skips_and_emits_completion_before_submit(cx: &mut TestAppContext) {
        let (harness, state, _, _, cx) = harness(cx);

        assert_eq!(cx.read(|cx| state.read(cx).progress().current()), 1);
        assert_eq!(cx.read(|cx| state.read(cx).progress().total()), 2);
        assert_eq!(
            cx.read(|cx| state.read(cx).item_state("first").unwrap().status()),
            QuestionnaireItemStatus::Unanswered
        );
        assert!(!cx.update(|window, cx| state.update(cx, |state, cx| state.submit(window, cx))));
        assert!(cx.read(|cx| state.read(cx).error("first").is_some()));
        assert_eq!(
            cx.read(|cx| state.read(cx).error("second").unwrap().clone()),
            "Use the valid answer"
        );

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_input_value("first", "draft", window, cx).unwrap();
            });
        });
        assert!(cx.read(|cx| state.read(cx).error("first").is_none()));
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_input_value("first", "", window, cx).unwrap();
            });
        });
        assert!(
            cx.read(|cx| state.read(cx).error("first").is_some()),
            "once validation has been attempted, clearing an answer updates the error live"
        );

        state.update(cx, |state, cx| {
            state.activate_choice("first", "a", cx).unwrap()
        });
        assert!(
            cx.update(|window, cx| { state.update(cx, |state, cx| state.go_next(window, cx)) })
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).current_item().unwrap().clone()),
            "second"
        );
        assert!(
            cx.update(|window, cx| {
                state.update(cx, |state, cx| state.skip_current(window, cx))
            })
        );

        assert!(cx.read(|cx| state.read(cx).is_complete()));
        assert_eq!(
            cx.read(|cx| state.read(cx).item_state("second").unwrap().status()),
            QuestionnaireItemStatus::Skipped
        );
        assert_eq!(
            cx.read(|cx| harness.read(cx).events.clone()),
            vec!["answer", "current", "answer", "completed", "submit"]
        );
    }

    #[gpui::test]
    fn keeps_input_draft_separate_and_synchronizes_silent_setters_and_reset(
        cx: &mut TestAppContext,
    ) {
        let (harness, state, first_input, second_input, cx) = harness(cx);
        let initial_events = cx.read(|cx| harness.read(cx).events.len());

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_answer(
                        "first",
                        QuestionnaireAnswer::new().with_freeform("  custom  "),
                        window,
                        cx,
                    )
                    .unwrap();
            });
        });
        assert_eq!(
            cx.read(|cx| harness.read(cx).events.len()),
            initial_events,
            "programmatic setters are silent"
        );
        assert_eq!(cx.read(|cx| first_input.read(cx).value()), "  custom  ");
        assert_eq!(
            cx.read(|cx| {
                state
                    .read(cx)
                    .answer("first")
                    .unwrap()
                    .freeform()
                    .unwrap()
                    .clone()
            }),
            "  custom  "
        );

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_answer(
                        "first",
                        QuestionnaireAnswer::new().with_choices(["a"]),
                        window,
                        cx,
                    )
                    .unwrap();
            });
        });
        assert_eq!(cx.read(|cx| first_input.read(cx).value()), "  custom  ");
        assert!(cx.read(|cx| { state.read(cx).answer("first").unwrap().freeform().is_none() }));
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("first").unwrap().choices()[0].clone()),
            "a"
        );

        cx.update(|window, cx| {
            first_input.update(cx, |input, cx| input.replace_all("", window, cx));
        });
        cx.run_until_parked();
        assert_eq!(
            cx.read(|cx| harness.read(cx).events.len()),
            initial_events,
            "editing an unselected draft does not change the semantic answer"
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("first").unwrap().choices()[0].clone()),
            "a"
        );

        state.update(cx, |state, cx| {
            state.activate_choice("first", "b", cx).unwrap()
        });

        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.reset(window, cx));
        });
        assert_eq!(cx.read(|cx| first_input.read(cx).value()), "");
        assert_eq!(cx.read(|cx| second_input.read(cx).value()), "initial draft");
        assert!(cx.read(|cx| state.read(cx).answer("first").unwrap().is_empty()));
        assert_eq!(
            cx.read(|cx| {
                state
                    .read(cx)
                    .answer("second")
                    .unwrap()
                    .freeform()
                    .unwrap()
                    .clone()
            }),
            "initial draft"
        );
        assert_eq!(
            cx.read(|cx| harness.read(cx).events.len()),
            initial_events + 1
        );
    }

    #[gpui::test]
    fn validates_all_items_and_returns_to_the_first_invalid_item(cx: &mut TestAppContext) {
        let (_, state, _, _, cx) = harness(cx);
        state.update(cx, |state, cx| {
            state.activate_choice("first", "a", cx).unwrap()
        });

        assert!(
            !cx.update(|window, cx| { state.update(cx, |state, cx| state.submit(window, cx)) })
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).current_item().unwrap().clone()),
            "second"
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).error("second").unwrap().clone()),
            "Use the valid answer"
        );

        state.update(cx, |state, cx| {
            state
                .set_external_error("first", "Server rejected it", cx)
                .unwrap()
        });
        assert!(
            !cx.update(|window, cx| { state.update(cx, |state, cx| state.submit(window, cx)) })
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).current_item().unwrap().clone()),
            "first"
        );

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.clear_external_error("first", cx).unwrap();
                state
                    .set_input_value("second", "valid", window, cx)
                    .unwrap();
            });
        });
        assert!(cx.update(|window, cx| { state.update(cx, |state, cx| state.submit(window, cx)) }));
    }

    #[gpui::test]
    fn preserves_schema_order_and_temporarily_excludes_disabled_answers(cx: &mut TestAppContext) {
        let (_, state, _, _, cx) = harness(cx);

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_answer(
                        "second",
                        QuestionnaireAnswer::new()
                            .with_choices(["y", "x", "y"])
                            .with_freeform("valid"),
                        window,
                        cx,
                    )
                    .unwrap();
            });
        });
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("second").unwrap().choices().to_vec()),
            vec![SharedString::from("x"), SharedString::from("y")]
        );
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_current_item("second", window, cx).unwrap();
                state.focus_current_item(window, cx);
                assert!(state.focus_next_answer(window, cx));
            });
        });
        assert_eq!(
            cx.update(|window, cx| state.read(cx).focused_current_choice(window).cloned()),
            Some("x".into()),
            "filled multiple choices are focused in schema order"
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("second").unwrap().choices().to_vec()),
            vec![SharedString::from("x"), SharedString::from("y")],
            "focusing a filled choice does not toggle it"
        );

        state.update(cx, |state, cx| {
            state.set_choice_disabled("second", "x", true, cx).unwrap()
        });
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("second").unwrap().choices().to_vec()),
            vec![SharedString::from("y")]
        );
        state.update(cx, |state, cx| {
            state.set_choice_disabled("second", "x", false, cx).unwrap()
        });
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("second").unwrap().choices().to_vec()),
            vec![SharedString::from("x"), SharedString::from("y")]
        );

        let error = cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_answer(
                    "second",
                    QuestionnaireAnswer::new().with_choices(["unknown"]),
                    window,
                    cx,
                )
            })
        });
        assert_eq!(
            error,
            Err(QuestionnaireStateError::UnknownChoice {
                item: "second".into(),
                choice: "unknown".into(),
            })
        );
    }

    #[gpui::test]
    fn shortcuts_disabled_current_fallback_and_recompletion_are_deterministic(
        cx: &mut TestAppContext,
    ) {
        let (harness, state, first_input, _, cx) = harness(cx);
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_answer(
                        "first",
                        QuestionnaireAnswer::new().with_choices(["b"]),
                        window,
                        cx,
                    )
                    .unwrap();
                state.focus_current_item(window, cx);
                assert!(state.focus_next_answer(window, cx));
            });
        });
        assert_eq!(
            cx.update(|window, cx| state.read(cx).focused_current_choice(window).cloned()),
            Some("b".into()),
            "the first move from the item group focuses the existing answer"
        );
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("first").unwrap().choices()[0].clone()),
            "b",
            "focusing the filled radio does not replace the answer"
        );
        cx.update(|window, cx| state.update(cx, |state, cx| state.reset(window, cx)));

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.focus_input("first", window, cx);
                assert!(!state.current_input_has_text(cx));
                assert!(state.focus_next_answer(window, cx));
            });
        });
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("first").unwrap().choices()[0].clone()),
            "a",
            "an empty focused input may move to and activate a radio"
        );
        cx.update(|window, cx| state.update(cx, |state, cx| state.reset(window, cx)));

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_input_value("first", "draft", window, cx).unwrap();
                state.focus_input("first", window, cx);
                assert!(state.current_input_has_text(cx));
                assert!(!state.focus_next_answer(window, cx));
            });
        });
        assert!(cx.update(|window, cx| first_input.focus_handle(cx).is_focused(window)));
        cx.update(|window, cx| state.update(cx, |state, cx| state.reset(window, cx)));

        assert!(cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.focus_current_item(window, cx);
                state.focus_next_answer(window, cx)
            })
        }));
        assert_eq!(
            cx.read(|cx| state.read(cx).answer("first").unwrap().choices()[0].clone()),
            "a",
            "moving from the item group to a radio activates it"
        );
        cx.update(|window, cx| state.update(cx, |state, cx| state.reset(window, cx)));

        assert_eq!(
            cx.read(|cx| state.read(cx).shortcut_for_choice("first", "a")),
            Some("A".into())
        );
        state.update(cx, |state, cx| {
            state.set_choice_disabled("first", "a", true, cx).unwrap()
        });
        assert_eq!(
            cx.read(|cx| state.read(cx).shortcut_for_choice("first", "b")),
            Some("A".into())
        );
        state.update(cx, |state, cx| {
            state.set_choice_disabled("first", "a", false, cx).unwrap()
        });
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                assert!(state.activate_shortcut("a", window, cx));
                state.focus_choice("first", "a", window, cx);
                assert!(state.move_current_radio(1, window, cx));
                assert!(state.go_next(window, cx));
                assert!(state.skip_current(window, cx));
            });
        });
        assert!(cx.read(|cx| state.read(cx).is_complete()));

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state
                    .set_input_value("second", "valid", window, cx)
                    .unwrap();
                state.activate_choice("second", "x", cx).unwrap();
            });
        });
        assert!(!cx.read(|cx| state.read(cx).is_complete()));
        assert!(cx.update(|window, cx| { state.update(cx, |state, cx| state.submit(window, cx)) }));
        let events = cx.read(|cx| harness.read(cx).events.clone());
        assert_eq!(
            events.iter().filter(|event| **event == "completed").count(),
            2
        );

        let before_disable = events.len();
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_item_disabled("second", true, window, cx).unwrap();
            });
        });
        assert_eq!(
            cx.read(|cx| state.read(cx).current_item().unwrap().clone()),
            "first"
        );
        assert_eq!(
            cx.read(|cx| harness.read(cx).events.len()),
            before_disable,
            "programmatic disable and fallback are silent"
        );
    }
}
