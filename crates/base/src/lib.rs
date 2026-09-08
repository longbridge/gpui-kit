//! Behavior and infrastructure foundations for GPUI applications.
//!
//! Primitives deliberately avoid presentation styles. Layout, positioning,
//! colors, sizing, and motion belong to applications or the
//! `gpui-component` façade.

mod accordion;
pub mod actions;
mod alert_dialog;
pub mod animation;
#[doc(hidden)]
pub mod async_util;
mod auto_scroll;
mod avatar;
mod button;
mod calendar;
mod checkbox;
mod collapsible;
mod color_picker;
mod combobox;
pub mod component_traits;
mod date_picker;
mod dialog;
pub mod dock;
mod element_ext;
mod event;
mod focus_trap;
mod geometry;
mod global_state;
mod history;
mod hover_card;
mod index_path;
pub mod input;
mod link;
mod list_settings;
#[cfg(all(target_os = "macos", not(test)))]
mod macos_accessibility;
mod measure;
pub mod motion;
mod nav_stack;
mod number_input;
mod observe;
mod otp_input;
mod pagination;
mod popover;
mod popup;
mod positioner;
mod progress;
mod radio;
mod radio_group;
mod resizable;
mod scrollable_mask;
mod scrollbar;
mod select;
mod selectable_text;
mod sheet;
pub mod slider;
mod state_style;
mod styled;
mod switch;
mod table;
mod tabs;
pub mod text;
mod text_boundary;
mod text_selection;
mod theme;
pub mod theme_tokens;
mod toast;
mod toggle;
mod toggle_group;
mod tooltip;
mod tree;
mod undo_history;
mod virtual_list;

pub use accordion::{Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger};
pub use alert_dialog::{
    AlertDialog, AlertDialogAction, AlertDialogBackdrop, AlertDialogCancel, AlertDialogClose,
    AlertDialogDescription, AlertDialogPopup, AlertDialogTitle, AlertDialogTrigger,
};
pub use auto_scroll::AutoScroll;
pub use avatar::{Avatar, AvatarFallback, AvatarImage};
pub use button::{Button, ButtonStyles};
pub use calendar::{
    Calendar, CalendarEvent, CalendarItem, CalendarItemKind, CalendarItemState, CalendarState,
    CalendarView, Date, IntervalMatcher, Matcher, RangeMatcher,
};
pub use checkbox::{
    Checkbox, CheckboxIndicator, CheckboxIndicatorStyles, CheckboxState, CheckboxStyles,
};
pub use collapsible::Collapsible;
pub use color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState, ColorSwatch, HslaSliders};
pub use combobox::Combobox;
pub use component_traits::FocusableExt;
pub use component_traits::{Disableable, Selectable};
pub use date_picker::DatePicker;
pub use dialog::{
    Dialog, DialogBackdrop, DialogChangeReason, DialogClose, DialogDescription, DialogHandle,
    DialogPopup, DialogTitle, DialogTrigger,
};
pub use element_ext::ElementExt;
pub use event::{InteractiveElementExt, OngoingScrollExt};
pub use focus_trap::FocusTrapElement;
#[doc(hidden)]
pub use focus_trap::active_focus_trap;
pub use geometry::*;
pub use global_state::{DeferredPopover, GlobalState};
pub use history::History;
pub use hover_card::{HoverCard, HoverCardState};
pub use index_path::IndexPath;
pub use input::{Editor, Input, InputBase, InputStyles, Textarea};
pub use link::{Link, LinkStyles};
pub use list_settings::ListSettings;
#[cfg(all(target_os = "macos", not(test)))]
#[doc(hidden)]
pub use macos_accessibility::install_window_hit_test_forwarder;
#[doc(hidden)]
pub use measure::measurement_enabled;
pub use measure::{Measure, measure, measure_if};
pub use motion::{
    Discrete, DiscreteError, Easing, EasingError, Interpolate, IterationCount, Keyframe,
    KeyframeError, Keyframes, LinearStop, MotionPhase, MotionReveal, MotionStatus, MotionTransform,
    MotionValue, PlaybackDirection, Presence, PresencePhase, PresenceSample, SignedDuration,
    Spring, SpringError, Stagger, StaggerOrigin, StepPosition, Timing, TimingSample, Transition,
    TransitionId, animate_keyframes, spring, transition, transition_with_status,
};
pub use nav_stack::{NavMotion, NavOperation, NavPage, NavStack, NavStackEvent, NavStackState};
pub use number_input::{
    Decrement, Increment, NumberInput, NumberInputEvent, NumberInputText, NumberStep, StepAction,
    step_value,
};
pub use observe::{ObservedElement, TestSupportExt};
pub use otp_input::{OtpEvent, OtpInput, OtpState};
pub use pagination::{Pagination, PaginationItem, PaginationState};
pub use popover::{Popover, PopoverState};
pub use popup::{POPUP_PRIORITY, Popup};
pub use positioner::{Align, Positioner, ResolvedPosition};
pub use progress::{Progress, ProgressIndicator, ProgressTrack};
pub use radio::{Radio, RadioStyles};
pub use radio_group::RadioGroup;
#[doc(hidden)]
pub use resizable::{PANEL_MIN_SIZE, resize_handle};
pub use resizable::{
    ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, ResizeHandleContext,
    ResizeHandleRenderer, h_resizable, resizable_panel, v_resizable,
};
pub use scrollable_mask::ScrollableMask;
pub use scrollbar::{
    Scrollbar, ScrollbarAxis, ScrollbarEntrance, ScrollbarHandle, ScrollbarMode, ScrollbarMotion,
    ScrollbarStyles, ScrollbarThumbStyle, ScrollbarTrackStyle,
};
pub use select::Select;
pub use selectable_text::SelectableText;
pub use sheet::Sheet;
pub use slider::{Slider, SliderIndicator, SliderThumb, SliderTrack};
pub use state_style::StateStyle;
#[cfg(any(feature = "inspector", debug_assertions))]
pub use styled::styled_ext_reflection_methods;
pub use styled::{RoleOverride, StyledExt, box_shadow, h_flex, v_flex};
pub use switch::{
    Switch, SwitchStyles, SwitchThumb, SwitchThumbStyles, SwitchTrack, SwitchTrackStyles,
};
pub use table::{Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow};
pub use tabs::{Tab, TabStyles, Tabs};
pub use text::{
    MarkdownExtensions, MarkdownNode, MarkdownPlugin, SelectionFormat, TableData, Text, TextView,
    TextViewDefaults, TextViewPlugin, TextViewState, TextViewStyle, html, markdown,
};
pub use text_selection::{
    TextSelection, TextSelectionContentKey, TextSelectionCoverage, TextSelectionEndpoint,
    TextSelectionEvent, TextSelectionHandle, TextSelectionLayer, TextSelectionProjection,
    TextSelectionRegistration, TextSelectionRun, TextSelectionScopeId, TextSelectionSnapshot,
    TextSelectionWindowPoints,
};
pub use theme::{ResizableTheme, ScrollbarTheme, Theme, ThemeAppearance};
pub use theme_tokens::{
    ColorTokens, RadiusTokens, SemanticThemeTokens, ShadowTokens, SpacingTokens, TextStyleToken,
    TypographyTokens,
};
pub use toast::{
    Toast, ToastAdvance, ToastManager, ToastMotion, ToastOptions, ToastStack, ToastStackState,
    ToastTransitionStatus,
};
pub use toggle::{Toggle, ToggleStyles};
pub use toggle_group::ToggleGroup;
pub use tooltip::{Tooltip, TooltipOverlay, TooltipPositioner, TooltipRequest, TooltipTransition};
pub use tree::{Tree, TreeEntry, TreeEntryState, TreeEvent, TreeItem, TreeState};
#[doc(hidden)]
pub use tree::{init as init_tree, key_context as tree_key_context};
pub use undo_history::UndoHistory;
#[doc(hidden)]
pub use virtual_list::virtual_list;
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};

use gpui::App;

/// Initializes global infrastructure owned by the base layer.
pub fn init(cx: &mut App) {
    let _ = Theme::global_mut(cx);
    GlobalState::init(cx);
    dialog::init(cx);
    focus_trap::init(cx);
    popover::init(cx);
    sheet::init(cx);
    combobox::init(cx);
    color_picker::init(cx);
    select::init(cx);
    number_input::init(cx);
    input::init(cx);
    tree::init(cx);
    text::init(cx);
}

#[cfg(feature = "test-support")]
pub mod test_support;
