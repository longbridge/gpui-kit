//! The arithmetic of a grid layout: where items may sit, what pushes what, and
//! how a layout settles. Everything here is pure, in grid cells, and knows
//! nothing about pixels, entities or rendering.

use gpui::{Axis, SharedString};
use serde::{Deserialize, Serialize};

/// One item's place in a grid layout, measured in grid cells.
///
/// `x` counts columns from the leading edge and `y` counts rows from the top;
/// `w` and `h` are the item's span. A layout is a list of these, one per item,
/// keyed by `id`, which is what an application persists to restore a user's
/// arrangement.
///
/// ```ignore
/// GridItem::new("revenue", 0, 0, 6, 4).with_min_size(3, 2)
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GridItem {
    /// The stable identity that pairs this entry with its rendered item.
    pub id: SharedString,
    /// The leading column.
    pub x: usize,
    /// The top row.
    pub y: usize,
    /// The number of columns spanned; at least 1.
    pub w: usize,
    /// The number of rows spanned; at least 1.
    pub h: usize,
    /// The narrowest a resize may make the item.
    #[serde(default = "one")]
    pub min_w: usize,
    /// The shortest a resize may make the item.
    #[serde(default = "one")]
    pub min_h: usize,
    /// The widest a resize may make the item; `None` allows the full width.
    #[serde(default)]
    pub max_w: Option<usize>,
    /// The tallest a resize may make the item; `None` is unbounded.
    #[serde(default)]
    pub max_h: Option<usize>,
    /// A locked item is neither dragged, resized nor pushed; other items flow
    /// around it.
    #[serde(default)]
    pub locked: bool,
}

fn one() -> usize {
    1
}

impl GridItem {
    /// An item at column `x`, row `y`, spanning `w` columns and `h` rows.
    pub fn new(id: impl Into<SharedString>, x: usize, y: usize, w: usize, h: usize) -> Self {
        Self {
            id: id.into(),
            x,
            y,
            w: w.max(1),
            h: h.max(1),
            min_w: 1,
            min_h: 1,
            max_w: None,
            max_h: None,
            locked: false,
        }
    }

    /// The smallest span a resize may reach.
    pub fn with_min_size(mut self, w: usize, h: usize) -> Self {
        self.min_w = w.max(1);
        self.min_h = h.max(1);
        self
    }

    /// The largest span a resize may reach.
    pub fn with_max_size(mut self, w: usize, h: usize) -> Self {
        self.max_w = Some(w.max(1));
        self.max_h = Some(h.max(1));
        self
    }

    /// Lock the item in place; see [`GridItem::locked`].
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// The row just below the item.
    pub fn bottom(&self) -> usize {
        self.y + self.h
    }

    /// The column just past the item's trailing edge.
    pub fn right(&self) -> usize {
        self.x + self.w
    }

    fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }

    /// Clamp a requested span to the item's limits and the grid's width.
    pub(crate) fn clamp_size(&self, w: usize, h: usize, columns: usize) -> (usize, usize) {
        let max_w = self
            .max_w
            .unwrap_or(usize::MAX)
            .min(columns.saturating_sub(self.x))
            .max(1);
        let max_h = self.max_h.unwrap_or(usize::MAX).max(1);
        (
            w.clamp(self.min_w.min(max_w), max_w),
            h.clamp(self.min_h.min(max_h), max_h),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Rect {
    fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

/// How a layout responds to edits, shared by every operation in this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutRules {
    pub columns: usize,
    /// The direction items float toward; `None` leaves them where they are put.
    pub compaction: Option<Axis>,
    /// Reject a move or resize that would overlap another item instead of
    /// pushing that item away.
    pub prevent_collision: bool,
}

/// The number of rows a layout occupies.
pub(crate) fn rows(items: &[GridItem]) -> usize {
    items.iter().map(GridItem::bottom).max().unwrap_or(0)
}

/// Bring a layout into a valid state: every item fits the columns, no locked
/// items overlap, and the remainder are compacted around them.
pub(crate) fn normalize(items: &mut [GridItem], rules: LayoutRules) {
    let columns = rules.columns.max(1);
    for item in items.iter_mut() {
        item.w = item.w.clamp(1, columns);
        item.h = item.h.max(1);
        if item.right() > columns {
            item.x = columns - item.w;
        }
    }

    // Locked items cannot be pushed by others, so settle them among
    // themselves first, in reading order.
    let locked = sorted_indices(items, rules.compaction)
        .into_iter()
        .filter(|&ix| items[ix].locked)
        .collect::<Vec<_>>();
    for (n, &ix) in locked.iter().enumerate() {
        while let Some(other) = first_collision(items, &locked[..n], items[ix].rect(), ix) {
            items[ix].y = items[other].bottom();
        }
    }

    settle(items, rules, None);
}

/// Compact the layout, or without compaction just push overlapping items
/// down. `priority` names an item that keeps its place when it overlaps
/// another — the one the user is moving.
fn settle(items: &mut [GridItem], rules: LayoutRules, priority: Option<usize>) {
    if rules.compaction.is_some() {
        compact(items, rules);
        return;
    }
    let mut placed = (0..items.len())
        .filter(|&ix| items[ix].locked)
        .collect::<Vec<_>>();
    if let Some(priority) = priority.filter(|&ix| !items[ix].locked) {
        placed.push(priority);
    }
    for ix in sorted_indices(items, None) {
        if placed.contains(&ix) {
            continue;
        }
        while let Some(other) = first_collision(items, &placed, items[ix].rect(), ix) {
            items[ix].y = items[other].bottom();
        }
        placed.push(ix);
    }
}

/// Float every unlocked item toward `axis` as far as the others allow.
pub(crate) fn compact(items: &mut [GridItem], rules: LayoutRules) {
    let Some(axis) = rules.compaction else {
        return;
    };
    let columns = rules.columns.max(1);
    let mut placed = (0..items.len())
        .filter(|&ix| items[ix].locked)
        .collect::<Vec<_>>();
    for ix in sorted_indices(items, Some(axis)) {
        if items[ix].locked {
            continue;
        }
        compact_item(items, &placed, ix, axis, columns);
        placed.push(ix);
    }
}

fn compact_item(items: &mut [GridItem], placed: &[usize], ix: usize, axis: Axis, columns: usize) {
    match axis {
        Axis::Vertical => {
            // Start no lower than the bottom of what is already placed, so a
            // tall gap closes in one step instead of row by row.
            let floor = placed.iter().map(|&p| items[p].bottom()).max().unwrap_or(0);
            items[ix].y = items[ix].y.min(floor);
            loop {
                while items[ix].y > 0 {
                    let mut probe = items[ix].rect();
                    probe.y -= 1;
                    if first_collision(items, placed, probe, ix).is_some() {
                        break;
                    }
                    items[ix].y -= 1;
                }
                match first_collision(items, placed, items[ix].rect(), ix) {
                    Some(other) => items[ix].y = items[other].bottom(),
                    None => break,
                }
            }
        }
        Axis::Horizontal => loop {
            while items[ix].x > 0 {
                let mut probe = items[ix].rect();
                probe.x -= 1;
                if first_collision(items, placed, probe, ix).is_some() {
                    break;
                }
                items[ix].x -= 1;
            }
            // Past the trailing edge, wrap to the next row and try again from
            // the leading edge.
            if items[ix].right() > columns {
                items[ix].x = 0;
                items[ix].y += 1;
                continue;
            }
            match first_collision(items, placed, items[ix].rect(), ix) {
                Some(other) => {
                    items[ix].x = items[other].right();
                    if items[ix].right() > columns {
                        items[ix].x = 0;
                        items[ix].y += 1;
                    }
                }
                None => break,
            }
        },
    }
}

/// Move the item `id` to `(x, y)`, pushing whatever it lands on out of the
/// way, then compact. Returns `None` when the rules refuse the move.
///
/// The move is computed from `items` as they were when the gesture began, not
/// from the previous step of the gesture, so items the pointer merely passed
/// over return to where they were.
pub(crate) fn move_item(
    items: &[GridItem],
    id: &str,
    x: usize,
    y: usize,
    rules: LayoutRules,
) -> Option<Vec<GridItem>> {
    let ix = items.iter().position(|item| item.id.as_ref() == id)?;
    if items[ix].locked {
        return None;
    }
    let mut next = items.to_vec();
    let x = x.min(rules.columns.saturating_sub(next[ix].w));
    if rules.prevent_collision {
        let mut probe = next[ix].rect();
        probe.x = x;
        probe.y = y;
        if any_collision(&next, probe, ix) {
            return None;
        }
    }
    Push::new(rules, next.len()).move_element(&mut next, ix, Some(x), Some(y), true);
    settle(&mut next, rules, Some(ix));
    Some(next)
}

/// Resize the item `id` to span `w` by `h`, clamped to its limits, pushing
/// whatever it grows over, then compact. Returns `None` when the rules refuse
/// the resize.
pub(crate) fn resize_item(
    items: &[GridItem],
    id: &str,
    w: usize,
    h: usize,
    rules: LayoutRules,
) -> Option<Vec<GridItem>> {
    let ix = items.iter().position(|item| item.id.as_ref() == id)?;
    if items[ix].locked {
        return None;
    }
    let mut next = items.to_vec();
    let (w, h) = next[ix].clamp_size(w, h, rules.columns);

    // A locked neighbour is a wall. Rather than refuse the whole gesture, try
    // keeping the other dimension where it started.
    let start = next[ix].clone();
    let fits = |w: usize, h: usize, next: &[GridItem]| {
        let probe = Rect {
            w,
            h,
            ..start.rect()
        };
        !next
            .iter()
            .enumerate()
            .any(|(other, item)| other != ix && item.locked && probe.overlaps(&item.rect()))
    };
    let (w, h) = [(w, h), (w, start.h), (start.w, h)]
        .into_iter()
        .find(|&(w, h)| fits(w, h, &next))?;

    next[ix].w = w;
    next[ix].h = h;
    if rules.prevent_collision && any_collision(&next, next[ix].rect(), ix) {
        return None;
    }

    let mut push = Push::new(rules, next.len());
    push.moved[ix] = true;
    for other in collisions(&next, sorted_indices(&next, rules.compaction), ix) {
        let below = next[ix].bottom();
        push.move_element(&mut next, other, None, Some(below), false);
    }
    settle(&mut next, rules, Some(ix));
    Some(next)
}

/// The cascade of a move: an item that lands on others pushes them, and they
/// push the items they land on in turn.
struct Push {
    rules: LayoutRules,
    /// Items already settled by this move, which later collisions leave alone.
    moved: Vec<bool>,
}

impl Push {
    fn new(rules: LayoutRules, len: usize) -> Self {
        Self {
            rules,
            moved: vec![false; len],
        }
    }

    fn move_element(
        &mut self,
        items: &mut [GridItem],
        ix: usize,
        x: Option<usize>,
        y: Option<usize>,
        user_action: bool,
    ) {
        if items[ix].locked {
            return;
        }
        let (old_x, old_y) = (items[ix].x, items[ix].y);
        let x = x.unwrap_or(old_x);
        let y = y.unwrap_or(old_y);
        if (x, y) == (old_x, old_y) {
            return;
        }
        items[ix].x = x;
        items[ix].y = y;
        self.moved[ix] = true;

        // Moving toward the compaction direction, resolve the nearest
        // collisions last so they win the space the item vacated.
        let mut order = sorted_indices(items, self.rules.compaction);
        let moving_back = match self.rules.compaction {
            Some(Axis::Horizontal) => old_x >= x,
            _ => old_y >= y,
        };
        if moving_back {
            order.reverse();
        }

        for other in collisions(items, order, ix) {
            if self.moved[other] {
                continue;
            }
            if items[other].locked {
                // A locked item cannot give way, so the mover does.
                self.away(items, other, ix, user_action);
            } else {
                self.away(items, ix, other, user_action);
            }
        }
    }

    /// Move `mover` clear of `obstacle`.
    fn away(&mut self, items: &mut [GridItem], obstacle: usize, mover: usize, user_action: bool) {
        let horizontal = self.rules.compaction == Some(Axis::Horizontal);
        if user_action {
            // A dragged item that lands on another first tries to trade places
            // with it: if the space on the near side of the obstacle is free,
            // the pushed item goes there instead of being shoved further away.
            let (o, m) = (&items[obstacle], &items[mover]);
            let probe = Rect {
                x: if horizontal {
                    o.x.saturating_sub(m.w)
                } else {
                    m.x
                },
                y: if horizontal {
                    m.y
                } else {
                    o.y.saturating_sub(m.h)
                },
                w: m.w,
                h: m.h,
            };
            let blocker = (0..items.len()).find(|&other| {
                other != mover && other != obstacle && probe.overlaps(&items[other].rect())
            });
            let obstacle_rect = items[obstacle].rect();
            match blocker {
                None if !probe.overlaps(&obstacle_rect) => {
                    let (x, y) = if horizontal {
                        (Some(probe.x), None)
                    } else {
                        (None, Some(probe.y))
                    };
                    return self.move_element(items, mover, x, y, false);
                }
                _ => {}
            }
        }

        // Otherwise push it past the obstacle along the compaction axis.
        let (x, y) = if horizontal {
            (Some(items[obstacle].right()), None)
        } else {
            (None, Some(items[obstacle].bottom()))
        };
        self.move_element(items, mover, x, y, false);
    }
}

/// Item indices in the order compaction visits them: row-major for vertical
/// layouts and column-major for horizontal ones.
fn sorted_indices(items: &[GridItem], compaction: Option<Axis>) -> Vec<usize> {
    let mut order = (0..items.len()).collect::<Vec<_>>();
    match compaction {
        Some(Axis::Horizontal) => order.sort_by_key(|&ix| (items[ix].x, items[ix].y)),
        _ => order.sort_by_key(|&ix| (items[ix].y, items[ix].x)),
    }
    order
}

fn first_collision(items: &[GridItem], among: &[usize], rect: Rect, skip: usize) -> Option<usize> {
    among
        .iter()
        .copied()
        .find(|&other| other != skip && rect.overlaps(&items[other].rect()))
}

fn any_collision(items: &[GridItem], rect: Rect, skip: usize) -> bool {
    items
        .iter()
        .enumerate()
        .any(|(other, item)| other != skip && rect.overlaps(&item.rect()))
}

fn collisions(items: &[GridItem], order: Vec<usize>, ix: usize) -> Vec<usize> {
    let rect = items[ix].rect();
    order
        .into_iter()
        .filter(|&other| other != ix && rect.overlaps(&items[other].rect()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(compaction: Option<Axis>) -> LayoutRules {
        LayoutRules {
            columns: 12,
            compaction,
            prevent_collision: false,
        }
    }

    fn at<'a>(items: &'a [GridItem], id: &str) -> &'a GridItem {
        items.iter().find(|item| item.id.as_ref() == id).unwrap()
    }

    fn pos(items: &[GridItem], id: &str) -> (usize, usize) {
        let item = at(items, id);
        (item.x, item.y)
    }

    fn assert_no_overlaps(items: &[GridItem]) {
        for (a, first) in items.iter().enumerate() {
            for second in &items[a + 1..] {
                assert!(
                    !first.rect().overlaps(&second.rect()),
                    "{} overlaps {}",
                    first.id,
                    second.id
                );
            }
        }
    }

    #[test]
    fn test_grid_item_builder() {
        let item = GridItem::new("a", 1, 2, 0, 3)
            .with_min_size(2, 0)
            .with_max_size(6, 4)
            .with_locked(true);
        assert_eq!((item.x, item.y, item.w, item.h), (1, 2, 1, 3));
        assert_eq!((item.min_w, item.min_h), (2, 1));
        assert_eq!((item.max_w, item.max_h), (Some(6), Some(4)));
        assert!(item.locked);
        assert_eq!((item.right(), item.bottom()), (2, 5));
    }

    #[test]
    fn vertical_compaction_floats_items_up() {
        let mut items = vec![
            GridItem::new("a", 0, 3, 4, 2),
            GridItem::new("b", 0, 9, 4, 2),
            GridItem::new("c", 4, 7, 4, 1),
        ];
        compact(&mut items, rules(Some(Axis::Vertical)));
        assert_eq!(pos(&items, "a"), (0, 0));
        assert_eq!(pos(&items, "b"), (0, 2));
        assert_eq!(pos(&items, "c"), (4, 0));
    }

    #[test]
    fn horizontal_compaction_floats_items_leading() {
        let mut items = vec![
            GridItem::new("a", 5, 0, 2, 2),
            GridItem::new("b", 9, 0, 2, 2),
        ];
        compact(&mut items, rules(Some(Axis::Horizontal)));
        assert_eq!(pos(&items, "a"), (0, 0));
        assert_eq!(pos(&items, "b"), (2, 0));
    }

    #[test]
    fn compaction_flows_around_locked_items() {
        let mut items = vec![
            GridItem::new("wall", 0, 0, 12, 1).with_locked(true),
            GridItem::new("a", 0, 0, 4, 2),
        ];
        normalize(&mut items, rules(Some(Axis::Vertical)));
        assert_eq!(pos(&items, "wall"), (0, 0));
        assert_eq!(pos(&items, "a"), (0, 1));
    }

    #[test]
    fn normalize_fits_items_to_the_columns_and_removes_overlaps() {
        let mut items = vec![
            GridItem::new("wide", 4, 0, 20, 1),
            GridItem::new("a", 0, 0, 2, 1),
            GridItem::new("b", 0, 0, 2, 1),
        ];
        normalize(&mut items, rules(None));
        assert_eq!(at(&items, "wide").w, 12);
        assert_eq!(at(&items, "wide").x, 0);
        assert_no_overlaps(&items);
    }

    #[test]
    fn moving_onto_an_item_pushes_it_down() {
        let items = vec![
            GridItem::new("a", 0, 0, 4, 2),
            GridItem::new("b", 4, 0, 4, 2),
        ];
        let next = move_item(&items, "a", 4, 1, rules(None)).unwrap();
        assert_eq!(pos(&next, "a"), (4, 1));
        assert_eq!(pos(&next, "b"), (4, 3));
        assert_no_overlaps(&next);
    }

    #[test]
    fn moving_up_onto_an_item_swaps_places() {
        let items = vec![
            GridItem::new("a", 0, 0, 4, 2),
            GridItem::new("b", 0, 2, 4, 2),
        ];
        let next = move_item(&items, "b", 0, 0, rules(Some(Axis::Vertical))).unwrap();
        assert_eq!(pos(&next, "b"), (0, 0));
        assert_eq!(pos(&next, "a"), (0, 2));
        assert_no_overlaps(&next);
    }

    #[test]
    fn moves_and_resizes_never_leave_overlaps() {
        let mut items = (0..7)
            .map(|n| GridItem::new(format!("item-{n}"), (n % 3) * 4, (n / 3) * 2, 4, 2 + n % 2))
            .collect::<Vec<_>>();
        items.push(GridItem::new("wall", 5, 5, 2, 1).with_locked(true));
        for compaction in [None, Some(Axis::Vertical), Some(Axis::Horizontal)] {
            let rules = rules(compaction);
            let mut start = items.clone();
            normalize(&mut start, rules);
            assert_no_overlaps(&start);
            for n in 0..7 {
                let id = format!("item-{n}");
                for (x, y) in (0..12).flat_map(|x| (0..9).map(move |y| (x, y))) {
                    let next = move_item(&start, &id, x, y, rules).unwrap();
                    assert_no_overlaps(&next);
                    assert_eq!(pos(&next, "wall"), pos(&start, "wall"));
                    if let Some(next) = resize_item(&start, &id, x + 1, y + 1, rules) {
                        assert_no_overlaps(&next);
                    }
                }
            }
        }
    }

    #[test]
    fn a_move_is_clamped_to_the_columns() {
        let items = vec![GridItem::new("a", 0, 0, 4, 2)];
        let next = move_item(&items, "a", 11, 0, rules(None)).unwrap();
        assert_eq!(pos(&next, "a"), (8, 0));
    }

    #[test]
    fn locked_items_neither_move_nor_get_pushed() {
        let items = vec![
            GridItem::new("wall", 4, 0, 4, 2).with_locked(true),
            GridItem::new("a", 0, 0, 4, 2),
        ];
        assert!(move_item(&items, "wall", 0, 4, rules(None)).is_none());
        let next = move_item(&items, "a", 4, 0, rules(Some(Axis::Vertical))).unwrap();
        assert_eq!(pos(&next, "wall"), (4, 0));
        assert_no_overlaps(&next);
    }

    #[test]
    fn prevent_collision_refuses_overlapping_moves() {
        let items = vec![
            GridItem::new("a", 0, 0, 4, 2),
            GridItem::new("b", 4, 0, 4, 2),
        ];
        let rules = LayoutRules {
            prevent_collision: true,
            ..rules(None)
        };
        assert!(move_item(&items, "a", 2, 0, rules).is_none());
        assert_eq!(
            pos(&move_item(&items, "a", 0, 3, rules).unwrap(), "a"),
            (0, 3)
        );
    }

    #[test]
    fn resize_respects_limits_and_pushes_neighbours() {
        let items = vec![
            GridItem::new("a", 0, 0, 4, 2)
                .with_min_size(2, 2)
                .with_max_size(8, 4),
            GridItem::new("b", 4, 0, 4, 2),
        ];
        let next = resize_item(&items, "a", 20, 1, rules(Some(Axis::Vertical))).unwrap();
        assert_eq!((at(&next, "a").w, at(&next, "a").h), (8, 2));
        assert_eq!(pos(&next, "b"), (4, 2));
        assert_no_overlaps(&next);
    }

    #[test]
    fn resize_stops_at_locked_items() {
        let items = vec![
            GridItem::new("a", 0, 0, 4, 2),
            GridItem::new("wall", 4, 0, 4, 2).with_locked(true),
        ];
        let next = resize_item(&items, "a", 6, 3, rules(None)).unwrap();
        assert_eq!((at(&next, "a").w, at(&next, "a").h), (4, 3));
        assert_no_overlaps(&next);
    }
}
