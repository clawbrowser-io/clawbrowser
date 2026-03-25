pub mod console;
pub mod dom;
pub mod fetch;
pub mod timer;
pub mod window;

use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use rquickjs::{Ctx, Result};

use self::dom::SharedDoc;
use self::timer::SharedTimerQueue;

pub type InflightCounter = Arc<AtomicU32>;

pub fn install_all(
    ctx: &Ctx<'_>,
    doc: SharedDoc,
    timer_queue: SharedTimerQueue,
    http_client: Arc<reqwest::Client>,
    url: &str,
) -> Result<InflightCounter> {
    let inflight = Arc::new(AtomicU32::new(0));
    console::install(ctx)?;
    window::install(ctx, url)?;
    dom::install(ctx, doc)?;
    timer::install(ctx, timer_queue)?;
    fetch::install(ctx, http_client, inflight.clone())?;
    install_dom_shim(ctx)?;
    Ok(inflight)
}

fn install_dom_shim(ctx: &Ctx<'_>) -> Result<()> {
    ctx.eval::<(), _>(DOM_SHIM_JS)?;
    Ok(())
}

const DOM_SHIM_JS: &str = include_str!("dom_shim.js");
