// Preserve the published #[gpui_kit::test] API.
// This package has its own workspace and depends only on gpui-kit. Running the
// same contract here prevents Kit's direct GPUI dependency from hiding a broken
// re-export or macro expansion in consumers.
#[path = "../../../crates/kit/tests/test_macro.rs"]
mod test_macro;

// The complete example included by docs/test must work with the same imports.
#[path = "../../../crates/kit/tests/ui.rs"]
mod ui;
