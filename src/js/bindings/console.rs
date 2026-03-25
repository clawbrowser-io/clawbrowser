use rquickjs::{Ctx, Function, Object, Result, Value};
use rquickjs::prelude::Rest;
use tracing::{info, warn, error, debug};

pub fn install(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let console = Object::new(ctx.clone())?;

    console.set("log", Function::new(ctx.clone(), js_log)?)?;
    console.set("info", Function::new(ctx.clone(), js_log)?)?;
    console.set("warn", Function::new(ctx.clone(), js_warn)?)?;
    console.set("error", Function::new(ctx.clone(), js_error)?)?;
    console.set("debug", Function::new(ctx.clone(), js_debug)?)?;

    globals.set("console", console)?;
    Ok(())
}

fn js_log(args: Rest<Value<'_>>) {
    let msg = format_args_to_string(&args.0);
    info!(target: "js:console", "{msg}");
}

fn js_warn(args: Rest<Value<'_>>) {
    let msg = format_args_to_string(&args.0);
    warn!(target: "js:console", "{msg}");
}

fn js_error(args: Rest<Value<'_>>) {
    let msg = format_args_to_string(&args.0);
    error!(target: "js:console", "{msg}");
}

fn js_debug(args: Rest<Value<'_>>) {
    let msg = format_args_to_string(&args.0);
    debug!(target: "js:console", "{msg}");
}

fn format_args_to_string(args: &[Value<'_>]) -> String {
    args.iter()
        .map(|v| value_to_string(v))
        .collect::<Vec<_>>()
        .join(" ")
}

fn value_to_string(val: &Value<'_>) -> String {
    if let Some(s) = val.as_string() {
        s.to_string().unwrap_or_else(|_| "[string]".into())
    } else if val.is_undefined() {
        "undefined".into()
    } else if val.is_null() {
        "null".into()
    } else if let Some(b) = val.as_bool() {
        b.to_string()
    } else if let Some(n) = val.as_int() {
        n.to_string()
    } else if let Some(n) = val.as_float() {
        n.to_string()
    } else {
        "[object]".into()
    }
}
