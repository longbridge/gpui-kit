use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, ClickEvent, ElementId, ImageSource, InteractiveElement as _,
    IntoElement, MouseButton, ObjectFit, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, StyledImage as _, Window, div, img,
    prelude::FluentBuilder as _, relative, rems,
};

use crate::{
    ActiveTheme as _, InteractiveElementExt as _, Sizable, Size, StyledExt as _, h_flex,
    shimmer::{ShimmerStyle, ShimmerText},
    v_flex,
};

/// The lifecycle status of an attachment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AttachmentStatus {
    /// The attachment has been selected and is waiting to be uploaded.
    Pending,
    /// The attachment is currently being uploaded.
    Uploading,
    /// The upload has completed and the attachment is being processed.
    Processing,
    /// The attachment failed to upload or process.
    Failed,
    /// The attachment is ready.
    #[default]
    Complete,
}

impl AttachmentStatus {
    /// Returns whether the attachment is waiting to start.
    pub fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns whether the attachment is being uploaded.
    pub fn is_uploading(self) -> bool {
        matches!(self, Self::Uploading)
    }

    /// Returns whether the attachment is being processed.
    pub fn is_processing(self) -> bool {
        matches!(self, Self::Processing)
    }

    /// Returns whether the attachment has failed.
    pub fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Returns whether the attachment is ready.
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }

    /// Returns whether the attachment is in an in-progress state.
    pub fn is_in_progress(self) -> bool {
        matches!(self, Self::Uploading | Self::Processing)
    }
}

/// A file or image attachment composed from media, content, and actions slots.
#[derive(IntoElement)]
pub struct Attachment {
    id: Option<ElementId>,
    style: StyleRefinement,
    status: AttachmentStatus,
    size: Size,
    axis: Axis,
    media: Option<AttachmentMedia>,
    content: Option<AttachmentContent>,
    actions: Option<AttachmentActions>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl Attachment {
    /// Create an attachment in the [`AttachmentStatus::Complete`] state.
    pub fn new() -> Self {
        Self {
            id: None,
            style: StyleRefinement::default(),
            status: AttachmentStatus::Complete,
            size: Size::Medium,
            axis: Axis::Horizontal,
            media: None,
            content: None,
            actions: None,
            on_click: None,
        }
    }

    /// Set a stable identity for the whole-card click layer.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Make the whole card clickable, e.g. to open a preview.
    ///
    /// The click layer is painted below the actions slot, so action buttons
    /// stay independently clickable. Click state needs a stable identity, so
    /// the handler takes effect only together with [`Self::id`].
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the attachment lifecycle status.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the attachment layout axis.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Set the media slot.
    pub fn media(mut self, media: AttachmentMedia) -> Self {
        self.media = Some(media);
        self
    }

    /// Set the metadata content slot.
    pub fn content(mut self, content: AttachmentContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the actions slot.
    pub fn actions(mut self, actions: AttachmentActions) -> Self {
        self.actions = Some(actions);
        self
    }
}

impl Default for Attachment {
    fn default() -> Self {
        Self::new()
    }
}

impl Sizable for Attachment {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Attachment {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Attachment {
    fn layout_slots(&mut self) {
        let size = self.size;
        let axis = self.axis;
        let status = self.status;

        self.media = self
            .media
            .take()
            .map(|media| media.layout(size, status, axis));
        self.content = self
            .content
            .take()
            .map(|content| content.layout(axis, status));
        self.actions = self
            .actions
            .take()
            .map(|actions| actions.layout_for_axis(axis));
    }
}

impl RenderOnce for Attachment {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let size = self.size;
        let axis = self.axis;
        let status = self.status;
        let has_media = self.media.is_some();
        let has_content = self.content.is_some();
        let clickable = self.id.is_some() && self.on_click.is_some();

        self.layout_slots();

        div()
            .relative()
            .flex()
            .flex_none()
            .max_w_full()
            .min_w_0()
            .rounded(if size == Size::XSmall {
                tokens.radius.xl
            } else {
                cx.theme().radius_2xl()
            })
            .border_1()
            .border_color(if status.is_failed() {
                tokens.colors.destructive.opacity(0.3)
            } else {
                tokens.colors.border
            })
            .when(status.is_pending(), |this| this.border_dashed())
            .bg(tokens.colors.background)
            .text_color(tokens.colors.foreground)
            // Register `hover` unconditionally: a conditionally registered
            // hover style stays cached when the condition later flips off.
            .hover(move |style| {
                if clickable {
                    style.bg(tokens.colors.muted.opacity(0.5))
                } else {
                    style
                }
            })
            .line_height(relative(1.25))
            .map(|this| attachment_size_style(this, size, has_media, has_content))
            .map(|this| match axis {
                Axis::Horizontal => this.min_w_40().items_center(),
                Axis::Vertical => this
                    .when(has_content, |this| this.w(rems(7.5)))
                    .when(!has_content, |this| this.w_24())
                    .flex_col()
                    .items_start(),
            })
            .when_some(self.media, |this, media| this.child(media))
            .when_some(self.content, |this, content| this.child(content))
            .when_some(self.id.zip(self.on_click), |this, (id, on_click)| {
                // The click layer is painted before the actions slot, so the
                // actions' hitboxes stay on top and their buttons keep working.
                this.child(
                    div()
                        .id(id)
                        .absolute()
                        .inset_0()
                        .on_click(move |event, window, cx| on_click(event, window, cx)),
                )
            })
            .when_some(self.actions, |this, actions| this.child(actions))
            .refine_style(&self.style)
    }
}

/// The media slot for an attachment.
///
/// Add an icon or another element as a child for an icon-style preview. Use
/// [`Self::src`] when the attachment has an image preview.
#[derive(IntoElement)]
pub struct AttachmentMedia {
    style: StyleRefinement,
    size: Option<Size>,
    status: AttachmentStatus,
    axis: Axis,
    source: Option<ImageSource>,
    children: Vec<AnyElement>,
}

impl AttachmentMedia {
    /// Create an empty media slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: None,
            status: AttachmentStatus::Complete,
            axis: Axis::Horizontal,
            source: None,
            children: Vec::new(),
        }
    }

    /// Set an image preview source.
    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add centered content above the preview without dimming it during loading.
    pub fn overlay(mut self, overlay: impl IntoElement) -> Self {
        self.children.push(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(overlay)
                .into_any_element(),
        );
        self
    }

    fn layout(mut self, size: Size, status: AttachmentStatus, axis: Axis) -> Self {
        if self.size.is_none() {
            self.size = Some(size);
        }
        self.status = status;
        self.axis = axis;
        self
    }
}

impl Default for AttachmentMedia {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AttachmentMedia {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for AttachmentMedia {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl Styled for AttachmentMedia {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentMedia {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let resolved_size = self.size.unwrap_or_default();
        let radius = match resolved_size {
            Size::XSmall => tokens.radius.sm,
            Size::Small | Size::Medium | Size::Large | Size::Size(_) => tokens.radius.md,
        };
        let source = self.source;
        let has_source = source.is_some();
        let failed_media = self.status.is_failed() && !has_source;
        let dimmed_image = has_source
            && !matches!(
                self.status,
                AttachmentStatus::Pending | AttachmentStatus::Complete
            );
        let children = self.children;

        div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .when(self.axis == Axis::Horizontal, |this| match resolved_size {
                Size::XSmall => this.size_7(),
                Size::Small => this.size_8(),
                Size::Medium => this.size_10(),
                Size::Large => this.size_12(),
                Size::Size(size) => this.size(size),
            })
            .when(self.axis == Axis::Vertical, |this| {
                this.w_full().aspect_ratio(1.)
            })
            .rounded(radius)
            .bg(if failed_media {
                tokens.colors.destructive.opacity(0.1)
            } else {
                tokens.colors.muted
            })
            .text_color(if failed_media {
                tokens.colors.destructive
            } else {
                tokens.colors.foreground
            })
            .when_some(source, |this, source| {
                this.child(
                    img(source)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .when(dimmed_image, |this| this.opacity(0.6)),
                )
            })
            .children(children)
            .refine_style(&self.style)
    }
}

/// The metadata slot for an attachment.
#[derive(IntoElement)]
pub struct AttachmentContent {
    style: StyleRefinement,
    vertical_layout: bool,
    children: Vec<AttachmentContentChild>,
}

enum AttachmentContentChild {
    Title(AttachmentTitle),
    Description(AttachmentDescription),
    Element(AnyElement),
}

impl AttachmentContent {
    /// Create an empty metadata slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            vertical_layout: false,
            children: Vec::new(),
        }
    }

    /// Add a title that automatically inherits the attachment lifecycle status.
    pub fn title(mut self, title: AttachmentTitle) -> Self {
        self.children.push(AttachmentContentChild::Title(title));
        self
    }

    /// Add a description that automatically inherits the attachment lifecycle status.
    pub fn description(mut self, description: AttachmentDescription) -> Self {
        self.children
            .push(AttachmentContentChild::Description(description));
        self
    }

    fn layout(mut self, axis: Axis, status: AttachmentStatus) -> Self {
        self.vertical_layout = axis == Axis::Vertical;

        for child in &mut self.children {
            match child {
                AttachmentContentChild::Title(title) => {
                    if title.status.is_none() {
                        title.status = Some(status);
                    }
                }
                AttachmentContentChild::Description(description) => {
                    if description.status.is_none() {
                        description.status = Some(status);
                    }
                }
                AttachmentContentChild::Element(_) => {}
            }
        }

        self
    }
}

impl Default for AttachmentContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AttachmentContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(AttachmentContentChild::Element));
    }
}

impl Styled for AttachmentContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        v_flex()
            .max_w_full()
            .min_w_0()
            .flex_1()
            .gap_0p5()
            .line_height(relative(1.25))
            .when(self.vertical_layout, |this| this.w_full().px_1())
            .children(self.children.into_iter().map(|child| match child {
                AttachmentContentChild::Title(title) => title.into_any_element(),
                AttachmentContentChild::Description(description) => description.into_any_element(),
                AttachmentContentChild::Element(element) => element,
            }))
            .refine_style(&self.style)
    }
}

/// A single-line attachment title.
#[derive(IntoElement)]
pub struct AttachmentTitle {
    style: StyleRefinement,
    text: SharedString,
    status: Option<AttachmentStatus>,
    shimmer_style: Option<ShimmerStyle>,
}

impl AttachmentTitle {
    /// Create an attachment title.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
            status: None,
            shimmer_style: None,
        }
    }

    /// Override the attachment lifecycle status used for the loading shimmer.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Customize the shimmer used while this attachment is uploading or processing.
    pub fn with_shimmer_style(mut self, style: ShimmerStyle) -> Self {
        self.shimmer_style = Some(style);
        self
    }
}

impl Styled for AttachmentTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let loading = self.status.is_some_and(AttachmentStatus::is_in_progress);

        div()
            .max_w_full()
            .min_w_0()
            .truncate()
            .font_medium()
            .map(|this| {
                if loading {
                    this.child(
                        ShimmerText::new(self.text).when_some(self.shimmer_style, |this, style| {
                            this.with_shimmer_style(style)
                        }),
                    )
                } else {
                    this.child(self.text)
                }
            })
            .refine_style(&self.style)
    }
}

/// A single-line attachment description or status message.
#[derive(IntoElement)]
pub struct AttachmentDescription {
    style: StyleRefinement,
    text: SharedString,
    status: Option<AttachmentStatus>,
}

impl AttachmentDescription {
    /// Create an attachment description.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
            status: None,
        }
    }

    /// Set the status used for the semantic description color.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = Some(status);
        self
    }
}

impl Styled for AttachmentDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let color = self
            .status
            .is_some_and(AttachmentStatus::is_failed)
            .then(|| tokens.colors.destructive.opacity(0.8))
            .unwrap_or(tokens.colors.muted_foreground);

        div()
            .max_w_full()
            .min_w_0()
            .truncate()
            .text_xs()
            .line_height(relative(1.25))
            .text_color(color)
            .child(self.text)
            .refine_style(&self.style)
    }
}

/// A composition slot for attachment actions.
///
/// Add existing [`crate::button::Button`] or other controls as children. A
/// separate attachment-specific action wrapper is intentionally unnecessary.
#[derive(IntoElement)]
pub struct AttachmentActions {
    style: StyleRefinement,
    vertical_layout: bool,
    children: Vec<AnyElement>,
}

impl AttachmentActions {
    /// Create an empty actions slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            vertical_layout: false,
            children: Vec::new(),
        }
    }

    fn layout_for_axis(mut self, axis: Axis) -> Self {
        self.vertical_layout = axis == Axis::Vertical;
        self
    }
}

impl Default for AttachmentActions {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AttachmentActions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for AttachmentActions {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentActions {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .when(self.vertical_layout, |this| {
                this.absolute().top_3().right_3()
            })
            // The actions cluster owns its presses: an action (or the gap
            // between actions) must not also arm the whole-card click layer
            // below, mirroring the shadcn stacking where actions sit above
            // the trigger. Buttons run first in the bubble phase, so they
            // are unaffected.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(self.children)
            .refine_style(&self.style)
    }
}

fn attachment_size_style<T: Styled + gpui::prelude::FluentBuilder>(
    element: T,
    size: Size,
    has_media: bool,
    has_content: bool,
) -> T {
    match size {
        Size::XSmall => element
            .gap_1p5()
            .text_xs()
            .when(has_content, |this| this.px_1p5().py_1())
            .when(has_media, |this| this.p_1()),
        Size::Small => element
            .gap_2p5()
            .text_xs()
            .when(has_content, |this| this.px_2().py_1p5())
            .when(has_media, |this| this.p_1p5()),
        Size::Medium => element
            .gap_2()
            .text_sm()
            .when(has_content, |this| this.px_2p5().py_2())
            .when(has_media, |this| this.p_2()),
        Size::Large => element
            .gap_3()
            .text_base()
            .when(has_content, |this| this.px_4().py_3())
            .when(has_media, |this| this.p_3()),
        Size::Size(value) => element
            .gap_1()
            .text_size(value * 0.875)
            .when(has_content || has_media, |this| this.p(value * 0.25)),
    }
}

/// A horizontally scrollable row of attachments.
#[derive(IntoElement)]
pub struct AttachmentGroup {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl AttachmentGroup {
    /// Create an empty attachment group with a stable scroll identifier.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for AttachmentGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for AttachmentGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .id(self.id)
            .w_full()
            .min_w_0()
            .gap_3()
            .py_1()
            .overflow_x_scroll()
            .lock_scroll_axis()
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_builder() {
        let mut attachment = Attachment::new()
            .status(AttachmentStatus::Uploading)
            .axis(Axis::Vertical)
            .with_size(Size::Small)
            .media(AttachmentMedia::new().src("preview.png"))
            .content(
                AttachmentContent::new()
                    .title(AttachmentTitle::new("report.pdf"))
                    .description(AttachmentDescription::new("Uploading")),
            )
            .actions(AttachmentActions::new().child("Cancel"));

        assert_eq!(attachment.status, AttachmentStatus::Uploading);
        assert_eq!(attachment.axis, Axis::Vertical);
        assert_eq!(attachment.size, Size::Small);
        assert!(attachment.media.is_some());
        assert!(attachment.content.is_some());
        assert!(attachment.actions.is_some());

        attachment.layout_slots();
        assert_eq!(attachment.media.as_ref().unwrap().size, Some(Size::Small));
        assert_eq!(
            attachment.media.as_ref().unwrap().status,
            AttachmentStatus::Uploading
        );
        assert!(attachment.content.as_ref().unwrap().vertical_layout);
        assert!(attachment.actions.as_ref().unwrap().vertical_layout);
    }

    #[test]
    fn test_attachment_whole_card_click_builder() {
        assert!(Attachment::new().id.is_none());
        assert!(Attachment::new().on_click.is_none());

        let clickable = Attachment::new()
            .id("report-attachment")
            .on_click(|_, _, _| {});
        assert_eq!(clickable.id, Some("report-attachment".into()));
        assert!(clickable.on_click.is_some());
    }

    #[test]
    fn test_attachment_defaults_and_status_helpers() {
        assert_eq!(Attachment::new().status, AttachmentStatus::Complete);
        assert_eq!(AttachmentStatus::default(), AttachmentStatus::Complete);
        assert!(AttachmentStatus::Pending.is_pending());
        assert!(AttachmentStatus::Uploading.is_in_progress());
        assert!(AttachmentStatus::Processing.is_processing());
        assert!(AttachmentStatus::Failed.is_failed());
        assert!(AttachmentStatus::Complete.is_complete());
        assert!(!AttachmentStatus::Complete.is_in_progress());
    }

    #[test]
    fn test_attachment_slots_are_composable() {
        let media = AttachmentMedia::new().child("icon");
        assert_eq!(media.children.len(), 1);

        let content = AttachmentContent::new()
            .title(AttachmentTitle::new("name"))
            .description(AttachmentDescription::new("Details"))
            .child("Custom progress");
        assert_eq!(content.children.len(), 3);
        assert!(matches!(
            content.children[0],
            AttachmentContentChild::Title(_)
        ));
        assert!(matches!(
            content.children[1],
            AttachmentContentChild::Description(_)
        ));
        assert!(matches!(
            content.children[2],
            AttachmentContentChild::Element(_)
        ));

        let legacy = AttachmentContent::new().child(AttachmentTitle::new("legacy"));
        assert!(matches!(
            legacy.children[0],
            AttachmentContentChild::Element(_)
        ));

        let actions = AttachmentActions::new().child("remove");
        assert_eq!(actions.children.len(), 1);
    }

    #[test]
    fn test_attachment_typed_content_inherits_status() {
        let mut attachment = Attachment::new()
            .status(AttachmentStatus::Uploading)
            .content(
                AttachmentContent::new()
                    .title(AttachmentTitle::new("report.pdf"))
                    .description(AttachmentDescription::new("Uploading")),
            );

        attachment.layout_slots();

        let content = attachment.content.unwrap();
        let AttachmentContentChild::Title(title) = &content.children[0] else {
            panic!("expected the typed title slot");
        };
        assert_eq!(title.status, Some(AttachmentStatus::Uploading));

        let AttachmentContentChild::Description(description) = &content.children[1] else {
            panic!("expected the typed description slot");
        };
        assert_eq!(description.status, Some(AttachmentStatus::Uploading));
    }

    #[test]
    fn test_attachment_explicit_child_status_overrides_parent() {
        let mut attachment = Attachment::new().status(AttachmentStatus::Failed).content(
            AttachmentContent::new()
                .title(AttachmentTitle::new("report.pdf").status(AttachmentStatus::Processing))
                .description(
                    AttachmentDescription::new("Previous upload completed")
                        .status(AttachmentStatus::Complete),
                ),
        );

        attachment.layout_slots();

        let content = attachment.content.unwrap();
        let AttachmentContentChild::Title(title) = &content.children[0] else {
            panic!("expected the typed title slot");
        };
        assert_eq!(title.status, Some(AttachmentStatus::Processing));

        let AttachmentContentChild::Description(description) = &content.children[1] else {
            panic!("expected the typed description slot");
        };
        assert_eq!(description.status, Some(AttachmentStatus::Complete));
    }

    #[test]
    fn test_attachment_title_keeps_custom_shimmer_style() {
        let mut attachment = Attachment::new()
            .status(AttachmentStatus::Processing)
            .content(
                AttachmentContent::new().title(
                    AttachmentTitle::new("report.pdf")
                        .with_shimmer_style(ShimmerStyle::new().spread(0.45).reverse(true)),
                ),
            );

        attachment.layout_slots();

        let content = attachment.content.unwrap();
        let AttachmentContentChild::Title(title) = &content.children[0] else {
            panic!("expected the typed title slot");
        };
        assert_eq!(title.status, Some(AttachmentStatus::Processing));
        assert!(title.shimmer_style.is_some());
    }

    #[test]
    fn test_attachment_media_preview_keeps_children_and_overlays() {
        let media = AttachmentMedia::new()
            .src("preview.png")
            .child("Existing overlay")
            .overlay("Centered overlay");

        assert!(media.source.is_some());
        assert_eq!(media.children.len(), 2);
    }

    #[test]
    fn test_attachment_media_size_inherits_root_unless_explicit() {
        let inherited =
            AttachmentMedia::new().layout(Size::Small, AttachmentStatus::Complete, Axis::Vertical);
        assert_eq!(inherited.size, Some(Size::Small));
        assert_eq!(inherited.axis, Axis::Vertical);

        let explicit = AttachmentMedia::new().with_size(Size::XSmall).layout(
            Size::Large,
            AttachmentStatus::Failed,
            Axis::Horizontal,
        );
        assert_eq!(explicit.size, Some(Size::XSmall));
        assert_eq!(explicit.status, AttachmentStatus::Failed);
    }

    #[test]
    fn test_attachment_group_builder() {
        let group = AttachmentGroup::new("attachments")
            .child("First")
            .child("Second");

        assert_eq!(group.children.len(), 2);
    }

    mod click_dispatch {
        use std::{cell::Cell, rc::Rc};

        use gpui::{Context, Modifiers, Render, TestAppContext, point, px};

        use super::super::*;
        use crate::button::Button;

        struct AttachmentClickHarness {
            card_clicks: Rc<Cell<usize>>,
            action_clicks: Rc<Cell<usize>>,
        }

        impl Render for AttachmentClickHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let card_clicks = self.card_clicks.clone();
                let action_clicks = self.action_clicks.clone();

                Attachment::new()
                    .id("attachment")
                    .w(px(200.))
                    .h(px(60.))
                    .on_click(move |_, _, _| card_clicks.set(card_clicks.get() + 1))
                    .actions(
                        AttachmentActions::new().child(
                            Button::new("open")
                                .w(px(40.))
                                .h(px(40.))
                                .on_click(move |_, _, _| {
                                    action_clicks.set(action_clicks.get() + 1)
                                }),
                        ),
                    )
            }
        }

        #[gpui::test]
        fn whole_card_click_stays_below_the_actions(cx: &mut TestAppContext) {
            cx.update(crate::init);
            let card_clicks = Rc::new(Cell::new(0));
            let action_clicks = Rc::new(Cell::new(0));
            let (_, cx) = cx.add_window_view({
                let card_clicks = card_clicks.clone();
                let action_clicks = action_clicks.clone();
                move |_, _| AttachmentClickHarness {
                    card_clicks,
                    action_clicks,
                }
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));

            // A click on an action must not also fire the whole-card handler.
            cx.simulate_click(point(px(20.), px(30.)), Modifiers::default());
            assert_eq!(action_clicks.get(), 1);
            assert_eq!(card_clicks.get(), 0);

            // A click elsewhere on the card fires the whole-card handler.
            cx.simulate_click(point(px(150.), px(30.)), Modifiers::default());
            assert_eq!(action_clicks.get(), 1);
            assert_eq!(card_clicks.get(), 1);
        }
    }
}
