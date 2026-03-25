use std::collections::HashMap;

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 ClawBrowser/0.1";

pub struct FetchConfig {
    pub timeout_secs: u64,
    pub wait_ms: u64,
    pub no_js: bool,
    pub user_agent: String,
    pub cookie: Option<String>,
    pub proxy: Option<String>,
    pub extra_headers: HashMap<String, String>,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            wait_ms: 5000,
            no_js: false,
            user_agent: DEFAULT_USER_AGENT.to_string(),
            cookie: None,
            proxy: None,
            extra_headers: HashMap::new(),
        }
    }
}
