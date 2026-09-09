use gpui::{Axis, Pixels};

use super::layout::{NodeKind, PaneNode, PaneRef, PaneTree, PanelId, RootKind, TilePanel};
use super::state::{PanelInfo, PanelState, TileMeta};

/// The names written to persisted layouts. These are contract, not type names:
/// they must keep their values even if the Rust types are renamed.
pub(crate) const STACK_PANEL_NAME: &str = "StackPanel";
pub(crate) const TAB_PANEL_NAME: &str = "TabPanel";
pub(crate) const TILES_PANEL_NAME: &str = "Tiles";

/// How the layout tree reads properties of panels it only knows by id.
///
/// Keeping this behind a trait is what lets the whole layout algebra be tested
/// without an `App`.
pub trait PanelSource {
    fn panel_name(&self, id: PanelId) -> &'static str;
    fn is_visible(&self, id: PanelId) -> bool;
    fn dump(&self, id: PanelId) -> PanelState;
}

impl PaneTree {
    pub fn to_state(&self, source: &dyn PanelSource) -> PanelState {
        node_to_state(self.persisted_root(), source)
    }

    /// The node the persisted layout starts at.
    ///
    /// A `RootKind::Split` tree keeps a `Split` root even around a lone tiles
    /// canvas, so an edit always has a container to land in. That wrapper is a
    /// tree invariant, not part of the persisted schema: the dock before the
    /// tree rewrite wrote a tiles center as a bare `Tiles`, and its
    /// `StackPanel::insert_panel` asserts every split child is a `TabPanel` or
    /// `StackPanel`, so writing the wrapper crashes that reader on load.
    /// `from_state` puts the wrapper back, so the round trip still holds.
    fn persisted_root(&self) -> &PaneNode {
        let root = self.root();
        match root.kind() {
            PaneRef::Split {
                children: [only], ..
            } if self.root_kind() == RootKind::Split
                && matches!(only.kind(), PaneRef::Tiles { .. }) =>
            {
                only
            }
            _ => root,
        }
    }
}

fn node_to_state(node: &PaneNode, source: &dyn PanelSource) -> PanelState {
    match node.kind() {
        PaneRef::Split {
            axis,
            children,
            sizes,
        } => PanelState {
            panel_name: STACK_PANEL_NAME.to_string(),
            children: children
                .iter()
                .map(|child| node_to_state(child, source))
                .collect(),
            // `None` is a representation this rewrite introduces, not
            // something the old writer ever produced: the schema's `sizes`
            // field is `Vec<Pixels>`, with no slot for "unconstrained". `0.0`
            // is the sentinel this writer chooses for that case, and the
            // corresponding reader maps a `0.0` it loads back to `None`.
            //
            // That makes a `None` slot safe to persist only transiently: a
            // caller building a tree meant to be written out must resolve
            // every slot to concrete pixels first. An older build reading a
            // persisted `0.0` back has no notion of the sentinel and
            // constructs a real, zero-pixel-wide panel from it.
            info: PanelInfo::stack(
                sizes.iter().map(|size| size.unwrap_or_default()).collect(),
                axis,
            ),
        },
        PaneRef::Tabs { panels, active_ix } => PanelState {
            panel_name: TAB_PANEL_NAME.to_string(),
            children: panels.iter().map(|panel| source.dump(*panel)).collect(),
            // Unconditional, unlike the old writer which assigned this inside
            // its loop and left an empty group looking like a bare panel.
            info: PanelInfo::tabs(active_ix),
        },
        PaneRef::Tiles { panels } => PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: panels
                .iter()
                .map(|tile| source.dump(tile.panel()))
                .collect(),
            info: PanelInfo::tiles(
                panels
                    .iter()
                    .map(|tile| TileMeta {
                        bounds: tile.bounds(),
                        z_index: tile.z_index(),
                    })
                    .collect(),
            ),
        },
    }
}

/// Turns a persisted leaf into a live panel id.
///
/// The production implementation (at the `gpui-component` layer, above this
/// crate) consults `PanelRegistry` and falls back to an invalid-panel
/// placeholder that retains the original `PanelState`, so a panel type this
/// build does not know about survives a load/save round trip instead of
/// being erased.
pub trait PanelBuilder {
    fn build(&mut self, state: &PanelState, info: &PanelInfo) -> PanelId;
}

impl PaneTree {
    /// Read a persisted layout.
    ///
    /// Compatibility rules, all previously implicit in `PanelState::to_item`:
    ///
    /// - a `Tabs` whose children are themselves `Tabs` is flattened;
    /// - a bare `Panel` leaf appearing where a container belongs is wrapped in
    ///   a `Tabs`;
    /// - a node named `TabPanel` carrying `PanelInfo::Panel` is read as an
    ///   empty tab group, recovering data written by the old dump defect (an
    ///   empty `TabPanel` never entered the loop that set its `info` to
    ///   `Tabs`, so it kept `PanelState`'s default `Panel(Value::Null)`). The
    ///   old reader had no such rule: it looked "TabPanel" up in the panel
    ///   registry, found nothing, and rendered an `InvalidPanel` placeholder
    ///   where an empty tab group belonged. This rule is a genuine fix, not a
    ///   preserved behavior;
    /// - a `Tiles` child without a matching meta keeps the default placement.
    ///   The old writer's counterpart (`DockItem::tiles`) hard-asserted
    ///   `items.len() == metas.len()` and panicked the whole load on a short
    ///   `metas` list, so this rule is a new safety net, not a preserved
    ///   graceful-degradation path.
    pub fn from_state(
        state: &PanelState,
        root_kind: RootKind,
        builder: &mut dyn PanelBuilder,
    ) -> Self {
        let mut tree = PaneTree::new(root_kind);
        let root = build_node(&mut tree, state, builder);

        let root = match (root_kind, &root) {
            (RootKind::Split, node) if !matches!(node.kind_ref(), NodeKind::Split { .. }) => {
                let id = tree.allocate_node_id();
                PaneNode::new(
                    id,
                    NodeKind::Split {
                        axis: Axis::Horizontal,
                        children: vec![node.clone()],
                        sizes: vec![None],
                    },
                )
            }
            _ => root,
        };

        tree.replace_root(root);
        tree.normalize();
        tree
    }
}

fn build_node(tree: &mut PaneTree, state: &PanelState, builder: &mut dyn PanelBuilder) -> PaneNode {
    let id = tree.allocate_node_id();

    match &state.info {
        PanelInfo::Stack { sizes, axis } => {
            let axis = if *axis == 0 {
                Axis::Horizontal
            } else {
                Axis::Vertical
            };
            let children: Vec<PaneNode> = state
                .children
                .iter()
                .map(|child| build_node(tree, child, builder))
                .collect();
            let sizes = (0..children.len())
                .map(|ix| sizes.get(ix).copied().filter(|size| *size > Pixels::ZERO))
                .collect();
            PaneNode::new(
                id,
                NodeKind::Split {
                    axis,
                    children,
                    sizes,
                },
            )
        }
        PanelInfo::Tabs { active_index } => {
            let panels = collect_tab_panels(&state.children, builder);
            PaneNode::new(
                id,
                NodeKind::Tabs {
                    panels,
                    active_ix: *active_index,
                },
            )
        }
        PanelInfo::Tiles { metas } => {
            let mut panels = Vec::new();
            for (ix, child) in state.children.iter().enumerate() {
                // Keyed by the *child* index, not by the output index: a
                // child that expands to several tiles must not shift the
                // metas of the children after it.
                let meta = metas.get(ix).copied().unwrap_or_default();
                for panel in tile_panels(child, builder) {
                    panels.push(TilePanel::new(panel, meta.bounds).with_z_index(meta.z_index));
                }
            }
            PaneNode::new(id, NodeKind::Tiles { panels })
        }
        PanelInfo::Panel(_) => {
            // A container name carrying a leaf info means the writer that
            // produced this file had the empty-group defect.
            let panels = if state.panel_name == TAB_PANEL_NAME {
                Vec::new()
            } else {
                vec![builder.build(state, &state.info)]
            };
            PaneNode::new(
                id,
                NodeKind::Tabs {
                    panels,
                    active_ix: 0,
                },
            )
        }
    }
}

/// The panels one persisted `Tiles` child stands for.
///
/// Every tile the old dock ever wrote is `TabPanel`-shaped: `DockItem::tiles`
/// wraps each child in a `TabPanel`, `DockItem::add_panel`'s tiles arm wraps
/// every UI-added panel in a fresh one, and `PanelState::to_item` converts a
/// plain-panel child into a `TabPanel` on the next save. In the tree model a
/// tile *is* a panel, so the group has to be unwrapped — building the
/// `"TabPanel"` leaf directly would miss the registry, fall to a placeholder,
/// and never build the user's real panels at all, restoring a saved tiles
/// canvas as blank tiles.
///
/// A group holding several panels expands to one tile per panel, sharing the
/// group's meta. That is not merely defensive: the UI cannot produce it
/// (tiles groups are locked, so they can never gain a tab), but
/// `DockItem::tiles(vec![DockItem::tabs(vec![a, b])], ..)` can, and a tiles
/// canvas has no way to show a second tab.
fn tile_panels(child: &PanelState, builder: &mut dyn PanelBuilder) -> Vec<PanelId> {
    match &child.info {
        PanelInfo::Tabs { .. } => {
            let panels = collect_tab_panels(&child.children, builder);
            if panels.len() > 1 {
                tracing::warn!(
                    panels = panels.len(),
                    "a tiles child held a tab group with more than one panel; \
                     expanding it to one tile per panel, all sharing the group's placement"
                );
            }
            panels
        }
        // The legacy empty-group form: a `TabPanel` name carrying leaf info.
        // It stands for no panel, so it contributes no tile.
        PanelInfo::Panel(_) if child.panel_name == TAB_PANEL_NAME => Vec::new(),
        _ => vec![builder.build(child, &child.info)],
    }
}

/// Flatten one level of tab nesting, which the old writer could produce.
fn collect_tab_panels(children: &[PanelState], builder: &mut dyn PanelBuilder) -> Vec<PanelId> {
    children
        .iter()
        .flat_map(|child| match &child.info {
            PanelInfo::Tabs { .. } => collect_tab_panels(&child.children, builder),
            PanelInfo::Panel(_) if child.panel_name == TAB_PANEL_NAME => Vec::new(),
            _ => vec![builder.build(child, &child.info)],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::super::layout::{RootKind, TilePanel};
    use super::super::state::DockAreaState;
    use super::*;

    /// A `PanelSource` backed by a fixed map, so conversion is testable
    /// without an `App`.
    struct FakePanels(Vec<(PanelId, &'static str)>);

    impl PanelSource for FakePanels {
        fn panel_name(&self, id: PanelId) -> &'static str {
            self.0
                .iter()
                .find(|(p, _)| *p == id)
                .map(|(_, n)| *n)
                .unwrap_or("Unknown")
        }
        fn is_visible(&self, _: PanelId) -> bool {
            true
        }
        fn dump(&self, id: PanelId) -> PanelState {
            PanelState {
                panel_name: self.panel_name(id).to_string(),
                children: Vec::new(),
                info: PanelInfo::panel(serde_json::Value::Null),
            }
        }
    }

    #[test]
    fn a_split_serializes_as_a_stack_panel() {
        let mut tree = PaneTree::new(RootKind::Split);
        tree.push_tabs_for_test(tree.root().id(), vec![PanelId::from_u64(1)]);
        tree.push_tabs_for_test(tree.root().id(), vec![PanelId::from_u64(2)]);
        tree.normalize();

        let source = FakePanels(vec![
            (PanelId::from_u64(1), "Alpha"),
            (PanelId::from_u64(2), "Beta"),
        ]);
        let state = tree.to_state(&source);

        assert_eq!(state.panel_name, "StackPanel");
        let PanelInfo::Stack { sizes, .. } = &state.info else {
            panic!("expected Stack info");
        };
        // Both slots were pushed with no explicit size (`push_tabs_for_test`
        // always pushes `None`), so both serialize as the zero sentinel.
        assert_eq!(sizes, &vec![px(0.), px(0.)]);
        assert_eq!(state.children[0].panel_name, "TabPanel");
        assert_eq!(state.children[0].children[0].panel_name, "Alpha");
    }

    #[test]
    fn an_unresolved_slot_size_serializes_as_the_zero_sentinel() {
        let mut tree = PaneTree::new(RootKind::Split);
        let root = tree.root().id();
        tree.push_sized_tabs_for_test(root, vec![PanelId::from_u64(1)], Some(px(120.)));
        tree.push_sized_tabs_for_test(root, vec![PanelId::from_u64(2)], None);
        tree.normalize();

        let source = FakePanels(vec![
            (PanelId::from_u64(1), "Alpha"),
            (PanelId::from_u64(2), "Beta"),
        ]);
        let state = tree.to_state(&source);

        let PanelInfo::Stack { sizes, .. } = state.info else {
            panic!("expected Stack info");
        };
        // The `Some` slot keeps its concrete value; only the genuinely
        // unresolved (`None`) slot is written as the `0.0` sentinel.
        assert_eq!(sizes, vec![px(120.), px(0.)]);
    }

    #[test]
    fn an_empty_tab_group_serializes_as_tabs_not_as_a_panel() {
        let mut tree = PaneTree::new(RootKind::Any);
        tree.set_root_tabs_for_test(vec![], 0);

        let state = tree.to_state(&FakePanels(vec![]));

        assert_eq!(state.panel_name, "TabPanel");
        assert!(
            matches!(state.info, PanelInfo::Tabs { active_index: 0 }),
            "the old writer emitted PanelInfo::Panel here, which failed to restore"
        );
    }

    #[test]
    fn an_empty_center_still_serializes_as_a_stack_panel() {
        let tree = PaneTree::new(RootKind::Split);
        let state = tree.to_state(&FakePanels(vec![]));
        assert_eq!(state.panel_name, "StackPanel");
        assert!(matches!(state.info, PanelInfo::Stack { .. }));
    }

    #[test]
    fn tiles_serialize_with_their_metas_in_order() {
        let mut tree = PaneTree::new(RootKind::Any);
        let bounds = Bounds {
            origin: point(px(5.), px(6.)),
            size: size(px(7.), px(8.)),
        };
        tree.set_root_tiles_for_test(vec![
            TilePanel::new(PanelId::from_u64(1), bounds).with_z_index(2),
        ]);

        let state = tree.to_state(&FakePanels(vec![(PanelId::from_u64(1), "Alpha")]));

        assert_eq!(state.panel_name, "Tiles");
        let PanelInfo::Tiles { metas } = state.info else {
            panic!()
        };
        assert_eq!(metas[0].bounds, bounds);
        assert_eq!(metas[0].z_index, 2);
    }

    /// Assigns each leaf `PanelState` an id in encounter order, so the reader
    /// can be tested without a registry or an `App`.
    #[derive(Default)]
    struct RecordingBuilder {
        built: Vec<String>,
    }

    impl PanelBuilder for RecordingBuilder {
        fn build(&mut self, state: &PanelState, _: &PanelInfo) -> PanelId {
            self.built.push(state.panel_name.clone());
            PanelId::from_u64(self.built.len() as u64)
        }
    }

    fn tabs_state(children: Vec<PanelState>, active_index: usize) -> PanelState {
        PanelState {
            panel_name: TAB_PANEL_NAME.to_string(),
            children,
            info: PanelInfo::tabs(active_index),
        }
    }

    fn panel_state(name: &str) -> PanelState {
        PanelState {
            panel_name: name.to_string(),
            children: Vec::new(),
            info: PanelInfo::panel(serde_json::Value::Null),
        }
    }

    #[test]
    fn nested_tab_groups_are_flattened() {
        let state = tabs_state(
            vec![
                tabs_state(vec![panel_state("Alpha")], 0),
                tabs_state(vec![panel_state("Beta")], 0),
            ],
            1,
        );

        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&state, RootKind::Any, &mut builder);

        assert_eq!(builder.built, vec!["Alpha", "Beta"]);
        let PaneRef::Tabs { panels, active_ix } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels.len(), 2);
        assert_eq!(active_ix, 1);
    }

    #[test]
    fn a_bare_panel_leaf_is_wrapped_in_a_tab_group() {
        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&panel_state("Alpha"), RootKind::Any, &mut builder);

        assert!(matches!(tree.root().kind(), PaneRef::Tabs { panels, .. } if panels.len() == 1));
    }

    #[test]
    fn a_tab_panel_carrying_panel_info_is_read_as_an_empty_group() {
        // What the old `TabPanel::dump` wrote for an empty tab group.
        let state = PanelState {
            panel_name: TAB_PANEL_NAME.to_string(),
            children: Vec::new(),
            info: PanelInfo::panel(serde_json::Value::Null),
        };

        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&state, RootKind::Any, &mut builder);

        assert!(
            builder.built.is_empty(),
            "no panel is built for the phantom leaf"
        );
        assert!(matches!(tree.root().kind(), PaneRef::Tabs { panels, .. } if panels.is_empty()));
    }

    #[test]
    fn a_split_root_is_forced_even_when_the_state_is_a_tab_group() {
        let state = tabs_state(vec![panel_state("Alpha")], 0);
        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&state, RootKind::Split, &mut builder);

        assert!(matches!(tree.root().kind(), PaneRef::Split { .. }));
    }

    #[test]
    fn tile_metas_are_paired_with_children_by_index() {
        let bounds = Bounds {
            origin: point(px(1.), px(2.)),
            size: size(px(3.), px(4.)),
        };
        let state = PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: vec![panel_state("Alpha")],
            info: PanelInfo::tiles(vec![TileMeta { bounds, z_index: 5 }]),
        };

        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&state, RootKind::Any, &mut builder);

        let PaneRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels[0].bounds(), bounds);
        assert_eq!(panels[0].z_index(), 5);
    }

    #[test]
    fn a_tile_child_missing_its_meta_falls_back_to_the_default_placement() {
        let state = PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: vec![panel_state("Alpha"), panel_state("Beta")],
            info: PanelInfo::tiles(vec![TileMeta::default()]),
        };

        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&state, RootKind::Any, &mut builder);

        let PaneRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels.len(), 2, "a short metas list must not drop panels");
    }

    /// Round-trips leaves by remembering the exact `PanelState` each id came
    /// from, which is what the production invalid-panel path must also do.
    ///
    /// `PanelSource::panel_name` returns `&'static str`, which a JSON-sourced
    /// `String` can only satisfy by leaking. Rather than leak on every call
    /// to `panel_name` (as many times as the layout is dumped), each leak
    /// happens once, in `build`, when the id is minted; `panel_name` then
    /// just indexes into the already-leaked slice. Still a leak, but a
    /// bounded one, and test-only.
    #[derive(Default)]
    struct PreservingPanels {
        states: Vec<PanelState>,
        names: Vec<&'static str>,
    }

    impl PanelBuilder for PreservingPanels {
        fn build(&mut self, state: &PanelState, _: &PanelInfo) -> PanelId {
            self.states.push(state.clone());
            self.names
                .push(Box::leak(state.panel_name.clone().into_boxed_str()));
            PanelId::from_u64(self.states.len() as u64)
        }
    }

    impl PanelSource for PreservingPanels {
        fn panel_name(&self, id: PanelId) -> &'static str {
            self.names[id.as_u64() as usize - 1]
        }
        fn is_visible(&self, _: PanelId) -> bool {
            true
        }
        fn dump(&self, id: PanelId) -> PanelState {
            self.states[id.as_u64() as usize - 1].clone()
        }
    }

    fn canonicalize(json: &str) -> PanelState {
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        let mut panels = PreservingPanels::default();
        let tree = PaneTree::from_state(&state.center, RootKind::Split, &mut panels);
        tree.to_state(&panels)
    }

    #[test]
    fn canonicalization_reaches_a_fixpoint_in_one_pass() {
        for json in [
            include_str!("fixtures/layout.json"),
            include_str!("fixtures/tiles.json"),
            include_str!("fixtures/nested_splits.json"),
            include_str!("fixtures/legacy_empty_tab_group.json"),
            include_str!("fixtures/unregistered_panel.json"),
            include_str!("fixtures/zero_size_sentinel.json"),
            include_str!("fixtures/tiles_tab_panel_children.json"),
        ] {
            let once = canonicalize(json);
            let twice = {
                let wrapped = DockAreaState {
                    center: once.clone(),
                    ..Default::default()
                };
                canonicalize(&serde_json::to_string(&wrapped).unwrap())
            };
            assert_eq!(once, twice, "r(r(x)) != r(x)");
        }
    }

    #[test]
    fn an_unregistered_panel_keeps_its_payload_through_a_round_trip() {
        let state = canonicalize(include_str!("fixtures/unregistered_panel.json"));
        let leaf = &state.children[0].children[0];

        assert_eq!(leaf.panel_name, "PanelFromTheFuture");
        assert_eq!(
            leaf.info,
            PanelInfo::panel(serde_json::json!({"keep": "me"}))
        );
    }

    #[test]
    fn the_legacy_empty_tab_group_is_rewritten_into_the_tabs_form() {
        let state = canonicalize(include_str!("fixtures/legacy_empty_tab_group.json"));

        // The empty group collapses, leaving the mandatory split root.
        assert_eq!(state.panel_name, "StackPanel");
        assert!(state.children.is_empty());
    }

    #[test]
    fn nested_same_axis_splits_are_flattened_and_single_child_splits_collapse() {
        let state = canonicalize(include_str!("fixtures/nested_splits.json"));

        assert_eq!(state.panel_name, "StackPanel");
        assert_eq!(
            state.children.len(),
            3,
            "the horizontal inner split splices in and the vertical single-child split collapses"
        );
        assert!(
            state
                .children
                .iter()
                .all(|child| child.panel_name == "TabPanel")
        );
        // Order matters as much as count: a splice that reversed the spliced
        // children, or that put the collapsed single-child split ahead of
        // them, would still satisfy the two assertions above.
        assert_eq!(state.children[0].children[0].panel_name, "Alpha");
        assert_eq!(state.children[1].children[0].panel_name, "Beta");
        assert_eq!(state.children[2].children[0].panel_name, "Gamma");

        // The fixture's inner slot (50.0 + 150.0 = 200.0) does not equal its
        // outer slot (400.0), so the scale factor is a genuine 2x, not 1x:
        // `distribute_slot`'s `slot / total` and a wrongly inverted
        // `total / slot` would disagree here. The vertical single-child
        // split's own inner size (300.0) is discarded; its outer slot
        // (300.0) is what the surviving child inherits.
        let PanelInfo::Stack { sizes, .. } = &state.info else {
            panic!("expected Stack info");
        };
        assert_eq!(
            sizes,
            &vec![px(100.), px(300.), px(300.)],
            "the spliced-in sizes are scaled by outer/inner (400/200 = 2x), and the \
             collapsed split hands its own outer slot (300.0) to its surviving child"
        );
    }

    /// The shape every persisted tiles canvas actually has: each tile child
    /// is a `TabPanel` wrapping the real panel. Reading the `"TabPanel"` leaf
    /// literally would build a placeholder and drop the user's panel.
    #[test]
    fn a_tiles_child_that_is_a_tab_group_is_unwrapped_to_its_panels() {
        let json = include_str!("fixtures/tiles_tab_panel_children.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        let mut panels = PreservingPanels::default();
        let tree = PaneTree::from_state(&state.center, RootKind::Split, &mut panels);

        assert_eq!(
            panels.names,
            vec!["Alpha", "Beta", "Gamma"],
            "the real panels are built; no `TabPanel` leaf is handed to the builder"
        );

        let dumped = tree.to_state(&panels);
        let tiles = &dumped;
        assert_eq!(tiles.panel_name, "Tiles");
        assert_eq!(
            tiles
                .children
                .iter()
                .map(|child| child.panel_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Beta", "Gamma"],
        );

        let PanelInfo::Tiles { metas } = &tiles.info else {
            panic!("expected Tiles info");
        };
        // The two-panel child expands to two tiles sharing its own meta, and
        // the *next* child still gets the meta at its own child index — an
        // implementation keyed on the output index would hand Gamma the first
        // meta and lose the second entirely.
        assert_eq!(metas.len(), 3);
        assert_eq!(
            metas[0].bounds.origin.x,
            px(10.),
            "Alpha keeps child 0's meta"
        );
        assert_eq!(
            metas[1].bounds.origin.x,
            px(10.),
            "Beta shares child 0's meta"
        );
        assert_eq!(metas[1].z_index, 0);
        assert_eq!(
            metas[2].bounds.origin.x,
            px(400.),
            "Gamma gets child 1's meta"
        );
        assert_eq!(metas[2].z_index, 3);
    }

    #[test]
    fn an_empty_tab_group_on_a_tiles_canvas_contributes_no_tile() {
        let state = PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: vec![
                // The legacy empty-group form.
                PanelState {
                    panel_name: TAB_PANEL_NAME.to_string(),
                    children: Vec::new(),
                    info: PanelInfo::panel(serde_json::Value::Null),
                },
                tabs_state(vec![panel_state("Alpha")], 0),
            ],
            info: PanelInfo::tiles(vec![TileMeta::default(), TileMeta::default()]),
        };

        let mut builder = RecordingBuilder::default();
        let tree = PaneTree::from_state(&state, RootKind::Any, &mut builder);

        assert_eq!(builder.built, vec!["Alpha"]);
        let PaneRef::Tiles { panels } = tree.root().kind() else {
            panic!()
        };
        assert_eq!(panels.len(), 1);
    }

    /// The dock before the tree rewrite wrote a tiles center as a bare
    /// `Tiles`, and its `StackPanel::insert_panel` asserts every split child
    /// is a `TabPanel` or `StackPanel`. Writing the in-memory `Split` root
    /// around the canvas therefore crashes an older build on load: the wrapper
    /// is a tree invariant, not part of the persisted schema.
    #[test]
    fn a_tiles_center_is_written_the_way_the_pre_tree_dock_wrote_it() {
        let legacy = PanelState {
            panel_name: TILES_PANEL_NAME.to_string(),
            children: vec![tabs_state(vec![panel_state("Alpha")], 0)],
            info: PanelInfo::tiles(vec![TileMeta::default()]),
        };
        let mut panels = PreservingPanels::default();
        let tree = PaneTree::from_state(&legacy, RootKind::Split, &mut panels);

        let dumped = tree.to_state(&panels);

        assert_eq!(
            dumped.panel_name, "Tiles",
            "no `StackPanel` wrapper: the old reader hands the canvas to `StackPanel::insert_panel`"
        );
        assert_eq!(dumped.children[0].panel_name, "Alpha");
    }

    #[test]
    fn tile_bounds_and_z_order_survive_a_round_trip() {
        let state = canonicalize(include_str!("fixtures/tiles.json"));
        let tiles = &state;

        assert_eq!(tiles.panel_name, "Tiles");
        // `metas` and `children` are parallel arrays, matched up by index. A
        // bug that swapped `children`'s order while leaving `metas` in place
        // would misattribute bounds to the wrong panel without either array
        // changing length, so check both arrays' contents *and* that they
        // still line up by identity, not just that each array is correct in
        // isolation.
        assert_eq!(tiles.children[0].panel_name, "Alpha");
        assert_eq!(tiles.children[1].panel_name, "Beta");

        let PanelInfo::Tiles { metas } = &tiles.info else {
            panic!()
        };
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].z_index, 0, "Alpha's meta");
        assert_eq!(metas[0].bounds.origin.x, px(10.), "Alpha's meta");
        assert_eq!(metas[1].z_index, 1, "Beta's meta");
        assert_eq!(metas[1].bounds.origin.x, px(220.), "Beta's meta");
    }

    #[test]
    fn a_tree_survives_a_round_trip_exactly() {
        let json = include_str!("fixtures/nested_splits.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        let mut panels = PreservingPanels::default();
        let tree = PaneTree::from_state(&state.center, RootKind::Split, &mut panels);

        let dumped = tree.to_state(&panels);
        let mut rebuilt_panels = PreservingPanels::default();
        let rebuilt = PaneTree::from_state(&dumped, RootKind::Split, &mut rebuilt_panels);

        assert_eq!(
            tree.to_state(&panels),
            rebuilt.to_state(&rebuilt_panels),
            "load(dump(t)) must describe the same layout as t"
        );
    }

    /// Pins the one value in the schema that means something different now
    /// than it ever did before: a literal `0.0` slot size sits next to a
    /// genuine, non-collapsing `200.0` sibling, so the fixture cannot pass by
    /// accident of single-child-split collapse swallowing the slot. The
    /// reader must map the `0.0` back to `None` (unconstrained) while
    /// leaving the `200.0` sibling as `Some`, and the writer must round-trip
    /// `None` back to the literal `0.0` sentinel.
    #[test]
    fn a_zero_size_slot_round_trips_through_the_none_sentinel() {
        let json = include_str!("fixtures/zero_size_sentinel.json");
        let state: DockAreaState = serde_json::from_str(json).unwrap();
        let mut panels = PreservingPanels::default();
        let tree = PaneTree::from_state(&state.center, RootKind::Split, &mut panels);

        let PaneRef::Split { sizes, .. } = tree.root().kind() else {
            panic!("expected the root to stay a split");
        };
        assert_eq!(
            sizes,
            &[None, Some(px(200.))],
            "the 0.0 sentinel reads back as None, not as a real zero-width panel"
        );

        let dumped = tree.to_state(&panels);
        let PanelInfo::Stack { sizes, .. } = &dumped.info else {
            panic!("expected Stack info");
        };
        assert_eq!(
            sizes,
            &vec![px(0.), px(200.)],
            "the unconstrained slot writes back out as the 0.0 sentinel"
        );
    }

    /// Pins that a bare `TabPanel` root — the shape every dock in the
    /// shipped `fixtures/layout.json` (`left_dock`, `right_dock`,
    /// `bottom_dock`) actually stores — survives a round trip under
    /// `RootKind::Any` as a `Tabs` node, with its one panel intact.
    ///
    /// It does *not* pin that the forced-wrap arm in `from_state` is gated
    /// on `RootKind::Split` and correctly skipped here. It can't: `normalize`
    /// runs `collapse_root` on every pass, which un-wraps a single-child
    /// `Split` root for any `root_kind != RootKind::Split` before this test
    /// (or anything else) can observe the tree. So if the guard were deleted
    /// and the wrap fired unconditionally, `collapse_root` would strip the
    /// synthetic split right back off within the same `normalize()` call,
    /// and this test would still pass — a correctly-guarded arm and a
    /// missing guard produce byte-identical output here. The half of the
    /// guard that *is* observable — the wrap firing and surviving — is
    /// pinned by `a_split_root_is_forced_even_when_the_state_is_a_tab_group`,
    /// where `RootKind::Split` makes `collapse_root` return early instead of
    /// undoing it.
    #[test]
    fn a_bare_tab_panel_root_is_not_wrapped_under_root_kind_any() {
        let json = include_str!("fixtures/bare_tab_panel_root.json");
        let state: PanelState = serde_json::from_str(json).unwrap();
        let mut panels = PreservingPanels::default();
        let tree = PaneTree::from_state(&state, RootKind::Any, &mut panels);

        assert!(
            matches!(tree.root().kind(), PaneRef::Tabs { .. }),
            "a bare TabPanel root must stay a Tabs node under RootKind::Any, \
             not get wrapped in a synthetic split"
        );

        let dumped = tree.to_state(&panels);
        assert_eq!(dumped.panel_name, "TabPanel");
    }
}
