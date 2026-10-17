//! `BlockEditor` view (design §7).
//!
//! `render` takes the cached projection and renders it with gpui-kit's own
//! `BlockNode` renderer, so the editor looks exactly like `TextView`. The
//! [`EditorRoot`] element installs a frame-local [`FrameSink`] while its
//! children prepaint: keyed text leaves report their layouts into it and take
//! their selection backgrounds from it (`text::editor_seam`, D3). The root
//! then paints the caret and the composition underline from that geometry.
//! Paint never updates the editor entity.

use std::{cell::Cell, collections::HashMap, ops::Range, rc::Rc, sync::Arc};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, Bounds, CursorStyle, DefiniteLength, DispatchPhase, Edges, Element, ElementId,
    ElementInputHandler, Entity, GlobalElementId, Hitbox, HitboxBehavior, Hsla, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Refineable, RenderOnce, Role, StatefulInteractiveElement,
    Style, StyleRefinement, Styled, TextStyle, Window, canvas, div, fill, point, px, size,
};

use super::{
    cull::Measured, frame::FrameSink, id::BlockId, projection::Snapshot, state::*,
    style::BlockEditorStyle,
};
use crate::StyledExt as _;
use crate::text::{
    TextViewDefaults, TextViewStyle,
    document::{NodeRenderOptions, PrevBlock},
    node::{BlockNode, NodeContext},
};

/// Items per cull unit of a long top-level list.
const LIST_CHUNK: usize = 32;

/// A block-based WYSIWYG Markdown editor (spike). Hold an
/// `Entity<BlockEditorState>` and pass it here in `render`. Instance styling
/// applies to the native input root; it installs no padding or height defaults.
#[derive(IntoElement)]
pub struct BlockEditor {
    state: Entity<BlockEditorState>,
    style: StyleRefinement,
}

impl BlockEditor {
    pub fn new(state: &Entity<BlockEditorState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }
}

// Presentation is applied inside EditorRoot so padding is part of its
// native hitbox and input bounds, not an unfocusable outer wrapper.
impl Styled for BlockEditor {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The accessible role (and heading level) of a top-level block (Phase 5,
/// per-block accessibility). `None` for blocks that are not exposed on
/// their own (definitions, breaks).
pub(crate) fn block_role(node: &BlockNode) -> Option<(Role, Option<usize>)> {
    Some(match node {
        BlockNode::Paragraph(_) => (Role::Paragraph, None),
        BlockNode::Heading { level, .. } => (Role::Heading, Some(*level as usize)),
        BlockNode::Blockquote { .. } => (Role::Blockquote, None),
        BlockNode::List { .. } => (Role::List, None),
        BlockNode::CodeBlock(_) => (Role::Code, None),
        BlockNode::Table(_) => (Role::Table, None),
        BlockNode::HorizontalRule { .. } => (Role::Splitter, None),
        BlockNode::Custom { .. } => (Role::Group, None),
        BlockNode::ListItem { .. } => (Role::ListItem, None),
        BlockNode::Root { .. }
        | BlockNode::Break { .. }
        | BlockNode::Definition { .. }
        | BlockNode::Unknown => return None,
    })
}

/// `flow_position` over the flattened top-level nodes.
fn prev_block(nodes: &[&BlockNode], ix: usize) -> (bool, PrevBlock) {
    let prev = nodes[..ix].iter().rev().find(|block| {
        !matches!(
            block,
            BlockNode::Definition { .. } | BlockNode::Break { .. } | BlockNode::Unknown
        )
    });
    match prev {
        None => (true, PrevBlock::Other),
        Some(BlockNode::Heading { level, .. }) => (false, PrevBlock::Heading(*level)),
        Some(BlockNode::HorizontalRule { .. }) => (false, PrevBlock::Rule),
        Some(_) => (false, PrevBlock::Other),
    }
}

impl RenderOnce for BlockEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = crate::Theme::global(cx);
        let defaults = cx.try_global::<TextViewDefaults>();
        let text_style = defaults
            .and_then(|d| d.style().cloned())
            .unwrap_or_else(|| TextViewStyle::from_theme(&theme));
        let highlighter = defaults.and_then(|d| d.code_block_highlighter());
        let mut style = self.state.read(cx).style.resolved(&theme.tokens);
        if style.selection_is_default() {
            style = style.with_selection(text_style.selection());
        }
        // Read the prior frame before request_layout can reset shared text
        // layouts. Manual scrolling must not become a reflow reveal request.
        let caret_was_visible = {
            let state = self.state.read(cx);
            state
                .scroll_handle
                .as_ref()
                .zip(state.model.caret())
                .is_some_and(|(handle, pos)| {
                    state
                        .frame
                        .borrow()
                        .caret_bounds(pos, style.caret_width())
                        .is_some_and(|caret| handle.bounds().contains(&caret.center()))
                })
        };
        let (focus, snapshot, backgrounds, extensions, keep, list_items) =
            self.state.update(cx, |state, _| {
                let snapshot = state.snapshot();
                let mut backgrounds: HashMap<u64, Vec<(Range<usize>, Hsla)>> = HashMap::new();
                for (block, range) in state.selection_ranges() {
                    // Atoms paint whole (O(1) lookup, not a scan of the leaves).
                    let range = match state.model.doc.get(block).map(|e| &e.content) {
                        Some(super::document::BlockContent::Atom(_)) => {
                            0..state.model.doc.leaf_len(block).unwrap_or(range.end)
                        }
                        _ => range,
                    };
                    backgrounds
                        .entry(block.as_u64())
                        .or_default()
                        .push((range, style.selection()));
                }
                (
                    state.focus_handle.clone(),
                    snapshot,
                    backgrounds,
                    state.model.doc.extensions.clone(),
                    state.pinned_blocks(),
                    // Only needed to rebuild the cached cull units.
                    (!state.cull.borrow().index.is_current(state.model.revision())).then(|| {
                        let doc = &state.model.doc;
                        doc.children(doc.root)
                            .iter()
                            .filter(|id| {
                                matches!(
                                    doc.kind(**id),
                                    Some(super::kind::BlockKindData::List { .. })
                                ) && doc.children(**id).len() > LIST_CHUNK
                            })
                            .map(|id| (*id, Rc::<[BlockId]>::from(doc.children(*id))))
                            .collect::<HashMap<_, _>>()
                    }),
                )
            });

        // Match the native root's effective inherited/instance typography
        // before deriving body-relative node metrics and culling geometry.
        let mut typography = window.text_style();
        typography.refine(&self.style.text);
        let node_cx = NodeContext {
            style: Arc::new(text_style),
            code_block_highlighter: highlighter,
            markdown_extensions: extensions,
            image_policy: self.state.read(cx).image_policy(),
            body_font_size: typography.font_size.to_pixels(window.rem_size()),
            ..Default::default()
        };
        let (cull, revision) = {
            let state = self.state.read(cx);
            (state.cull.clone(), state.model.revision())
        };
        let mut children: Vec<AnyElement> = Vec::new();
        {
            let mut cull_state = cull.borrow_mut();
            cull_state.measured.clear();
            let window_span = cull_state.visible.map(|(top, bottom)| {
                // Overscan two viewports each way, so a fast scroll stays
                // within laid-out blocks until the next frame catches up.
                let margin = ((bottom - top) * 2.).max(px(600.));
                (top - margin, bottom + margin)
            });
            let cull_state = &mut *cull_state;
            if let Some(list_items) = list_items {
                // Cull units: each top-level node, except that a long
                // top-level list is split into chunks of `LIST_CHUNK` items,
                // each rendered as a list of its own (continuing the
                // numbering, no gap between chunks). Rebuilt per revision.
                let nodes: Vec<&BlockNode> = snapshot
                    .blocks
                    .iter()
                    .flat_map(|b| b.nodes.iter())
                    .collect();
                let mut units = Vec::with_capacity(nodes.len());
                let mut node_ix = 0;
                for (block_ix, block) in snapshot.blocks.iter().enumerate() {
                    for (node_in_block, node) in block.nodes.iter().enumerate() {
                        let (is_first, prev) = prev_block(&nodes, node_ix);
                        let unit = |id, items| super::cull::UnitDesc {
                            id,
                            block: block_ix,
                            node: node_in_block,
                            node_ix,
                            items,
                            is_first,
                            prev,
                        };
                        match (node, list_items.get(&block.id)) {
                            (BlockNode::List { children, .. }, Some(ids))
                                if ids.len() == children.len() =>
                            {
                                for start in (0..ids.len()).step_by(LIST_CHUNK) {
                                    let range = start..(start + LIST_CHUNK).min(ids.len());
                                    units.push(unit(ids[start], Some((range, ids.clone()))));
                                }
                            }
                            _ => units.push(unit(block.id, None)),
                        }
                        node_ix += 1;
                    }
                }
                cull_state.index.rebuild(
                    revision,
                    units.iter().map(|u| {
                        let covered: &[BlockId] = match &u.items {
                            Some((range, ids)) => &ids[range.clone()],
                            None => &[],
                        };
                        (u.id, covered)
                    }),
                    &cull_state.heights,
                );
                cull_state.units = units;
                cull_state.node_count = nodes.len();
            }
            let units = &cull_state.units;
            let count = cull_state.node_count;
            let index = &cull_state.index;
            let n = units.len();
            // Units to lay out: the visible range (binary search over the
            // prefix sums) plus pinned units (selection ends, IME).
            let (first, last) = match window_span {
                None => (0, n),
                Some((top, bottom)) => {
                    let first = index.find(top.max(px(0.))).min(n);
                    (first, (index.find(bottom) + 1).min(n).max(first))
                }
            };
            let mut wanted: Vec<usize> = keep
                .iter()
                .filter_map(|id| index.unit_of.get(id).copied())
                .filter(|ix| *ix < first || *ix >= last)
                .collect();
            wanted.extend(first..last);
            wanted.sort_unstable();
            wanted.dedup();
            let mut cursor = px(0.);
            let mut rendered = 0;
            for unit_ix in wanted {
                let unit = &units[unit_ix];
                let node_ix = unit.node_ix;
                let top = index.prefix(unit_ix);
                if top > cursor {
                    children.push(div().h(top - cursor).into_any_element());
                }
                cursor = top + index.height(unit_ix);
                rendered += 1;
                let (is_first, prev) = (unit.is_first, unit.prev);
                let is_last_node = node_ix + 1 == count;
                let node = &snapshot.blocks[unit.block].nodes[unit.node];
                let element = match (&unit.items, node) {
                    (
                        Some((range, ids)),
                        BlockNode::List {
                            children: items,
                            ordered,
                            start,
                            ..
                        },
                    ) => {
                        let last_chunk = range.end == ids.len();
                        let chunk = BlockNode::List {
                            children: items[range.clone()].to_vec(),
                            ordered: *ordered,
                            start: if *ordered {
                                Some(start.unwrap_or(1) + range.start as u32)
                            } else {
                                *start
                            },
                            span: None,
                        };
                        chunk.render_block(
                            NodeRenderOptions {
                                ix: node_ix,
                                is_first: is_first && range.start == 0,
                                prev,
                                // Inner chunks drop the list's bottom gap.
                                is_last: !last_chunk || is_last_node,
                                ..Default::default()
                            },
                            &node_cx,
                            window,
                            cx,
                        )
                    }
                    _ => node.render_block(
                        NodeRenderOptions {
                            ix: node_ix,
                            is_first,
                            prev,
                            is_last: is_last_node,
                            ..Default::default()
                        },
                        &node_cx,
                        window,
                        cx,
                    ),
                };
                // Per-block accessibility node around the rendered block.
                let element = match block_role(node) {
                    Some((role, level)) => div()
                        .id(("block", unit.id.as_u64() as usize))
                        .role(role)
                        .when_some(level, |this, level| this.aria_level(level))
                        .child(element)
                        .into_any_element(),
                    None => element,
                };
                children.push(
                    Measured {
                        id: unit.id,
                        index: unit_ix,
                        state: cull.clone(),
                        child: Some(element),
                    }
                    .into_any_element(),
                );
            }
            let total = index.prefix(n);
            if total > cursor {
                children.push(div().h(total - cursor).into_any_element());
            }
            cull_state.rendered = rendered;
        }

        let content_bounds = Rc::new(Cell::new(Bounds::default()));
        let measured_content = content_bounds.clone();
        let mut root_style = Style::default();
        root_style.refine(&self.style);
        let s = &self.state;
        div()
            .id("block-editor")
            // D8: one multi-line text input over the document; the value is
            // built only while accessibility is active (cached per revision).
            .role(Role::MultilineTextInput)
            .when(window.is_a11y_active() || cfg!(test), |this| {
                this.aria_value(self.state.read(cx).accessible_value())
            })
            .key_context(self.state.read(cx).key_context())
            .track_focus(&focus)
            .cursor(if self.state.read(cx).pointer_hand {
                CursorStyle::PointingHand
            } else {
                CursorStyle::IBeam
            })
            .w_full()
            .on_action(window.listener_for(s, BlockEditorState::on_backspace))
            .on_action(window.listener_for(s, BlockEditorState::on_delete))
            .on_action(window.listener_for(s, BlockEditorState::on_enter))
            .on_action(window.listener_for(s, BlockEditorState::on_structural_enter))
            .on_action(window.listener_for(s, BlockEditorState::on_soft_break))
            .on_action(window.listener_for(s, BlockEditorState::on_exit_block))
            .on_action(window.listener_for(s, BlockEditorState::on_move_left))
            .on_action(window.listener_for(s, BlockEditorState::on_move_right))
            .on_action(window.listener_for(s, BlockEditorState::on_move_up))
            .on_action(window.listener_for(s, BlockEditorState::on_move_down))
            .on_action(window.listener_for(s, BlockEditorState::on_move_home))
            .on_action(window.listener_for(s, BlockEditorState::on_move_end))
            .on_action(window.listener_for(s, BlockEditorState::on_select_left))
            .on_action(window.listener_for(s, BlockEditorState::on_move_word_left))
            .on_action(window.listener_for(s, BlockEditorState::on_move_word_right))
            .on_action(window.listener_for(s, BlockEditorState::on_select_word_left))
            .on_action(window.listener_for(s, BlockEditorState::on_select_word_right))
            .on_action(window.listener_for(s, BlockEditorState::on_select_right))
            .on_action(window.listener_for(s, BlockEditorState::on_select_up))
            .on_action(window.listener_for(s, BlockEditorState::on_select_down))
            .on_action(window.listener_for(s, BlockEditorState::on_select_home))
            .on_action(window.listener_for(s, BlockEditorState::on_select_end))
            .on_action(window.listener_for(s, BlockEditorState::on_select_all))
            .on_action(window.listener_for(s, BlockEditorState::on_undo))
            .on_action(window.listener_for(s, BlockEditorState::on_redo))
            .on_action(window.listener_for(s, BlockEditorState::on_toggle_bold))
            .on_action(window.listener_for(s, BlockEditorState::on_toggle_italic))
            .on_action(window.listener_for(s, BlockEditorState::on_edit_link))
            .on_action(window.listener_for(s, BlockEditorState::on_toggle_code))
            .on_action(window.listener_for(s, BlockEditorState::on_indent))
            .on_action(window.listener_for(s, BlockEditorState::on_outdent))
            .on_action(window.listener_for(s, BlockEditorState::on_copy))
            .on_action(window.listener_for(s, BlockEditorState::on_cut))
            .on_action(window.listener_for(s, BlockEditorState::on_paste))
            .on_action(window.listener_for(s, BlockEditorState::on_native_paste))
            .on_action(window.listener_for(s, BlockEditorState::on_paste_plain))
            .child(EditorRoot {
                state: self.state.clone(),
                child: Some(
                    div()
                        .w_full()
                        .refine_style(&self.style)
                        .child(
                            div().relative().w_full().children(children).child(
                                canvas(
                                    move |bounds, _, _| measured_content.set(bounds),
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full(),
                            ),
                        )
                        .into_any_element(),
                ),
                content_bounds,
                padding: root_style.padding,
                typography,
                caret_was_visible,
                sink: Some(Rc::new(FrameSink::new(backgrounds))),
                snapshot,
                style,
            })
    }
}

struct EditorRoot {
    state: Entity<BlockEditorState>,
    child: Option<AnyElement>,
    sink: Option<Rc<FrameSink>>,
    snapshot: Rc<Snapshot>,
    style: BlockEditorStyle,
    content_bounds: Rc<Cell<Bounds<Pixels>>>,
    padding: Edges<DefiniteLength>,
    typography: TextStyle,
    caret_was_visible: bool,
}

impl IntoElement for EditorRoot {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

pub(crate) struct Prepaint {
    hitbox: Hitbox,
}

impl EditorRoot {
    /// Scroll the caret into view in the owner's scroll container, once its
    /// block has been laid out (a culled block is pinned, so it is laid out
    /// in the frame after the caret moved there).
    fn reveal_caret(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let state = self.state.read(cx);
        if !state.reveal.get() {
            return;
        }
        let (Some(handle), Some(pos)) = (state.scroll_handle.clone(), state.model.caret()) else {
            state.reveal.set(false);
            return;
        };
        let Some(caret) = state
            .frame
            .borrow()
            .caret_bounds(pos, self.style.caret_width())
        else {
            return;
        };
        state.reveal.set(false);
        // The scroll owner stays at the region edge; text and the revealed
        // caret stay inside the root's presentation inset at either scroll end.
        let padding = self
            .padding
            .to_pixels(bounds.size.map(Into::into), window.rem_size());
        let mut view = handle.bounds();
        view.origin.y += padding.top;
        view.size.height = (view.size.height - padding.top - padding.bottom).max(Pixels::ZERO);
        let mut offset = handle.offset();
        if caret.top() < view.top() {
            offset.y += view.top() - caret.top();
        } else if caret.bottom() > view.bottom() {
            offset.y -= caret.bottom() - view.bottom();
        } else {
            return;
        }
        handle.set_offset(offset);
        window.refresh();
    }
}

impl Element for EditorRoot {
    type RequestLayoutState = AnyElement;
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let mut child = self
            .child
            .take()
            .unwrap_or_else(|| div().into_any_element());
        let layout_id = child.request_layout(window, cx);
        (layout_id, child)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> Prepaint {
        if let Some(sink) = self.sink.take() {
            crate::text::editor_seam::with_sink(sink.clone(), || child.prepaint(window, cx));
            // Swap in the complete frame; handlers read it until the next one.
            if let Ok(sink) = Rc::try_unwrap(sink) {
                let frame = sink.into_frame(self.snapshot.leaves.clone(), &self.snapshot.tables);
                *self.state.read(cx).frame.borrow_mut() = frame;
            }
            let mask = window.content_mask().bounds;
            let content = self.content_bounds.get();
            let cull = self.state.read(cx).cull.clone();
            let geometry_changed = cull.borrow_mut().set_geometry(
                content.size.width,
                window.rem_size(),
                &self.typography,
            );
            // Retain a visible caret or an existing reveal request, but do not
            // interrupt manual scrolling. Wait for fresh spacer positions.
            if geometry_changed {
                if self.caret_was_visible {
                    self.state.read(cx).reveal.set(true);
                }
            } else {
                self.reveal_caret(bounds, window, cx);
            }
            if geometry_changed || cull.borrow_mut().finish_frame(content, mask) {
                // Geometry changed or scrolling left the laid-out span.
                let state = self.state.clone();
                window.on_next_frame(move |_, cx| state.update(cx, |_, cx| cx.notify()));
            }
        } else {
            child.prepaint(window, cx);
        }
        Prepaint {
            hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut AnyElement,
        prepaint: &mut Prepaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.state.read(cx);
        let focus = state.focus_handle.clone();
        let collapsed = state.selection_ranges().is_empty();
        let marked = state.marked_range();
        let caret = state.model.caret();
        let show_caret = state.show_caret(window, cx) && collapsed;
        let frame = state.frame.clone();
        // Node selection (a table, code block, …): the blocks' laid-out area.
        let selected_area = state.model.selected_blocks().and_then(|(parent, range)| {
            let doc = &state.model.doc;
            let ids = doc.children(parent).get(range)?.to_vec();
            let frame = frame.borrow();
            frame
                .leaves
                .iter()
                .filter(|leaf| {
                    let path = doc.path(leaf.info.block);
                    ids.iter()
                        .any(|id| *id == leaf.info.block || path.contains(id))
                })
                .map(|leaf| leaf.bounds())
                .reduce(|a, b| a.union(&b))
        });

        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );
        child.paint(window, cx);

        let frame = frame.borrow();
        let style = &self.style;
        // Block handle: a grip in the left edge of the hovered table or code
        // block; clicking the edge node-selects it (`block_handle_at`).
        let mouse = window.mouse_position();
        let hovered = frame
            .tables
            .iter()
            .map(|(_, b)| *b)
            .chain(
                frame
                    .leaves
                    .iter()
                    .filter(|l| state_plain(&self.state, l.info.block, cx))
                    .map(|l| l.bounds()),
            )
            .find(|b| {
                mouse.y >= b.top()
                    && mouse.y < b.bottom()
                    && mouse.x >= b.left() - px(24.)
                    && mouse.x < b.right()
            });
        if let Some(b) = hovered {
            let grip = Bounds::new(
                point(b.left() - px(14.), b.top() + px(2.)),
                size(px(4.), px(16.).min(b.size.height)),
            );
            window.paint_quad(fill(grip, style.caret().opacity(0.35)).corner_radii(px(2.)));
        }
        if let Some(area) = selected_area {
            let area = area.dilate(px(4.));
            window.paint_quad(gpui::quad(
                area,
                px(4.),
                style.selection().opacity(0.35),
                px(2.),
                style.caret(),
                gpui::BorderStyle::Solid,
            ));
        }
        if let Some((block, range)) = marked
            && let Some(leaf) = frame.leaf(block)
        {
            let a = leaf.caret_point(range.start);
            let b = leaf.caret_point(range.end);
            let thickness = style.caret_width();
            let underline = Bounds::new(
                point(a.x, a.y + leaf.line_height() - thickness),
                size((b.x - a.x).max(thickness), thickness),
            );
            window.paint_quad(fill(underline, style.composition_underline()));
        }
        if show_caret
            && let Some(pos) = caret
            && let Some(b) = frame.caret_bounds(pos, style.caret_width())
        {
            // Whole device pixels, like the input caret: a 1px line at a
            // fractional x would blur over two pixels.
            let b = crate::input::snap_caret_to_device_pixels(b, window.scale_factor());
            window.paint_quad(fill(b, style.caret()));
        }
        drop(frame);

        // Mouse: handlers update the entity at event time, never in paint.
        let hitbox = prepaint.hitbox.clone();
        let entity = self.state.clone();
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !hitbox.is_hovered(window)
            {
                return;
            }
            // A task checkbox toggles without moving the caret or starting
            // a drag; nothing else handles this mouse-down.
            if !event.modifiers.shift
                && entity.update(cx, |state, cx| state.click_checkbox(event.position, cx))
            {
                cx.stop_propagation();
                return;
            }
            entity.update(cx, |state, cx| {
                let focus = state.focus_handle.clone();
                window.focus(&focus, cx);
                if event.modifiers.secondary() && state.follow_link_at(event.position, cx) {
                    return;
                }
                state.dragging = true;
                state.mouse_at(event.position, event.modifiers.shift, cx);
                if !event.modifiers.shift && event.click_count >= 2 {
                    state.select_unit(event.click_count >= 3, cx);
                }
            });
        });
        let entity = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            if event.pressed_button.is_none() {
                entity.update(cx, |state, cx| {
                    state.hover_at(event.position, event.modifiers.secondary(), cx)
                });
                return;
            }
            if event.pressed_button != Some(MouseButton::Left) {
                return;
            }
            entity.update(cx, |state, cx| {
                if state.dragging {
                    state.mouse_at(event.position, true, cx);
                }
            });
        });
        let entity = self.state.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble {
                entity.update(cx, |state, _| state.dragging = false);
            }
        });
    }
}

fn state_plain(state: &Entity<BlockEditorState>, block: BlockId, cx: &App) -> bool {
    state.read(cx).model.doc.plain(block).is_some()
}
