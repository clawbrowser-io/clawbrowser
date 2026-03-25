use std::time::Duration;

use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Proxy, Response};

use crate::config::FetchConfig;

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new(config: &FetchConfig) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .cookie_store(true)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .redirect(reqwest::redirect::Policy::limited(10));

        builder = builder.user_agent(&config.user_agent);

        if let Some(proxy_url) = &config.proxy {
            builder = builder.proxy(Proxy::all(proxy_url)?);
        }

        let mut default_headers = HeaderMap::new();
        default_headers.insert("accept", HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"));
        default_headers.insert("accept-language", HeaderValue::from_static("en-US,en;q=0.9"));

        for (key, value) in &config.extra_headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                default_headers.insert(name, val);
            }
        }

        if let Some(cookie) = &config.cookie {
            if let Ok(val) = HeaderValue::from_str(cookie) {
                default_headers.insert("cookie", val);
            }
        }

        builder = builder.default_headers(default_headers);

        Ok(Self {
            client: builder.build()?,
        })
    }

    pub async fn fetch_url(&self, url: &str) -> Result<Response> {
        let resp = self.client.get(url).send().await?;
        Ok(resp)
    }

    pub async fn fetch_text(&self, url: &str) -> Result<(String, String)> {
        let resp = self.fetch_url(url).await?;
        let final_url = resp.url().to_string();
        let body = resp.text().await?;
        Ok((body, final_url))
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }
}
