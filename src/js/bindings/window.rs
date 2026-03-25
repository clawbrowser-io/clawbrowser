use rquickjs::{Ctx, Function, Object, Result};

pub fn install(ctx: &Ctx<'_>, url: &str) -> Result<()> {
    let globals = ctx.globals();

    let location = Object::new(ctx.clone())?;
    location.set("href", url)?;
    location.set("reload", Function::new(ctx.clone(), || {})?)?;
    location.set("assign", Function::new(ctx.clone(), |_url: String| {})?)?;
    location.set("replace", Function::new(ctx.clone(), |_url: String| {})?)?;
    location.set("toString", Function::new(ctx.clone(), {
        let u = url.to_string();
        move || -> String { u.clone() }
    })?)?;
    if let Ok(parsed) = url::Url::parse(url) {
        location.set("protocol", format!("{}:", parsed.scheme()))?;
        location.set("hostname", parsed.host_str().unwrap_or(""))?;
        location.set("host", parsed.host_str().unwrap_or(""))?;
        location.set("pathname", parsed.path())?;
        location.set("search", if let Some(q) = parsed.query() { format!("?{q}") } else { String::new() })?;
        location.set("hash", if let Some(f) = parsed.fragment() { format!("#{f}") } else { String::new() })?;
        location.set("origin", parsed.origin().ascii_serialization())?;
        location.set("port", parsed.port().map(|p| p.to_string()).unwrap_or_default())?;
    }
    globals.set("location", location)?;

    let navigator = Object::new(ctx.clone())?;
    navigator.set("userAgent", crate::config::DEFAULT_USER_AGENT)?;
    navigator.set("language", "en-US")?;
    navigator.set("languages", vec!["en-US", "en"])?;
    navigator.set("platform", std::env::consts::OS)?;
    navigator.set("cookieEnabled", true)?;
    navigator.set("onLine", true)?;
    navigator.set("maxTouchPoints", 0)?;
    navigator.set("vendor", "ClawBrowser")?;
    navigator.set("appName", "Netscape")?;
    navigator.set("appVersion", "5.0")?;
    navigator.set("product", "Gecko")?;
    navigator.set("hardwareConcurrency", 4)?;
    let media_devices = Object::new(ctx.clone())?;
    media_devices.set("getUserMedia", Function::new(ctx.clone(), || {})?)?;
    navigator.set("mediaDevices", media_devices)?;
    navigator.set("sendBeacon", Function::new(ctx.clone(), || -> bool { true })?)?;
    globals.set("navigator", navigator)?;

    globals.set("window", globals.clone())?;
    globals.set("self", globals.clone())?;
    globals.set("globalThis", globals.clone())?;
    globals.set("top", globals.clone())?;
    globals.set("parent", globals.clone())?;
    globals.set("frames", globals.clone())?;

    let screen = Object::new(ctx.clone())?;
    screen.set("width", 1920)?;
    screen.set("height", 1080)?;
    screen.set("availWidth", 1920)?;
    screen.set("availHeight", 1080)?;
    screen.set("colorDepth", 24)?;
    screen.set("pixelDepth", 24)?;
    globals.set("screen", screen)?;

    globals.set("atob", Function::new(ctx.clone(), |s: String| -> String {
        let decoded = base64_decode(&s);
        String::from_utf8(decoded).unwrap_or_default()
    })?)?;
    globals.set("btoa", Function::new(ctx.clone(), |s: String| -> String {
        base64_encode(s.as_bytes())
    })?)?;

    let perf = Object::new(ctx.clone())?;
    let start = std::time::Instant::now();
    perf.set("now", Function::new(ctx.clone(), move || -> f64 {
        start.elapsed().as_secs_f64() * 1000.0
    })?)?;
    perf.set("getEntriesByType", Function::new(ctx.clone(), |_t: String| -> Vec<String> { vec![] })?)?;
    perf.set("getEntriesByName", Function::new(ctx.clone(), |_n: String| -> Vec<String> { vec![] })?)?;
    perf.set("mark", Function::new(ctx.clone(), |_n: String| {})?)?;
    perf.set("measure", Function::new(ctx.clone(), |_n: String| {})?)?;
    let timing = Object::new(ctx.clone())?;
    timing.set("navigationStart", 0)?;
    perf.set("timing", timing)?;
    globals.set("performance", perf)?;

    Ok(())
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { result.push(TABLE[((n >> 6) & 0x3F) as usize] as char); } else { result.push('='); }
        if chunk.len() > 2 { result.push(TABLE[(n & 0x3F) as usize] as char); } else { result.push('='); }
    }
    result
}

fn base64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|&b| b != b'=' && b != b'\n' && b != b'\r' && b != b' ').collect();
    let mut result = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 { break; }
        let a = val(chunk[0]) as u32;
        let b = val(chunk[1]) as u32;
        let c = if chunk.len() > 2 { val(chunk[2]) as u32 } else { 0 };
        let d = if chunk.len() > 3 { val(chunk[3]) as u32 } else { 0 };
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        result.push((n >> 16) as u8);
        if chunk.len() > 2 { result.push((n >> 8) as u8); }
        if chunk.len() > 3 { result.push(n as u8); }
    }
    result
}
