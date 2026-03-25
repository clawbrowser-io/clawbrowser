use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

use clawbrowser::config::FetchConfig;
use clawbrowser::engine::Page;

#[derive(Parser)]
#[command(
    name = "clawbrowser",
    version,
    about = "A lightweight headless browser for web scraping"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a single URL and output as Markdown
    Fetch {
        /// URL to fetch
        url: String,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// JS execution timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Wait time for page load in milliseconds
        #[arg(short, long, default_value = "5000")]
        wait: u64,

        /// Disable JavaScript execution
        #[arg(long)]
        no_js: bool,

        /// Custom User-Agent string
        #[arg(long)]
        user_agent: Option<String>,

        /// Cookie header value
        #[arg(long)]
        cookie: Option<String>,

        /// Extra headers in K:V format
        #[arg(long, value_name = "K:V")]
        header: Vec<String>,

        /// HTTP/SOCKS5 proxy URL
        #[arg(long)]
        proxy: Option<String>,

        /// Verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Output full HTML instead of Markdown
        #[arg(long)]
        html: bool,
    },
    /// Batch fetch URLs from a file
    Batch {
        /// File containing URLs, one per line
        file: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Max concurrency
        #[arg(long, default_value = "10")]
        concurrency: usize,

        /// JS execution timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// Wait time for page load in milliseconds
        #[arg(short, long, default_value = "5000")]
        wait: u64,

        /// Disable JavaScript execution
        #[arg(long)]
        no_js: bool,

        /// Custom User-Agent string
        #[arg(long)]
        user_agent: Option<String>,

        /// HTTP/SOCKS5 proxy URL
        #[arg(long)]
        proxy: Option<String>,

        /// Verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
}

fn build_config(
    timeout: u64,
    wait: u64,
    no_js: bool,
    user_agent: Option<String>,
    cookie: Option<String>,
    headers: &[String],
    proxy: Option<String>,
) -> FetchConfig {
    let extra_headers: HashMap<String, String> = headers
        .iter()
        .filter_map(|h| {
            let (k, v) = h.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();

    FetchConfig {
        timeout_secs: timeout,
        wait_ms: wait,
        no_js,
        user_agent: user_agent.unwrap_or_else(|| clawbrowser::config::DEFAULT_USER_AGENT.to_string()),
        cookie,
        proxy,
        extra_headers,
    }
}

fn init_tracing(verbose: bool) {
    use tracing_subscriber::EnvFilter;
    let filter = if verbose {
        EnvFilter::new("clawbrowser=debug")
    } else {
        EnvFilter::new("clawbrowser=info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn cmd_fetch(
    url: String,
    output: Option<PathBuf>,
    config: FetchConfig,
    output_html: bool,
) -> Result<()> {
    let page = Page::navigate(&url, &config).await?;

    let content = if output_html {
        page.to_full_html()
    } else {
        page.to_markdown()?
    };

    if let Some(title) = page.title() {
        info!(title = title.as_str(), "page title");
    }

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, &content).await?;
            info!(path = ?path, "written to file");
        }
        None => {
            println!("{content}");
        }
    }

    Ok(())
}

async fn cmd_batch(
    file: PathBuf,
    output_dir: Option<PathBuf>,
    concurrency: usize,
    config: FetchConfig,
) -> Result<()> {
    let content = tokio::fs::read_to_string(&file).await?;
    let urls: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    info!(count = urls.len(), "loaded URLs");

    let out_dir = output_dir.unwrap_or_else(|| PathBuf::from("./output"));
    tokio::fs::create_dir_all(&out_dir).await?;

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
    let config = std::sync::Arc::new(config);
    let out_dir = std::sync::Arc::new(out_dir);

    let local = tokio::task::LocalSet::new();
    for (i, url) in urls.into_iter().enumerate() {
        let sem = semaphore.clone();
        let cfg = config.clone();
        let dir = out_dir.clone();

        local.spawn_local(async move {
            let _permit = sem.acquire().await.unwrap();
            info!(i, url = url.as_str(), "fetching");

            match Page::navigate(&url, &cfg).await {
                Ok(page) => {
                    let md = match page.to_markdown() {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::error!(url = url.as_str(), error = %e, "markdown conversion failed");
                            return;
                        }
                    };

                    let filename = format!("{:04}.md", i);
                    let path = dir.join(filename);
                    if let Err(e) = tokio::fs::write(&path, &md).await {
                        tracing::error!(path = ?path, error = %e, "write failed");
                    } else {
                        info!(path = ?path, url = url.as_str(), "saved");
                    }
                }
                Err(e) => {
                    tracing::error!(url = url.as_str(), error = %e, "fetch failed");
                }
            }
        });
    }

    local.await;
    info!("batch complete");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Fetch {
            url,
            output,
            timeout,
            wait,
            no_js,
            user_agent,
            cookie,
            header,
            proxy,
            verbose,
            html,
        } => {
            init_tracing(verbose);
            let config = build_config(timeout, wait, no_js, user_agent, cookie, &header, proxy);
            cmd_fetch(url, output, config, html).await?;
        }
        Commands::Batch {
            file,
            output,
            concurrency,
            timeout,
            wait,
            no_js,
            user_agent,
            proxy,
            verbose,
        } => {
            init_tracing(verbose);
            let config = build_config(timeout, wait, no_js, user_agent, None, &[], proxy);
            cmd_batch(file, output, concurrency, config).await?;
        }
    }

    Ok(())
}
