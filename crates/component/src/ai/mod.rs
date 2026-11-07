//! Elements for building agent and chat interfaces: reasoning, tool calls,
//! plans, approvals, sources and the prompt composer.
//!
//! These follow the patterns of
//! [AI Elements](https://ui.shadcn.com/docs/ai-elements) and compose the
//! existing [`message`](crate::message) and
//! [`message_scroller`](crate::message_scroller) modules for the
//! conversation itself. Every element is a controlled `RenderOnce` value: the
//! application owns the transcript, streaming state and open states, and
//! passes them in on each render.

mod agent_task;
mod chain_of_thought;
mod checkpoint;
mod confirmation;
mod context_usage;
mod file_tree;
mod model_selector;
mod plan;
mod prompt_input;
mod queue;
mod reasoning;
mod sources;
mod stack_trace;
mod subagent;
mod suggestion;
mod terminal;
mod test_results;
mod tool;
mod trace;

pub use agent_task::*;
pub use chain_of_thought::*;
pub use checkpoint::*;
pub use confirmation::*;
pub use context_usage::*;
pub use file_tree::*;
pub use model_selector::*;
pub use plan::{Plan, PlanStep};
pub use prompt_input::*;
pub use queue::*;
pub use reasoning::*;
pub use sources::*;
pub use stack_trace::*;
pub use subagent::*;
pub use suggestion::*;
pub use terminal::*;
pub use test_results::*;
pub use tool::*;
pub use trace::{Trace, TraceStatus};
