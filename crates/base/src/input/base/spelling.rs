//! Spell checking: the words a [`SpellChecker`] does not know are underlined
//! as they are typed, and the context menu opened on one offers what the
//! checker suggests for it.

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{Action, App, Context, SharedString, Window, actions};
use serde::Deserialize;

use super::{InputBaseState, kind::InputModeKind};

/// What tells an input's words spelled right from wrong: the platform's spell
/// checker, a dictionary the app ships, or anything else that can.
///
/// Give one to an input with [`InputBaseState::spell_checker`]. An input
/// without one checks nothing.
pub trait SpellChecker {
    /// The UTF-8 byte ranges of the words in `text` that are misspelled, in
    /// order.
    fn misspelled_words(&self, text: &str, cx: &App) -> Vec<Range<usize>>;

    /// What `word` may have been meant to be, best first.
    fn guesses(&self, word: &str, cx: &App) -> Vec<SharedString>;

    /// Accept `word` as spelled right from now on.
    fn learn(&self, word: &str, cx: &mut App);
}

/// Replace the misspelled word the context menu was opened on.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = input, no_json)]
pub struct CorrectSpelling {
    /// The word to put in its place.
    pub replacement: SharedString,
}

actions!(
    input,
    [
        /// Accept the misspelled word the context menu was opened on.
        LearnSpelling
    ]
);

/// A misspelled word: where it is, what it is, and the checker's guesses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Misspelling {
    /// Its UTF-8 byte range in the input's text.
    pub range: Range<usize>,
    pub word: SharedString,
    pub guesses: Vec<SharedString>,
}

/// An input's spell checker, and what it last found.
pub(crate) struct Spelling {
    checker: Rc<dyn SpellChecker>,
    /// The misspelled ranges, and the revision of the text they were found in.
    found: RefCell<Option<(u64, Rc<[Range<usize>]>)>>,
}

impl Spelling {
    fn new(checker: Rc<dyn SpellChecker>) -> Self {
        Self {
            checker,
            found: RefCell::new(None),
        }
    }
}

impl<M: InputModeKind> InputBaseState<M> {
    /// Check the input's spelling with `checker`.
    pub fn spell_checker(mut self, checker: impl SpellChecker + 'static) -> Self {
        self.spelling = Some(Spelling::new(Rc::new(checker)));
        self
    }

    /// Check the input's spelling with `checker`, or with `None` not at all.
    pub fn set_spell_checker(
        &mut self,
        checker: Option<Rc<dyn SpellChecker>>,
        cx: &mut Context<Self>,
    ) {
        self.spelling = checker.map(Spelling::new);
        cx.notify();
    }

    /// The UTF-8 byte ranges of the misspelled words, checked again only
    /// when the text has changed. None while the input is masked.
    pub fn misspelled_words(&self, cx: &App) -> Rc<[Range<usize>]> {
        let Some(spelling) = self.spelling.as_ref().filter(|_| !self.masked) else {
            return Rc::new([]);
        };
        let mut found = spelling.found.borrow_mut();
        if let Some((revision, ranges)) = found.as_ref()
            && *revision == self.document_revision
        {
            return ranges.clone();
        }
        let ranges: Rc<[Range<usize>]> = spelling
            .checker
            .misspelled_words(&self.text.to_string(), cx)
            .into();
        *found = Some((self.document_revision, ranges.clone()));
        ranges
    }

    /// The misspelled words to underline: all but one the cursor is at the
    /// end of, which is still being typed.
    pub(crate) fn misspellings_shown(&self, cx: &App) -> Vec<Range<usize>> {
        let cursor = self.cursor();
        let typing = self.active_selection().is_empty();
        self.misspelled_words(cx)
            .iter()
            .filter(|range| !(typing && range.end == cursor))
            .cloned()
            .collect()
    }

    /// The misspelled word at the UTF-8 offset `offset`, if one is there.
    pub fn misspelling_at(&self, offset: usize, cx: &App) -> Option<Misspelling> {
        let spelling = self.spelling.as_ref()?;
        let range = self
            .misspelled_words(cx)
            .iter()
            .find(|range| range.start <= offset && offset <= range.end)?
            .clone();
        let word: SharedString = self.text.slice(range.clone()).to_string().into();
        let guesses = spelling.checker.guesses(&word, cx);
        Some(Misspelling {
            range,
            word,
            guesses,
        })
    }

    /// The misspelled word the context menu was last opened on, if it was
    /// opened on one.
    pub fn context_menu_misspelling(&self, cx: &App) -> Option<Misspelling> {
        self.misspelling_at(self.context_menu_offset()?, cx)
    }

    /// The misspelled word an action acts on: the one the context menu was
    /// opened on, else the one at the cursor.
    fn misspelling_acted_on(&self, cx: &App) -> Option<Misspelling> {
        self.misspelling_at(self.context_menu_offset().unwrap_or(self.cursor()), cx)
    }

    pub(super) fn correct_spelling(
        &mut self,
        action: &CorrectSpelling,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.is_editable() {
            return;
        }
        let Some(misspelling) = self.misspelling_acted_on(cx) else {
            return;
        };
        self.set_selection(misspelling.range.start, misspelling.range.end);
        self.replace(action.replacement.clone(), window, cx);
    }

    pub(super) fn learn_spelling(
        &mut self,
        _: &LearnSpelling,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(misspelling) = self.misspelling_acted_on(cx) else {
            return;
        };
        let Some(spelling) = self.spelling.as_ref() else {
            return;
        };
        let checker = spelling.checker.clone();
        spelling.found.borrow_mut().take();
        checker.learn(&misspelling.word, cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, ops::Range, rc::Rc};

    use gpui::{App, SharedString};

    use super::{CorrectSpelling, LearnSpelling, SpellChecker};
    use crate::input::TextareaState;

    /// Knows the words it was given and those it learned; guesses "world"
    /// for anything else.
    struct Words(Rc<RefCell<Vec<String>>>);

    impl SpellChecker for Words {
        fn misspelled_words(&self, text: &str, _: &App) -> Vec<Range<usize>> {
            let known = self.0.borrow();
            let mut found = Vec::new();
            let mut start = 0;
            for word in text.split(' ') {
                if !word.is_empty() && !known.iter().any(|k| k == word) {
                    found.push(start..start + word.len());
                }
                start += word.len() + 1;
            }
            found
        }

        fn guesses(&self, _: &str, _: &App) -> Vec<SharedString> {
            vec!["world".into()]
        }

        fn learn(&self, word: &str, _: &mut App) {
            self.0.borrow_mut().push(word.to_owned());
        }
    }

    #[gpui::test]
    fn misspelled_words_are_found_corrected_and_learned(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let known = Rc::new(RefCell::new(vec!["hello".to_owned(), "world".to_owned()]));
        let checker = Words(known.clone());
        let (state, cx) = cx.add_window_view(|window, cx| {
            TextareaState::new(window, cx)
                .spell_checker(checker)
                .default_value("hello wrold zorp")
        });
        let misspelled = |cx: &mut gpui::VisualTestContext| {
            state.read_with(cx, |state, cx| state.misspelled_words(cx).to_vec())
        };
        assert_eq!(misspelled(cx), vec![6..11, 12..16]);

        // A word is not marked while it is being typed at its end.
        cx.update(|_, cx| state.update(cx, |state, cx| state.set_selected_range(16..16, cx)));
        let shown = state.read_with(cx, |state, cx| state.misspellings_shown(cx));
        assert_eq!(shown, vec![6..11]);

        let found = state.read_with(cx, |state, cx| state.misspelling_at(8, cx));
        assert_eq!(
            found.map(|m| (m.word, m.guesses)),
            Some(("wrold".into(), vec!["world".into()]))
        );

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_selected_range(8..8, cx);
                let world = CorrectSpelling {
                    replacement: "world".into(),
                };
                state.correct_spelling(&world, window, cx);
            })
        });
        assert_eq!(
            state.read_with(cx, |state, _| state.value()),
            "hello world zorp"
        );
        assert_eq!(misspelled(cx), vec![12..16]);

        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.set_selected_range(13..13, cx);
                state.learn_spelling(&LearnSpelling, window, cx);
            })
        });
        assert!(known.borrow().iter().any(|k| k == "zorp"));
        assert!(misspelled(cx).is_empty());
    }
}
