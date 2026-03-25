use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use rquickjs::AsyncContext;
use tracing::debug;

use super::bindings::timer::TimerQueue;

pub async fn run_event_loop(
    ctx: &AsyncContext,
    timer_queue: Arc<Mutex<TimerQueue>>,
    timeout: Duration,
) -> Result<()> {
    let start = Instant::now();
    let mut idle_count = 0u32;

    loop {
        if start.elapsed() > timeout {
            debug!("event loop timeout reached");
            break;
        }

        let fired = ctx.with(|ctx| {
            let mut tq = timer_queue.lock().unwrap();
            tq.fire_ready(&ctx).unwrap_or(false)
        }).await;

        let has_timers = !timer_queue.lock().unwrap().is_empty();

        if !fired && !has_timers {
            idle_count += 1;
            if idle_count > 5 {
                debug!("event loop idle, finishing");
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        } else {
            idle_count = 0;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    Ok(())
}
