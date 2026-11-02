use rquickjs::{
    Ctx, Object, Result, Value,
    module::{Declarations, Exports, ModuleDef},
};

const EXPORTS: &[&str] = &["run", "spawn", "exit", "nextTick", "platform", "arch"];

pub(super) struct ProcessModule;

impl ModuleDef for ProcessModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("default")?;
        for name in EXPORTS {
            declarations.declare(*name)?;
        }
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let process: Object = ctx.globals().get("process")?;
        exports.export("default", Value::from_object(process.clone()))?;
        for name in EXPORTS {
            let value: Value = process.get(*name)?;
            exports.export(*name, value)?;
        }
        Ok(())
    }
}
