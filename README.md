# ClawBrowser

A lightweight headless browser CLI for web scraping with JavaScript execution. Built in Rust with QuickJS.

**Single binary (~6.5 MB), zero external dependencies, runs anywhere.**

## Why ClawBrowser?

### vs. Chrome / Puppeteer / Playwright

| | ClawBrowser | Chrome + Puppeteer |
|---|---|---|
| Binary size | **~6.5 MB** | 200 MB+ (Chrome) + 80 MB+ (Node.js) |
| Memory usage | **10–30 MB** | 100–300 MB per tab |
| Startup time | **Near instant** | 1–3 seconds cold start |
| Deployment | Single static binary | Chrome + Node.js + npm install |
| JS execution | QuickJS (lightweight) | V8 (full) |

### vs. AI Built-in Web Fetch (ChatGPT / Claude / Cursor)

AI tools' built-in web fetch is essentially a plain HTTP GET — no JavaScript, no interactivity, no control.

| | ClawBrowser | AI Web Fetch |
|---|---|---|
| JavaScript execution | Full QuickJS engine | None |
| Dynamic / SPA content | Rendered after JS | Blank or incomplete |
| Cookie & auth | Full support | Limited or none |
| Proxy support | HTTP / SOCKS5 | None |
| Login modal removal | Auto-cleaned | Not handled |
| Batch scraping | Concurrent with semaphore | Not supported |
| Rate limits | You control | Strict cloud limits |
| Runs locally | Yes, offline capable | Cloud-dependent |
| Output quality | Clean Markdown, auto denoised | Raw HTML or plain text |

### vs. curl / wget

| | ClawBrowser | curl |
|---|---|---|
| JavaScript execution | Yes | No |
| HTML → Markdown | Built-in | Requires extra tools |
| Overlay / modal removal | Automatic | No |
| Batch mode | Built-in | Scripting required |

## Features

- Fetch static and dynamic web pages
- Execute page JavaScript via embedded QuickJS engine
- Convert HTML to clean Markdown output
- Auto-remove intrusive overlays (login modals, cookie banners)
- Single page fetch and concurrent batch scraping
- Custom User-Agent, Cookie, headers, proxy (HTTP/SOCKS5)
- Graceful error recovery — JS failure falls back to static DOM

## Quick Start

### Build from source

```bash
# Release build (~6.5 MB binary)
cargo build --release

# Binary location
# Linux/macOS: target/release/clawbrowser
# Windows:     target/release/clawbrowser.exe
```

### Usage

```bash
# Fetch a page as Markdown
clawbrowser fetch https://example.com

# Fetch with JS execution, custom wait time
clawbrowser fetch https://example.com --wait 3000

# Static fetch (no JS, faster)
clawbrowser fetch https://example.com --no-js

# Output to file
clawbrowser fetch https://example.com -o result.md

# Output full HTML
clawbrowser fetch https://example.com --html

# With cookie and proxy
clawbrowser fetch https://example.com --cookie "sid=abc" --proxy socks5://127.0.0.1:1080

# Batch fetch from URL list
clawbrowser batch urls.txt -o ./output --concurrency 5

# Verbose logging
clawbrowser fetch https://example.com -v
```

### Full CLI Reference

| Flag | `fetch` | `batch` | Description |
|---|:---:|:---:|---|
| `--timeout, -t` | Y | Y | JS execution timeout in seconds (default: 30) |
| `--wait, -w` | Y | Y | Page load wait in ms (default: 5000) |
| `--no-js` | Y | Y | Disable JavaScript execution |
| `--user-agent` | Y | Y | Custom User-Agent string |
| `--proxy` | Y | Y | HTTP/SOCKS5 proxy URL |
| `--verbose, -v` | Y | Y | Enable debug logging |
| `--output, -o` | Y | Y | Output file / directory |
| `--cookie` | Y | | Cookie header value |
| `--header` | Y | | Extra headers (K:V format) |
| `--html` | Y | | Output HTML instead of Markdown |
| `--concurrency` | | Y | Max concurrent fetches (default: 10) |

## Architecture

```
┌─────────────────────────────────────────────┐
│                  CLI (clap)                 │
│            fetch / batch subcommands        │
├─────────────────────────────────────────────┤
│               Engine (Page)                 │
│   navigate → parse → run_js → to_markdown   │
├──────────┬──────────────┬───────────────────┤
│ HTTP     │  HTML Parser  │   JS Runtime     │
│ (reqwest)│  (html5ever)  │  (rquickjs)      │
├──────────┴──────────────┴───────────────────┤
│              Custom DOM (Arena)              │
│  Node / Element / Tree / Selector / Serialize│
├─────────────────────────────────────────────┤
│           Output (htmd → Markdown)          │
└─────────────────────────────────────────────┘
```

**Key design decisions:**

- **Arena-based DOM** — all nodes in a contiguous `Vec<NodeData>`, indexed by `NodeId(u32)`. Cache-friendly, no GC.
- **Bridge + Shim architecture** — Rust bridge returns only primitives (`u32`/`String`/`bool`), JS shim wraps them into full DOM API objects. Avoids `rquickjs::Value` lifetime invariance issues.
- **QuickJS** — lightweight, embeddable, compiles fast, no external dependencies.
- **Error recovery** — if JS execution fails completely, falls back to static DOM (re-parses original HTML). Always returns a valid result.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full technical deep-dive.

## Tech Stack

| Crate | Purpose |
|---|---|
| [clap](https://crates.io/crates/clap) | CLI argument parsing |
| [tokio](https://crates.io/crates/tokio) | Async runtime |
| [reqwest](https://crates.io/crates/reqwest) | HTTP client (rustls-tls, proxy, cookies, compression) |
| [html5ever](https://crates.io/crates/html5ever) | HTML5 spec-compliant parser |
| [rquickjs](https://crates.io/crates/rquickjs) | QuickJS Rust bindings |
| [htmd](https://crates.io/crates/htmd) | HTML → Markdown conversion |

## Tests

```bash
cargo test
```

17 integration tests covering DOM parsing, element queries, serialization, DOM manipulation, Markdown conversion, and network integration.

## License

[MIT](LICENSE)
