use anyhow::Result;
use tracing::{debug, warn};

use crate::dom::{Document, NodeId, NodeType};

pub struct ScriptInfo {
    pub node_id: NodeId,
    pub src: Option<String>,
    pub inline_code: Option<String>,
    pub is_module: bool,
    pub is_async: bool,
    pub is_deferred: bool,
}

pub fn find_scripts(doc: &Document) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    collect_scripts(doc, doc.document_node, &mut scripts);
    scripts
}

fn collect_scripts(doc: &Document, node: NodeId, scripts: &mut Vec<ScriptInfo>) {
    let data = doc.arena.get(node);
    if let NodeType::Element(elem) = &data.node_type {
        if elem.tag_name() == "script" {
            let script_type = elem.get_attribute("type").unwrap_or("").to_lowercase();

            let should_skip = script_type == "application/json"
                || script_type == "application/ld+json"
                || script_type == "text/template"
                || script_type == "text/x-template";

            if !should_skip {
                let src = elem.get_attribute("src").map(String::from);
                let is_module = script_type == "module";
                let is_async = elem.get_attribute("async").is_some();
                let is_deferred = elem.get_attribute("defer").is_some();

                let inline_code = if src.is_none() {
                    let text = doc.arena.deep_text_content(node);
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                } else {
                    None
                };

                scripts.push(ScriptInfo {
                    node_id: node,
                    src,
                    inline_code,
                    is_module,
                    is_async,
                    is_deferred,
                });
            }
        }
    }

    let mut child = data.first_child;
    while let Some(id) = child {
        collect_scripts(doc, id, scripts);
        child = doc.arena.get(id).next_sibling;
    }
}

pub async fn load_script(
    client: &reqwest::Client,
    base_url: &str,
    src: &str,
) -> Result<String> {
    let full_url = resolve_url(base_url, src);
    debug!(url = full_url.as_str(), "loading external script");

    match client.get(&full_url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let text = resp.text().await?;
                debug!(url = full_url.as_str(), len = text.len(), "script loaded");
                Ok(text)
            } else {
                warn!(
                    url = full_url.as_str(),
                    status = resp.status().as_u16(),
                    "script load failed"
                );
                Ok(String::new())
            }
        }
        Err(e) => {
            warn!(url = full_url.as_str(), error = %e, "script load error");
            Ok(String::new())
        }
    }
}

fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") || relative.starts_with("//") {
        if relative.starts_with("//") {
            let scheme = if base.starts_with("https") { "https:" } else { "http:" };
            return format!("{scheme}{relative}");
        }
        return relative.to_string();
    }
    if let Ok(base_url) = url::Url::parse(base) {
        if let Ok(resolved) = base_url.join(relative) {
            return resolved.to_string();
        }
    }
    relative.to_string()
}
