use gpui::{App, SharedString, Task, Window};

use crate::IndexPath;

use super::delegate::{SearchableListDelegate, SearchableListItem};

// MARK: Primitive impls

impl SearchableListItem for String {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(self.clone())
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

impl SearchableListItem for SharedString {
    type Value = Self;

    fn title(&self) -> SharedString {
        self.clone()
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

impl SearchableListItem for &'static str {
    type Value = Self;

    fn title(&self) -> SharedString {
        SharedString::from(*self)
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

// MARK: Vec delegate

impl<T: SearchableListItem + 'static> SearchableListDelegate for Vec<T> {
    type Item = T;

    fn items_count(&self, _: usize) -> usize {
        self.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.as_slice().get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.iter()
            .position(|v| v.value() == value)
            .map(|ix| IndexPath::default().row(ix))
    }
}

// MARK: SearchableVec

/// A vector of items that supports incremental filtering.
///
/// On each `perform_search` call the `matched_items` view is rebuilt by filtering
/// the full `items` list.  Use this as a delegate when all data is already in memory.
#[derive(Debug, Clone)]
pub struct SearchableVec<T> {
    items: Vec<T>,
    matched_items: Vec<T>,
}

impl<T: Clone> SearchableVec<T> {
    /// Create a new `SearchableVec` from an initial list of items.
    pub fn new(items: impl Into<Vec<T>>) -> Self {
        let items = items.into();

        Self {
            items: items.clone(),
            matched_items: items,
        }
    }

    /// Append an item to both the master list and the current filtered view.
    pub fn push(&mut self, item: T) {
        self.items.push(item.clone());
        self.matched_items.push(item);
    }
}

impl<T: SearchableListItem> From<Vec<T>> for SearchableVec<T> {
    fn from(items: Vec<T>) -> Self {
        Self {
            items: items.clone(),
            matched_items: items,
        }
    }
}

impl<I: SearchableListItem + 'static> SearchableListDelegate for SearchableVec<I> {
    type Item = I;

    fn items_count(&self, _: usize) -> usize {
        self.matched_items.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.matched_items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        self.matched_items
            .iter()
            .position(|v| v.value() == value)
            .map(|ix| IndexPath::default().row(ix))
    }

    fn perform_search(&mut self, query: &str, _: &mut Window, _: &mut App) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.matches(query))
            .cloned()
            .collect();

        Task::ready(())
    }
}

// MARK: SearchableGroup

/// A named group of items used for sectioned lists.
#[derive(Debug, Clone)]
pub struct SearchableGroup<I: SearchableListItem> {
    pub title: SharedString,
    pub items: Vec<I>,
}

impl<I: SearchableListItem> SearchableGroup<I> {
    /// Create an empty group with the given section title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            items: vec![],
        }
    }

    /// Append a single item to this group.
    pub fn item(mut self, item: I) -> Self {
        self.items.push(item);
        self
    }

    /// Append multiple items to this group.
    pub fn items(mut self, items: impl IntoIterator<Item = I>) -> Self {
        self.items.extend(items);
        self
    }

    pub(super) fn matches(&self, query: &str) -> bool {
        self.title.to_lowercase().contains(&query.to_lowercase())
            || self.items.iter().any(|item| item.matches(query))
    }
}

impl<I: SearchableListItem + 'static> SearchableListDelegate for SearchableVec<SearchableGroup<I>> {
    type Item = I;

    fn sections_count(&self, _: &App) -> usize {
        self.matched_items.len()
    }

    fn items_count(&self, section: usize) -> usize {
        self.matched_items
            .get(section)
            .map_or(0, |group| group.items.len())
    }

    fn section(&self, section: usize) -> Option<gpui::AnyElement> {
        use gpui::IntoElement as _;

        Some(
            self.matched_items
                .get(section)?
                .title
                .clone()
                .into_any_element(),
        )
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        let section = self.matched_items.get(ix.section)?;

        section.items.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: SearchableListItem<Value = V>,
        V: PartialEq,
    {
        for (ix, group) in self.matched_items.iter().enumerate() {
            for (row_ix, item) in group.items.iter().enumerate() {
                if item.value() == value {
                    return Some(IndexPath::default().section(ix).row(row_ix));
                }
            }
        }

        None
    }

    fn perform_search(&mut self, query: &str, _: &mut Window, _: &mut App) -> Task<()> {
        self.matched_items = self
            .items
            .iter()
            .filter(|item| item.matches(query))
            .cloned()
            .map(|mut item| {
                item.items.retain(|item| item.matches(query));
                item
            })
            .collect();

        Task::ready(())
    }
}
