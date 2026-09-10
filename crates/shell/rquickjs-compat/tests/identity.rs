use rquickjs::{Context, Object, Runtime};

#[test]
fn facade_and_distribution_share_values_and_runtime() {
    let runtime: upstream::Runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let object: upstream::Object = Object::new(ctx.clone()).unwrap();
        object.set("answer", 42).unwrap();
        ctx.globals().set("shared", object).unwrap();
        assert_eq!(ctx.eval::<i32, _>("shared.answer").unwrap(), 42);
    });
}

#[cfg(feature = "macro")]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct Answer {
    #[qjs(get)]
    value: i32,
}

#[cfg(feature = "macro")]
#[test]
fn reexported_macros_use_the_same_class_types() {
    let runtime = upstream::Runtime::new().unwrap();
    let context = upstream::Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let answer: upstream::Class<Answer> =
            rquickjs::Class::instance(ctx.clone(), Answer { value: 42 }).unwrap();
        ctx.globals().set("answer", answer).unwrap();
        assert_eq!(ctx.eval::<i32, _>("answer.value").unwrap(), 42);
    });
}
