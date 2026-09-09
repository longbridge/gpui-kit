use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let gpui = match crate::crate_path::gpui() {
        Ok(path) => path,
        Err(error) => return error.into_compile_error().into(),
    };

    let expanded = quote! {
        impl #impl_generics #gpui::IntoElement for #type_name #type_generics #where_clause {
            type Element = Self;

            fn into_element(self) -> Self::Element {
                self
            }
        }

        impl #impl_generics #type_name #type_generics #where_clause {
            /// Element-local cell holding the last cursor position (plot-relative), shared by
            /// the generated `prepaint`/`paint` so the cell type lives in a single place.
            #[doc(hidden)]
            fn __plot_tooltip_cursor(
                global_id: &#gpui::GlobalElementId,
                window: &mut #gpui::Window,
            ) -> std::rc::Rc<std::cell::Cell<Option<#gpui::Point<#gpui::Pixels>>>> {
                window.with_element_state(global_id, |prev, _| {
                    let cell: std::rc::Rc<
                        std::cell::Cell<Option<#gpui::Point<#gpui::Pixels>>>,
                    > = prev.unwrap_or_default();
                    (cell.clone(), cell)
                })
            }
        }

        impl #impl_generics #gpui::Element for #type_name #type_generics #where_clause {
            type RequestLayoutState = ();
            // Carries the hitbox used for occlusion-aware hover detection, the plot's
            // prepainted child elements and the prepainted tooltip overlay (if any)
            // from `prepaint` to `paint`.
            type PrepaintState = (
                Option<#gpui::Hitbox>,
                Vec<#gpui::AnyElement>,
                Option<#gpui::AnyElement>,
            );

            fn id(&self) -> Option<#gpui::ElementId> {
                // `Some` opts the plot in to interactive tooltips; `None` (the default)
                // keeps the element a pure, non-interactive plot identical to before.
                <Self as Plot>::id(self)
            }

            fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
                None
            }

            fn request_layout(
                &mut self,
                _: Option<&#gpui::GlobalElementId>,
                _: Option<&#gpui::InspectorElementId>,
                window: &mut #gpui::Window,
                cx: &mut #gpui::App,
            ) -> (#gpui::LayoutId, Self::RequestLayoutState) {
                let style = #gpui::Style {
                    size: #gpui::Size::full(),
                    ..Default::default()
                };

                (window.request_layout(style, None, cx), ())
            }

            fn prepaint(
                &mut self,
                global_id: Option<&#gpui::GlobalElementId>,
                _: Option<&#gpui::InspectorElementId>,
                bounds: #gpui::Bounds<#gpui::Pixels>,
                _: &mut Self::RequestLayoutState,
                window: &mut #gpui::Window,
                cx: &mut #gpui::App,
            ) -> Self::PrepaintState {
                // Child elements must be laid out here: `layout_as_root` / `prepaint_at`
                // are prepaint-only. Above the early return below, so plots without an
                // id still get their children.
                let children = <Self as Plot>::prepaint(self, bounds, window, cx);

                // No id => tooltips disabled => behave exactly like a non-interactive plot.
                let Some(global_id) = global_id else {
                    return (None, children, None);
                };

                // The hitbox lets the mouse handler hit-test with occlusion awareness:
                // `Hitbox::is_hovered` returns false while an occluding hitbox (e.g. an
                // open popup menu) is above the plot, unlike a plain bounds test.
                let hitbox = window.insert_hitbox(bounds, #gpui::HitboxBehavior::Normal);

                let overlay = (|| {
                    // The cell only gates visibility: it holds the cursor recorded by the
                    // mouse handler / per-frame sync in `paint`, which is one frame stale
                    // during scrolling. Rendering that cached point while the bounds move
                    // makes the tooltip jitter, so derive the position from the live mouse
                    // position and this frame's bounds instead.
                    Self::__plot_tooltip_cursor(global_id, window).get()?;
                    let mouse = window.mouse_position();
                    if !bounds.contains(&mouse) {
                        return None;
                    }
                    let position = mouse - bounds.origin;
                    let state = <Self as Plot>::tooltip_state(self, position, bounds, cx)?;

                    // Pass the live cursor so the tooltip box can follow it; the crosshair and
                    // dots in `state` stay snapped to the data point by `tooltip_state`.
                    //
                    // The overlay paints in the plot's own layer, so the crosshair and dots stay
                    // below content drawn over the plot. The tooltip box defers itself (see
                    // `plot::tooltip::Tooltip`) to paint above sibling content, since it can
                    // extend past the plot bounds.
                    let mut overlay = <Self as Plot>::tooltip(self, &state, position, bounds, window, cx)?;
                    overlay.prepaint_as_root(bounds.origin, bounds.size.into(), window, cx);
                    Some(overlay)
                })();

                (Some(hitbox), children, overlay)
            }

            fn paint(
                &mut self,
                global_id: Option<&#gpui::GlobalElementId>,
                _: Option<&#gpui::InspectorElementId>,
                bounds: #gpui::Bounds<#gpui::Pixels>,
                _: &mut Self::RequestLayoutState,
                prepaint: &mut Self::PrepaintState,
                window: &mut #gpui::Window,
                cx: &mut #gpui::App,
            ) {
                <Self as Plot>::paint(self, bounds, window, cx);

                let (hitbox, children, overlay) = prepaint;

                // Child elements (e.g. element labels) paint above the plot graphics.
                for child in children.iter_mut() {
                    child.paint(window, cx);
                }

                if let (Some(global_id), Some(hitbox)) = (global_id, hitbox.as_ref()) {
                    // Record the cursor position into element-local state on every move so the
                    // next frame can hit-test it. The handler never touches `self`, satisfying
                    // the `'static` bound; it only captures the (Copy) bounds, the hitbox id
                    // and the state cell.
                    let cell = Self::__plot_tooltip_cursor(global_id, window);
                    let hitbox = hitbox.clone();

                    // Scrolling (or any relayout) moves the plot under a stationary cursor
                    // without emitting MouseMoveEvent, so re-derive the cursor state every
                    // frame: `mouse_hit_test` is recomputed against this frame's hitboxes
                    // right before paint, so `is_hovered` is fresh here. Only a visibility
                    // flip needs a corrective frame — `prepaint` derives the tooltip position
                    // from the live mouse, not from the cell. `refresh()` is a no-op while
                    // drawing; schedule via `request_animation_frame`.
                    let next = if hitbox.is_hovered(window) {
                        Some(window.mouse_position() - bounds.origin)
                    } else {
                        None
                    };
                    if cell.get() != next {
                        let visibility_changed = cell.get().is_some() != next.is_some();
                        cell.set(next);
                        if visibility_changed {
                            window.request_animation_frame();
                        }
                    }

                    window.on_mouse_event(
                        move |e: &#gpui::MouseMoveEvent, _, window: &mut #gpui::Window, _| {
                            // `is_hovered` is false when an occluding hitbox (popup menu,
                            // modal, ...) is above the cursor, so the tooltip clears instead
                            // of tracking the mouse through the overlay.
                            let next = if hitbox.is_hovered(window) {
                                Some(e.position - bounds.origin)
                            } else {
                                None
                            };

                            if cell.get() != next {
                                cell.set(next);
                                window.refresh();
                            }
                        },
                    );
                }

                // Paint the tooltip overlay (crosshair, dots) above the plot graphics; the
                // deferred box paints later, above everything.
                if let Some(overlay) = overlay.as_mut() {
                    overlay.paint(window, cx);
                }
            }
        }
    };

    TokenStream::from(expanded)
}
