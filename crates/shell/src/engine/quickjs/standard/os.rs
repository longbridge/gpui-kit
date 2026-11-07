use crate::engine::quickjs::js_error;
use rquickjs::{
    Ctx, Object, Result, Value,
    function::Func,
    module::{Declarations, Exports, ModuleDef},
};

const EXPORTS: &[&str] = &["platform", "arch", "EOL", "homedir"];

pub(super) struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("default")?;
        for name in EXPORTS {
            declarations.declare(*name)?;
        }
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let os = Object::new(ctx.clone())?;
        os.set("platform", Func::from(|| std::env::consts::OS))?;
        os.set("arch", Func::from(|| std::env::consts::ARCH))?;
        os.set("EOL", if cfg!(windows) { "\r\n" } else { "\n" })?;
        // Needs no grant: the path names where the user's files are, not a way
        // to reach them, and `${homeDir}` in the manifest is what grants that.
        os.set(
            "homedir",
            Func::from(|ctx: Ctx<'_>| -> Result<String> {
                crate::runtime::home_dir()
                    .map(|home| home.to_string_lossy().into_owned())
                    .ok_or_else(|| {
                        js_error::error(
                            &ctx,
                            "os.homedir(): neither HOME nor USERPROFILE names an absolute directory",
                        )
                    })
            }),
        )?;
        exports.export("default", Value::from_object(os.clone()))?;
        for name in EXPORTS {
            exports.export(*name, os.get::<_, Value>(*name)?)?;
        }
        Ok(())
    }
}
