use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use rquickjs::CatchResultExt;
use tracing::{debug, info, warn};

use crate::config::FetchConfig;
use crate::dom::{serialize_inner_html, Document};
use crate::html::parse_html;
use crate::http::HttpClient;
use crate::js::bindings::timer::TimerQueue;
use crate::js::script_loader::{find_scripts, load_script};
use crate::js::JsRuntime;
use crate::output::{cleanup_intrusive_overlays, normalize_protocol_relative_urls, promote_href_elements, html_to_markdown};

const MAX_SETTLE_TIMER_DELAY: Duration = Duration::from_millis(100);

pub struct Page {
    pub document: Document,
    pub url: String,
}

impl Page {
    pub async fn navigate(url: &str, config: &FetchConfig) -> Result<Self> {
        info!(url, "fetching page");

        let client = HttpClient::new(config)?;
        let (html, final_url) = client.fetch_text(url).await?;

        debug!(
            final_url = final_url.as_str(),
            html_len = html.len(),
            "received HTML"
        );

        let document = parse_html(&html, Some(final_url.clone()));
        debug!(node_count = document.arena.len(), "DOM tree built");

        if config.no_js {
            return Ok(Self {
                document,
                url: final_url,
            });
        }

        let js_timeout = Duration::from_secs(config.timeout_secs + 5);
        let mut document = match tokio::time::timeout(
            js_timeout,
            Self::run_js(document, &final_url, config, client.inner()),
        )
        .await
        {
            Ok(Ok(doc)) => doc,
            Ok(Err(e)) => {
                warn!(error = %e, "JS execution failed, returning static DOM");
                parse_html(&html, Some(final_url.clone()))
            }
            Err(_) => {
                warn!("JS execution hard timeout ({}s), returning static DOM", js_timeout.as_secs());
                parse_html(&html, Some(final_url.clone()))
            }
        };

        let removed = cleanup_intrusive_overlays(&mut document);
        if removed > 0 {
            debug!(removed, "removed intrusive overlay nodes");
        }

        let promoted = promote_href_elements(&mut document);
        if promoted > 0 {
            debug!(promoted, "promoted non-anchor elements with href to <a>");
        }

        normalize_protocol_relative_urls(&mut document);

        Ok(Self {
            document,
            url: final_url,
        })
    }

    async fn run_js(
        document: Document,
        url: &str,
        config: &FetchConfig,
        http_client: &reqwest::Client,
    ) -> Result<Document> {
        let scripts = find_scripts(&document);
        if scripts.is_empty() {
            debug!("no scripts found, skipping JS execution");
            return Ok(document);
        }

        debug!(count = scripts.len(), "found scripts");

        let shared_doc = Arc::new(Mutex::new(document));
        let timer_queue = Arc::new(Mutex::new(TimerQueue::new()));
        let http = Arc::new(http_client.clone());

        let js_rt = JsRuntime::new().await?;

        let should_interrupt = Arc::new(AtomicBool::new(false));
        let flag = should_interrupt.clone();
        js_rt.rt.set_interrupt_handler(Some(Box::new(move || {
            flag.load(Ordering::Relaxed)
        }))).await;

        let url_owned = url.to_string();
        let sd = shared_doc.clone();
        let tq = timer_queue.clone();
        let h = http.clone();

        let inflight = js_rt
            .ctx
            .with(|ctx| {
                crate::js::bindings::install_all(&ctx, sd, tq, h, &url_owned)
                    .map_err(|e| anyhow::anyhow!("failed to install bindings: {e:?}"))
            })
            .await?;

        let overall_deadline = Instant::now() + Duration::from_secs(config.timeout_secs);
        let script_start = Instant::now();
        let mut scripts_executed = 0u32;
        let mut scripts_failed = 0u32;

        for (i, script) in scripts.iter().enumerate() {
            if Instant::now() > overall_deadline {
                warn!("overall timeout reached, skipping remaining scripts");
                break;
            }

            let code = if let Some(src) = &script.src {
                let load_timeout = Duration::from_secs(10)
                    .min(overall_deadline.saturating_duration_since(Instant::now()));
                match tokio::time::timeout(load_timeout, load_script(http_client, url, src)).await {
                    Ok(Ok(code)) if !code.is_empty() => code,
                    Ok(Ok(_)) => continue,
                    Ok(Err(e)) => {
                        warn!(src = src.as_str(), error = %e, "failed to load script");
                        scripts_failed += 1;
                        continue;
                    }
                    Err(_) => {
                        warn!(src = src.as_str(), "script load timed out");
                        scripts_failed += 1;
                        continue;
                    }
                }
            } else if let Some(inline) = &script.inline_code {
                inline.clone()
            } else {
                continue;
            };

            let filename = script
                .src
                .clone()
                .unwrap_or_else(|| format!("inline-{i}.js"));

            debug!(filename = filename.as_str(), len = code.len(), "executing script");

            let nid = script.node_id.0;
            let set_current = format!("document.__setCurrentScript({nid})");
            js_rt.ctx.with(|ctx| {
                let _: rquickjs::Result<()> = ctx.eval(set_current.as_bytes());
            }).await;

            let interrupt_flag = should_interrupt.clone();
            let eval_deadline = Instant::now() + Duration::from_secs(5);
            let restore_flag = interrupt_flag.clone();

            let code_owned = code.clone();
            let fname = filename.clone();

            interrupt_flag.store(false, Ordering::Relaxed);

            let timer_handle = {
                let flag = interrupt_flag.clone();
                tokio::spawn(async move {
                    let wait = eval_deadline.saturating_duration_since(Instant::now());
                    tokio::time::sleep(wait).await;
                    flag.store(true, Ordering::Relaxed);
                })
            };

            let result = js_rt
                .ctx
                .with(|ctx| {
                    let r: rquickjs::Result<()> = ctx.eval(code_owned.as_bytes());
                    r.catch(&ctx).map_err(|e| anyhow::anyhow!("JS error in {}: {e:?}", fname))
                })
                .await;

            timer_handle.abort();
            restore_flag.store(false, Ordering::Relaxed);

            js_rt.ctx.with(|ctx| {
                let _: rquickjs::Result<()> = ctx.eval(b"document.__setCurrentScript(null)");
            }).await;

            match result {
                Ok(()) => scripts_executed += 1,
                Err(e) => {
                    warn!(filename = filename.as_str(), error = %e, "script execution error");
                    scripts_failed += 1;
                }
            }

            let _ = js_rt.rt.execute_pending_job().await;
        }

        debug!(
            executed = scripts_executed,
            failed = scripts_failed,
            elapsed_ms = script_start.elapsed().as_millis() as u64,
            "script execution summary"
        );

        debug!("firing DOMContentLoaded");
        js_rt.ctx.with(|ctx| {
            let _: rquickjs::Result<()> = ctx.eval("try{__fireDOMContentLoaded()}catch(e){}");
        }).await;
        debug!("DOMContentLoaded fired, executing pending jobs");
        let _ = js_rt.rt.execute_pending_job().await;

        debug!("firing load");
        js_rt.ctx.with(|ctx| {
            let _: rquickjs::Result<()> = ctx.eval("try{__fireLoad()}catch(e){}");
        }).await;
        debug!("load fired, executing pending jobs");
        let _ = js_rt.rt.execute_pending_job().await;
        debug!("entering event loop");

        let event_loop_timeout = Duration::from_millis(config.wait_ms);
        let event_loop_deadline = Instant::now() + event_loop_timeout;
        let hard_deadline = overall_deadline;
        let tq_for_loop = timer_queue.clone();
        let event_loop_start = Instant::now();
        let mut idle_ticks = 0u32;
        let mut loop_iter = 0u32;

        {
            let deadline = event_loop_deadline.min(hard_deadline);
            let flag = should_interrupt.clone();
            tokio::spawn(async move {
                let wait = deadline.saturating_duration_since(Instant::now());
                tokio::time::sleep(wait).await;
                flag.store(true, Ordering::Relaxed);
            });
        }

        loop {
            let now = Instant::now();
            if now > event_loop_deadline || now > hard_deadline {
                debug!(
                    elapsed_ms = event_loop_start.elapsed().as_millis() as u64,
                    iterations = loop_iter,
                    "event loop timeout"
                );
                break;
            }

            if should_interrupt.load(Ordering::Relaxed) {
                debug!("event loop interrupted by deadline");
                break;
            }

            loop_iter += 1;

            match tokio::time::timeout(
                Duration::from_secs(10),
                js_rt.rt.execute_pending_job(),
            ).await {
                Ok(_) => {}
                Err(_) => {
                    warn!("execute_pending_job timed out (10s), breaking event loop");
                    break;
                }
            }

            match tokio::time::timeout(
                Duration::from_secs(5),
                js_rt.ctx.with(|ctx| {
                    let mut tq = tq_for_loop.lock().unwrap();
                    tq.fire_ready(&ctx).unwrap_or(false)
                }),
            ).await {
                Ok(fired) => {
                    let (has_pending, next_fire_time) = {
                        let tq = timer_queue.lock().unwrap();
                        ( !tq.is_empty(), tq.next_fire_time() )
                    };
                    let has_inflight = inflight.load(Ordering::Relaxed) > 0;

                    if !fired && !has_pending && !has_inflight {
                        idle_ticks += 1;
                        if idle_ticks >= 3 {
                            debug!("event loop idle: no timers, no inflight requests");
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    } else if !fired {
                        if !has_inflight {
                            if let Some(next_fire) = next_fire_time {
                                let delay = next_fire.saturating_duration_since(Instant::now());
                                if delay > MAX_SETTLE_TIMER_DELAY {
                                    debug!(
                                        delay_ms = delay.as_millis() as u64,
                                        "event loop settled before delayed timer"
                                    );
                                    break;
                                }
                                tokio::time::sleep(delay.min(Duration::from_millis(10))).await;
                            } else {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        } else {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        idle_ticks = 0;
                    } else {
                        idle_ticks = 0;
                    }
                }
                Err(_) => {
                    warn!("timer fire_ready timed out (5s), breaking event loop");
                    break;
                }
            }
        }

        should_interrupt.store(true, Ordering::Relaxed);

        js_rt.ctx.with(|ctx| {
            timer_queue.lock().unwrap().drain_all(&ctx);
        }).await;

        drop(js_rt);
        drop(timer_queue);
        drop(tq_for_loop);

        match Arc::try_unwrap(shared_doc) {
            Ok(mutex) => Ok(mutex.into_inner().unwrap()),
            Err(arc) => {
                debug!("Arc has {} strong refs, re-parsing HTML", Arc::strong_count(&arc));
                let doc = arc.lock().unwrap();
                let html = crate::dom::serialize_to_html(&doc.arena, doc.document_node);
                drop(doc);
                Ok(parse_html(&html, Some(url.to_string())))
            }
        }
    }

    pub fn to_html(&self) -> String {
        if let Some(body) = self.document.body() {
            serialize_inner_html(&self.document.arena, body)
        } else if let Some(doc_elem) = self.document.document_element() {
            serialize_inner_html(&self.document.arena, doc_elem)
        } else {
            serialize_inner_html(&self.document.arena, self.document.document_node)
        }
    }

    pub fn to_full_html(&self) -> String {
        crate::dom::serialize_to_html(&self.document.arena, self.document.document_node)
    }

    pub fn to_markdown(&self) -> Result<String> {
        let html = self.to_full_html();
        html_to_markdown(&html)
    }

    pub fn title(&self) -> Option<String> {
        self.document.title()
    }
}
