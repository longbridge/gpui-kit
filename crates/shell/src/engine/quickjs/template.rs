//! A description recorded once, and filled per call.
//!
//! §20.7 of `docs/gpui-shell.md` is the argument; this is the implementation.
//! A snapshot removes the cost of *no* change (§8.4). It does nothing for a
//! *small* one: when a price moves, the structure around it is identical and
//! the whole view is described again anyway — every `div()`, every `.gap()`,
//! every crossing.
//!
//! A template splits that description in two:
//!
//! ```text
//! first call   body(sentinel, sentinel, …)  ──▶  structure  +  slot list
//! every call   graft the structure, write the slots          (no script runs)
//! ```
//!
//! # How the slots are found
//!
//! Not by a compiler, and not by reaching into the engine for a call-site key —
//! §5.3 refuses the first and §6.5 refuses the second. The body is run **once**
//! with a sentinel object in each parameter position, and wherever a sentinel
//! comes to rest in the recorded description is a slot. What is left over is
//! structure. Three positions can hold one, which is the whole of
//! [`SlotSite`](crate::spec::SlotSite):
//!
//! ```text
//! .child(price)        ──▶  the string a Text node carries
//! .bg(color)           ──▶  one argument of a recorded style
//! .on_click(select)    ──▶  the CallbackId of a recorded handler
//! ```
//!
//! # The two rules this enforces rather than documents
//!
//! **An argument may be passed through, not computed on.** `` `${price}` ``
//! would consume the sentinel during discovery and bake a constant into the
//! structure, which is a panel that silently stops updating. So the sentinel
//! refuses to become a primitive, in the prelude, and the mistake is a
//! diagnostic at first use.
//!
//! **A handler must arrive as an argument.** A closure written inside a body is
//! created once and would capture that first call's values for as long as the
//! template lived. A body that registers one is refused at definition.
//!
//! Two smaller ones follow from the same place: a body has no conditionals
//! (it runs once, so a branch would freeze — a loading state and a content
//! state are two templates), and a body may not mount a retained entity, since
//! GPUI cannot mount one entity at two positions in a tree and a template is
//! grafted many times.
//!
//! # What this does not make free
//!
//! Handlers. A slot in a handler position still allocates a closure per call
//! and registers a callback per call, which on a panel with a button in every
//! row is a third of what filling costs. That is measured rather than
//! estimated — §20.7 carries the number.

use rquickjs::{Coerced, Ctx, Exception, Function, Persistent, Result as JsResult, Value};
use smallvec::SmallVec;

use super::{CallbackEntry, ShellRuntime};
use crate::{
    runtime::ApplicationGeneration,
    scope,
    spec::{CallbackId, Component, Slot, SlotSite, SlotValue, SpecArena, SpecId, SpecOp, Template},
    style,
    value::Bridged,
};

/// The property a sentinel carries, and the only thing that identifies one.
pub(super) const SENTINEL: &str = "__slot";

/// The `CallbackId` a handler slot holds until a call fills it.
///
/// A value no arena ever mints, so a template that reached definition holding a
/// *real* callback is a body that registered an inline handler — which is what
/// [`ShellRuntime::end_template`] refuses. Distinguishing the two by a
/// placeholder rather than by position keeps the check to one pass and cannot
/// be fooled by a template whose first callback happened to be id zero.
const UNFILLED: CallbackId = CallbackId::MAX;

/// The template being discovered, and the description it interrupted.
pub(super) struct Discovery {
    arity: usize,
    slots: Vec<Slot>,
    /// The arena the live render was recording into when the body started.
    ///
    /// Swapped out rather than recorded around, so a template's ids are dense
    /// and start at zero — which is what makes grafting one addition per id.
    saved: SpecArena,
}

impl ShellRuntime {
    /// Starts recording a template body.
    ///
    /// Nesting is refused rather than supported. A body that defines or calls
    /// another template would have to thread the outer sentinels through the
    /// inner template's slots, and there is no evidence yet that anyone wants
    /// to: refusing it now costs a message, and allowing it later costs
    /// nothing that has been promised.
    pub(super) fn begin_template(&self, ctx: &Ctx<'_>, arity: usize) -> JsResult<()> {
        if self.discovery.borrow().is_some() {
            return Err(Exception::throw_type(
                ctx,
                "a template body cannot define or call another template. Build the \
                 inner structure inline, or call the inner template where the outer \
                 one is called",
            ));
        }

        let saved = std::mem::take(&mut *self.arena.borrow_mut());
        *self.discovery.borrow_mut() = Some(Discovery {
            arity,
            slots: Vec::new(),
            saved,
        });
        Ok(())
    }

    /// Abandons a body that threw, and puts the interrupted description back.
    pub(super) fn abort_template(&self) {
        if let Some(discovery) = self.discovery.borrow_mut().take() {
            *self.arena.borrow_mut() = discovery.saved;
        }
    }

    /// Finishes a body and answers the id its closure will keep.
    pub(super) fn end_template(&self, ctx: &Ctx<'_>, root: Option<SpecId>) -> JsResult<u32> {
        let Some(discovery) = self.discovery.borrow_mut().take() else {
            return Err(Exception::throw_type(ctx, "no template is being defined"));
        };

        // The interrupted description goes back before any refusal below, or a
        // rejected template would take the render with it.
        let recorded = std::mem::replace(&mut *self.arena.borrow_mut(), discovery.saved);

        let root = root
            .filter(|root| (*root as usize) < recorded.len())
            .ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "a template body must return one element built inside it",
                )
            })?;

        if recorded.mounts_an_entity() {
            return Err(Exception::throw_type(
                ctx,
                "a template cannot mount a nested view or a dock area: it is grafted \
                 once per call, and GPUI mounts one entity at one place. Put the entity \
                 where the template is called",
            ));
        }

        if let Some(method) = inline_handler(&recorded, &discovery.slots) {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "`{method}` inside a template body registers one handler for the life \
                     of the template, which would capture this first call's values forever. \
                     Take the handler as a parameter and pass it in"
                ),
            ));
        }

        if let Some(argument) = unused_argument(discovery.arity, &discovery.slots) {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "template argument {argument} is never used in the body. A parameter \
                     that reaches no builder call fills nothing, which is usually a value \
                     that was formatted or compared instead of passed through"
                ),
            ));
        }

        let mut templates = self.templates.borrow_mut();
        templates.push(Some(std::rc::Rc::new(Template::new(
            recorded,
            root,
            discovery.slots,
            discovery.arity,
            scope::current_application_generation(),
        ))));
        Ok((templates.len() - 1) as u32)
    }

    /// Grafts a template into the description being recorded and writes this
    /// call's arguments into its slots.
    ///
    /// The whole of an instantiation: no builder method is interpreted, nothing
    /// crosses the bridge except the arguments themselves, and the structure is
    /// copied rather than described.
    pub(super) fn instantiate_template<'js>(
        &self,
        ctx: &Ctx<'js>,
        id: u32,
        arguments: Vec<Value<'js>>,
    ) -> JsResult<SpecId> {
        if self.discovery.borrow().is_some() {
            return Err(Exception::throw_type(
                ctx,
                "a template body cannot call another template. Call it where the outer \
                 template is called",
            ));
        }

        let template = self
            .templates
            .borrow()
            .get(id as usize)
            .cloned()
            .flatten()
            .ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "this template belongs to an application that has been unloaded",
                )
            })?;

        if arguments.len() != template.arity() {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "this template takes {} argument(s) and was given {}",
                    template.arity(),
                    arguments.len()
                ),
            ));
        }

        // Every value is converted and checked before the graft, so a bad
        // argument leaves the description it was being added to untouched
        // rather than half-grown.
        let mut values = Vec::with_capacity(template.slots().len());
        for slot in template.slots() {
            let argument = &arguments[slot.argument() as usize];
            values.push(self.slot_value(ctx, &template, slot, argument)?);
        }

        let root = self.arena.borrow_mut().graft(&template);
        let base = root - template.root();

        let mut arena = self.arena.borrow_mut();
        for (slot, value) in template.slots().iter().zip(values) {
            arena
                .write_slot(base, slot, value)
                .map_err(|error| Exception::throw_type(ctx, &error.to_string()))?;
        }

        Ok(root)
    }

    /// Converts one argument for the position it fills, and checks it there.
    fn slot_value<'js>(
        &self,
        ctx: &Ctx<'js>,
        template: &Template,
        slot: &Slot,
        argument: &Value<'js>,
    ) -> JsResult<SlotValue> {
        match slot.site() {
            SlotSite::Text => Ok(SlotValue::Text(text_of(ctx, argument)?)),
            SlotSite::Argument {
                op,
                argument: index,
            } => {
                let value = super::bridge(ctx, argument)?;
                // The check the ordinary path runs while recording. It could
                // not run then — the value did not exist — so it runs here,
                // which still reports one call rather than one frame later.
                if let Some(SpecOp::ParamStyle(name, existing)) = template
                    .arena()
                    .node(slot.node())
                    .map(crate::spec::SpecNode::ops)
                    .and_then(|ops| ops.get(op as usize))
                {
                    let mut checked: SmallVec<[Bridged; 2]> = existing.clone();
                    if let Some(place) = checked.get_mut(index as usize) {
                        *place = value.clone();
                    }
                    style::apply_param(name, &checked, Default::default())
                        .map_err(|error| Exception::throw_type(ctx, error.message()))?;
                }
                Ok(SlotValue::Value(value))
            }
            SlotSite::Handler { .. } => {
                let handler = argument.as_function().ok_or_else(|| {
                    Exception::throw_type(
                        ctx,
                        "this template argument fills a handler and must be a function",
                    )
                })?;
                Ok(SlotValue::Handler(
                    self.register_handler(Persistent::save(ctx, handler.clone())),
                ))
            }
        }
    }

    /// Records the text node a `.child(argument)` inside a body describes.
    pub(super) fn text_slot(&self, ctx: &Ctx<'_>, argument: u16) -> JsResult<SpecId> {
        self.require_discovery(ctx)?;
        let node = self.arena.borrow_mut().push(Component::Text(String::new()));
        self.note_slot(ctx, Slot::new(node, SlotSite::Text, argument))?;
        Ok(node)
    }

    /// Records a sentinel that arrived at a behaviour method.
    ///
    /// Handlers are the only such position a template fills today. Everything
    /// else is refused where it was written, naming what to do instead, rather
    /// than baked in as a constant that would never change again.
    pub(super) fn apply_slot(
        &self,
        ctx: &Ctx<'_>,
        id: SpecId,
        method: &str,
        position: usize,
        argument: u16,
    ) -> JsResult<()> {
        self.require_discovery(ctx)?;

        let Some(name) = super::callback_op_name(method).filter(|_| position == 0) else {
            return Err(Exception::throw_type(
                ctx,
                &format!(
                    "`{method}` cannot take a template argument: a template fills text \
                     children, style arguments and handlers. Compute the value where the \
                     template is called and pass the result"
                ),
            ));
        };

        self.push_op_checked(ctx, self.push_op(id, SpecOp::Callback(name, UNFILLED)))?;
        let op = self.last_op_index(ctx, id)?;
        self.note_slot(ctx, Slot::new(id, SlotSite::Handler { op }, argument))
    }

    /// Notes that the operation just pushed onto `id` carries a slot.
    pub(super) fn record_slot_at_last_op(
        &self,
        ctx: &Ctx<'_>,
        id: SpecId,
        argument_in_op: u8,
        argument: u16,
    ) -> JsResult<()> {
        self.require_discovery(ctx)?;
        let op = self.last_op_index(ctx, id)?;
        self.note_slot(
            ctx,
            Slot::new(
                id,
                SlotSite::Argument {
                    op,
                    argument: argument_in_op,
                },
                argument,
            ),
        )
    }

    /// Registers one script function and answers the id a description holds it
    /// by.
    ///
    /// The callback belongs to the view and the snapshot generation that are
    /// current now, which is what retires it when that description is replaced.
    fn register_handler(&self, handler: Persistent<Function<'static>>) -> CallbackId {
        self.callbacks.borrow_mut().push(CallbackEntry {
            value: handler,
            view: scope::current_view().map(|view| view.downgrade()),
            application: scope::current_application_generation(),
            registered_in: scope::current_generation(),
        })
    }

    /// Drops every template an application defined, when that application is
    /// released.
    ///
    /// The slot is emptied rather than removed, because a template's id is its
    /// index and a closure in a still-loaded module may hold one. Emptying frees
    /// the arena, which is all of the memory; what is left is one `None` per
    /// template ever defined, and a script that reaches a retired id is told so
    /// rather than handed someone else's structure.
    pub(super) fn retire_templates(&self, application: &std::rc::Rc<ApplicationGeneration>) {
        let mut templates = self.templates.borrow_mut();
        for slot in templates.iter_mut() {
            if slot
                .as_ref()
                .is_some_and(|template| template.belongs_to(application))
            {
                *slot = None;
            }
        }
    }

    /// How many templates are holding memory. Read by this module's tests.
    #[cfg(test)]
    fn live_template_count(&self) -> usize {
        self.templates.borrow().iter().flatten().count()
    }

    fn require_discovery(&self, ctx: &Ctx<'_>) -> JsResult<()> {
        if self.discovery.borrow().is_some() {
            return Ok(());
        }
        Err(Exception::throw_type(
            ctx,
            "a template argument escaped the body that declared it. It can be passed to a \
             builder call inside the template, and nowhere else",
        ))
    }

    fn last_op_index(&self, ctx: &Ctx<'_>, id: SpecId) -> JsResult<u16> {
        let count = self
            .arena
            .borrow()
            .node(id)
            .map(|node| node.ops().len())
            .filter(|count| *count > 0)
            .ok_or_else(|| Exception::throw_type(ctx, "no operation to attach a slot to"))?;
        u16::try_from(count - 1).map_err(|_| {
            Exception::throw_type(
                ctx,
                "one element recorded more operations than a template can address",
            )
        })
    }

    fn note_slot(&self, ctx: &Ctx<'_>, slot: Slot) -> JsResult<()> {
        match self.discovery.borrow_mut().as_mut() {
            Some(discovery) => {
                discovery.slots.push(slot);
                Ok(())
            }
            None => Err(Exception::throw_type(ctx, "no template is being defined")),
        }
    }
}

/// The first handler a body registered itself, if it registered one.
///
/// A slot-filled handler carries [`UNFILLED`] and a matching note; anything else
/// is a closure the body created, which a template cannot hold.
fn inline_handler(arena: &SpecArena, slots: &[Slot]) -> Option<&'static str> {
    let filled = |node: SpecId, op: u16| {
        slots
            .iter()
            .any(|slot| slot.node() == node && slot.site() == SlotSite::Handler { op })
    };

    for id in 0..arena.len() as SpecId {
        let Some(node) = arena.node(id) else { continue };
        for (index, op) in node.ops().iter().enumerate() {
            let index = index as u16;
            match op {
                SpecOp::Callback(name, callback) => {
                    if *callback != UNFILLED || !filled(id, index) {
                        return Some(name);
                    }
                }
                // An action's name is the script's own and is not a slot, so a
                // template can never hold one.
                SpecOp::ActionCallback(..) => return Some("on_action"),
                _ => {}
            }
        }
    }

    None
}

/// The first declared parameter that reached no builder call.
fn unused_argument(arity: usize, slots: &[Slot]) -> Option<usize> {
    (0..arity).find(|argument| {
        !slots
            .iter()
            .any(|slot| slot.argument() as usize == *argument)
    })
}

/// What a text slot writes, from the same three types `.child(value)` accepts.
///
/// Validated through the bridge first, so that an object or a function is
/// refused rather than stringified into `"[object Object]"`, and coerced after
/// — because the ordinary path is `.child(String(value))` in the prelude, and a
/// number has to read the same whichever path recorded it.
fn text_of<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> JsResult<String> {
    if matches!(super::bridge(ctx, value)?, Bridged::Nil) {
        return Err(Exception::throw_type(
            ctx,
            "this template argument fills a text child and must be a string, a number \
             or a boolean",
        ));
    }
    Ok(value.clone().get::<Coerced<String>>()?.0)
}

#[cfg(test)]
mod tests {
    use std::{ops::Deref, rc::Rc};

    use gpui::{TestAppContext, VisualTestContext};

    use crate::{ShellRuntime, runtime::ApplicationGeneration};

    /// A template is defined once and used for the life of the module, so
    /// nothing in a render would ever free one. What frees them is the release
    /// that already retires an application's callbacks and tasks — and it has
    /// to, because a hot reload re-evaluates the module and defines every
    /// template in it again.
    #[gpui::test]
    fn releasing_an_application_drops_the_templates_it_defined(cx: &mut TestAppContext) {
        struct Host;

        impl gpui::Render for Host {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl gpui::IntoElement {
                gpui::div()
            }
        }

        const SOURCE: &str = r#"
import { View, div } from "gpui-kit";
const template = globalThis.__template;
const Row = template((value) => div().child(value));
export default class Board extends View {
  render() { return div().child(Row("one")).child(Row("two")); }
}
"#;

        cx.update(|cx| crate::init(cx));
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));

        let application = ApplicationGeneration::new(41);
        let mut view_type = runtime.load_source("templates.js", SOURCE).expect("load");
        view_type.application = Some(Rc::clone(&application));

        let window = cx.add_window(|_, _| Host);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let object = context.update(|window, cx| {
            runtime
                .instantiate(&view_type, window, cx)
                .expect("instantiate")
        });

        context.update(|window, cx| {
            runtime
                .build_snapshot(&object, None, crate::policy::default(), window, cx)
                .expect("render");
        });
        assert_eq!(
            runtime.live_template_count(),
            1,
            "the render should have defined the row template"
        );

        context.update(|_, _| runtime.release_application_generation_without_context(&application));
        assert_eq!(
            runtime.live_template_count(),
            0,
            "releasing the application must free what its templates were holding"
        );
    }
}
