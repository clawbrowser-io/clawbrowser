use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use rquickjs::{Ctx, Function, Object, Result};
use reqwest::Client;

use super::InflightCounter;

const FETCH_TIMEOUT_SECS: u64 = 10;

pub fn install(ctx: &Ctx<'_>, client: Arc<Client>, inflight: InflightCounter) -> Result<()> {
    let globals = ctx.globals();

    let fetch_bridge = Object::new(ctx.clone())?;

    let c = client.clone();
    let inf = inflight;
    fetch_bridge.set("doFetch", Function::new(ctx.clone(), move |url: String, method: String, body: Option<String>| -> Vec<String> {
        let client = c.clone();
        inf.fetch_add(1, Ordering::Relaxed);
        let result = tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let method = reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);
                let mut req = client.request(method, &url)
                    .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS));
                if let Some(body) = body {
                    req = req.body(body);
                }
                match req.send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16().to_string();
                        match tokio::time::timeout(
                            Duration::from_secs(FETCH_TIMEOUT_SECS),
                            resp.text(),
                        ).await {
                            Ok(Ok(text)) => vec!["ok".to_string(), status, text],
                            Ok(Err(e)) => vec!["error".to_string(), format!("body read error: {e}")],
                            Err(_) => vec!["error".to_string(), "body read timed out".to_string()],
                        }
                    }
                    Err(e) => {
                        vec!["error".to_string(), e.to_string()]
                    }
                }
            })
        });
        inf.fetch_sub(1, Ordering::Relaxed);
        result
    })?)?;

    globals.set("__fetch_bridge", fetch_bridge)?;

    ctx.eval::<(), _>(FETCH_SHIM_JS)?;

    globals.set("Headers", Function::new(ctx.clone(), || {})?)?;
    globals.set("Request", Function::new(ctx.clone(), |_url: String| {})?)?;
    globals.set("Response", Function::new(ctx.clone(), || {})?)?;

    Ok(())
}

const FETCH_SHIM_JS: &str = r#"
(function() {
    globalThis.fetch = function(url, opts) {
        opts = opts || {};
        var method = (opts.method || 'GET').toUpperCase();
        var body = opts.body || null;

        return new Promise(function(resolve, reject) {
            try {
                var result = __fetch_bridge.doFetch(String(url), method, body);
                if (result[0] === 'ok') {
                    var status = parseInt(result[1], 10);
                    var text = result[2];
                    resolve({
                        ok: status >= 200 && status < 300,
                        status: status,
                        statusText: status === 200 ? 'OK' : '',
                        headers: { get: function() { return null; } },
                        text: function() { return Promise.resolve(text); },
                        json: function() { return Promise.resolve(JSON.parse(text)); }
                    });
                } else {
                    reject(new Error(result[1] || 'fetch error'));
                }
            } catch(e) {
                reject(e);
            }
        });
    };
})();
"#;
