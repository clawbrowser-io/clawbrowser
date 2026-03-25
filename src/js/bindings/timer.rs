use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rquickjs::{Ctx, Function, Persistent, Result, Value};

struct TimerEntry {
    id: u32,
    callback: Persistent<Function<'static>>,
    fire_at: Instant,
    interval: Option<Duration>,
}

// SAFETY: Persistent values are only accessed within the same JS runtime context.
// TimerQueue is always used behind Arc<Mutex<>> and only accessed within
// the same tokio task that owns the corresponding AsyncRuntime.
unsafe impl Send for TimerEntry {}

impl PartialEq for TimerEntry {
    fn eq(&self, other: &Self) -> bool {
        self.fire_at == other.fire_at
    }
}
impl Eq for TimerEntry {}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TimerEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.fire_at.cmp(&other.fire_at)
    }
}

pub struct TimerQueue {
    timers: BinaryHeap<Reverse<TimerEntry>>,
    next_id: u32,
    cleared_ids: Vec<u32>,
}

impl TimerQueue {
    pub fn new() -> Self {
        Self {
            timers: BinaryHeap::new(),
            next_id: 1,
            cleared_ids: Vec::new(),
        }
    }

    pub fn add_timeout(&mut self, callback: Persistent<Function<'static>>, delay_ms: u64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.timers.push(Reverse(TimerEntry {
            id,
            callback,
            fire_at: Instant::now() + Duration::from_millis(delay_ms),
            interval: None,
        }));
        id
    }

    pub fn add_interval(&mut self, callback: Persistent<Function<'static>>, delay_ms: u64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let dur = Duration::from_millis(delay_ms.max(1));
        self.timers.push(Reverse(TimerEntry {
            id,
            callback,
            fire_at: Instant::now() + dur,
            interval: Some(dur),
        }));
        id
    }

    pub fn clear(&mut self, id: u32) {
        self.cleared_ids.push(id);
    }

    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    pub fn fire_ready(&mut self, ctx: &Ctx<'_>) -> Result<bool> {
        let now = Instant::now();
        let mut fired = false;
        let mut re_add = Vec::new();

        while let Some(Reverse(entry)) = self.timers.peek() {
            if entry.fire_at > now {
                break;
            }
            let entry = self.timers.pop().unwrap().0;

            if self.cleared_ids.contains(&entry.id) {
                continue;
            }

            let cb = entry.callback.clone().restore(ctx)?;
            let _ = cb.call::<_, Value>(()); 
            fired = true;

            if let Some(dur) = entry.interval {
                re_add.push(TimerEntry {
                    id: entry.id,
                    callback: entry.callback,
                    fire_at: Instant::now() + dur,
                    interval: Some(dur),
                });
            }
        }

        for entry in re_add {
            self.timers.push(Reverse(entry));
        }

        self.cleared_ids.clear();
        Ok(fired)
    }

    pub fn drain_all(&mut self, ctx: &Ctx<'_>) {
        while let Some(Reverse(entry)) = self.timers.pop() {
            let _ = entry.callback.restore(ctx);
        }
        self.cleared_ids.clear();
    }

    pub fn next_fire_time(&self) -> Option<Instant> {
        self.timers.peek().map(|Reverse(e)| e.fire_at)
    }
}

pub type SharedTimerQueue = Arc<Mutex<TimerQueue>>;

pub fn install(ctx: &Ctx<'_>, timer_queue: SharedTimerQueue) -> Result<()> {
    let globals = ctx.globals();

    let tq = timer_queue.clone();
    globals.set("setTimeout", Function::new(ctx.clone(), move |cb: Persistent<Function<'static>>, ms: Option<i32>| -> u32 {
        let delay = ms.unwrap_or(0).max(0) as u64;
        tq.lock().unwrap().add_timeout(cb, delay)
    })?)?;

    let tq = timer_queue.clone();
    globals.set("setInterval", Function::new(ctx.clone(), move |cb: Persistent<Function<'static>>, ms: Option<i32>| -> u32 {
        let delay = ms.unwrap_or(16).max(1) as u64;
        tq.lock().unwrap().add_interval(cb, delay)
    })?)?;

    let tq = timer_queue.clone();
    globals.set("clearTimeout", Function::new(ctx.clone(), move |id: u32| {
        tq.lock().unwrap().clear(id);
    })?)?;

    let tq = timer_queue.clone();
    globals.set("clearInterval", Function::new(ctx.clone(), move |id: u32| {
        tq.lock().unwrap().clear(id);
    })?)?;

    let tq = timer_queue.clone();
    globals.set("requestAnimationFrame", Function::new(ctx.clone(), move |cb: Persistent<Function<'static>>| -> u32 {
        tq.lock().unwrap().add_timeout(cb, 16)
    })?)?;

    let tq = timer_queue.clone();
    globals.set("cancelAnimationFrame", Function::new(ctx.clone(), move |id: u32| {
        tq.lock().unwrap().clear(id);
    })?)?;

    Ok(())
}
