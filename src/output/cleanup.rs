use std::collections::HashSet;

use crate::dom::{Document, NodeId};

const BODY_LOCK_STYLE_KEYS: &[&str] = &[
    "overflow",
    "overflow-x",
    "overflow-y",
    "position",
    "touch-action",
];

const MODAL_CLASS_TOKENS: &[&str] = &[
    "dialog",
    "modal",
    "popup",
    "popover",
    "overlay",
    "backdrop",
    "mask",
    "scrim",
    "drawer",
    "sheet",
];

const AUTH_TOKENS: &[&str] = &[
    "login",
    "signin",
    "sign-in",
    "signup",
    "sign-up",
    "register",
    "auth",
    "passport",
    "二维码",
    "扫码",
    "登录",
    "注册",
];

const NATIVE_HREF_TAGS: &[&str] = &[
    "a", "area", "base", "link",
];

pub fn promote_href_elements(document: &mut Document) -> usize {
    let root = document
        .document_element()
        .unwrap_or(document.document_node);

    let mut promoted = 0usize;
    for node_id in document.arena.descendants(root) {
        let data = document.arena.get(node_id);
        let Some(elem) = data.as_element() else {
            continue;
        };
        let tag = elem.tag_name();
        if NATIVE_HREF_TAGS.iter().any(|&t| t == tag) {
            continue;
        }
        if elem.get_attribute("href").is_none() {
            continue;
        }
        let elem_mut = document.arena.get_mut(node_id).as_element_mut().unwrap();
        elem_mut.tag.local = "a".to_string();
        promoted += 1;
    }
    promoted
}

/// Rewrites protocol-relative URLs (`//example.com`) to `https://example.com`
/// on all `href` and `src` attributes across the document.
pub fn normalize_protocol_relative_urls(document: &mut Document) {
    let root = document
        .document_element()
        .unwrap_or(document.document_node);

    for node_id in document.arena.descendants(root) {
        let Some(elem) = document.arena.get_mut(node_id).as_element_mut() else {
            continue;
        };
        for attr in &mut elem.attributes {
            let name = attr.name.local.as_str();
            if !matches!(name, "href" | "src" | "srcset" | "action") {
                continue;
            }
            if attr.value.starts_with("//") {
                attr.value = format!("https:{}", attr.value);
            }
        }
    }
}

pub fn cleanup_intrusive_overlays(document: &mut Document) -> usize {
    let root = document
        .document_element()
        .unwrap_or(document.document_node);

    let mut candidates = Vec::new();
    for node_id in document.arena.descendants(root) {
        if should_remove_node(document, node_id) {
            candidates.push(node_id);
        }
    }

    let candidate_set: HashSet<NodeId> = candidates.iter().copied().collect();
    let top_level: Vec<NodeId> = candidates
        .into_iter()
        .filter(|&node_id| !has_candidate_ancestor(document, node_id, &candidate_set))
        .collect();

    for node_id in &top_level {
        document.arena.detach(*node_id);
    }

    cleanup_scroll_lock(document);

    top_level.len()
}

fn should_remove_node(document: &Document, node_id: NodeId) -> bool {
    let data = document.arena.get(node_id);
    let Some(elem) = data.as_element() else {
        return false;
    };

    let tag = elem.tag_name();
    if matches!(tag, "html" | "head" | "body" | "main" | "article") {
        return false;
    }

    let text = normalize_text(&document.arena.deep_text_content(node_id));
    let attr_blob = collect_attr_blob(elem);
    let style = elem.get_attribute("style").unwrap_or("").to_ascii_lowercase();

    let has_modal_role = matches!(
        elem.get_attribute("role"),
        Some("dialog") | Some("alertdialog")
    );
    let aria_modal = matches!(
        elem.get_attribute("aria-modal"),
        Some("true") | Some("True") | Some("TRUE")
    );
    let auth_hit = contains_any(&attr_blob, AUTH_TOKENS) || contains_any(&text, AUTH_TOKENS);
    let modal_hit = contains_any(&attr_blob, MODAL_CLASS_TOKENS);
    let backdrop_like = is_backdrop_like(&style, &attr_blob, &text);
    let fixed_like = style.contains("position:fixed") || style.contains("position: fixed");
    let high_z = extract_z_index(&style).is_some_and(|z| z >= 100);

    let text_len = text.chars().count();
    let likely_main_content = text_len > 1200 || (matches!(tag, "section" | "div") && text_len > 2000);
    if likely_main_content && !has_modal_role && !aria_modal {
        return false;
    }

    if has_modal_role || aria_modal {
        return true;
    }

    if auth_hit && (modal_hit || backdrop_like || fixed_like) {
        return true;
    }

    if backdrop_like {
        return true;
    }

    modal_hit && fixed_like && high_z && text_len < 1200
}

fn has_candidate_ancestor(
    document: &Document,
    node_id: NodeId,
    candidates: &HashSet<NodeId>,
) -> bool {
    let mut parent = document.arena.get(node_id).parent;
    while let Some(parent_id) = parent {
        if candidates.contains(&parent_id) {
            return true;
        }
        parent = document.arena.get(parent_id).parent;
    }
    false
}

fn cleanup_scroll_lock(document: &mut Document) {
    for node_id in [
        document.document_element(),
        document.body(),
    ]
    .into_iter()
    .flatten()
    {
        let Some(elem) = document.arena.get_mut(node_id).as_element_mut() else {
            continue;
        };

        if let Some(style) = elem.get_attribute("style").map(str::to_string) {
            let cleaned = remove_style_properties(&style, BODY_LOCK_STYLE_KEYS);
            if cleaned.is_empty() {
                elem.remove_attribute("style");
            } else {
                elem.set_attribute("style", cleaned);
            }
        }

        if let Some(class_name) = elem.get_attribute("class").map(str::to_string) {
            let cleaned = class_name
                .split_whitespace()
                .filter(|cls| {
                    let cls = cls.to_ascii_lowercase();
                    !cls.contains("modal-open")
                        && !cls.contains("overflow-hidden")
                        && !cls.contains("no-scroll")
                        && !cls.contains("scroll-lock")
                })
                .collect::<Vec<_>>()
                .join(" ");
            if cleaned.is_empty() {
                elem.remove_attribute("class");
            } else {
                elem.set_attribute("class", cleaned);
            }
        }
    }
}

fn collect_attr_blob(elem: &crate::dom::ElementData) -> String {
    let mut parts = vec![elem.tag_name().to_ascii_lowercase()];
    for attr in &elem.attributes {
        parts.push(attr.name.local.to_ascii_lowercase());
        parts.push(attr.value.to_ascii_lowercase());
    }
    parts.join(" ")
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_ascii_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn is_backdrop_like(style: &str, attr_blob: &str, text: &str) -> bool {
    let token_hit = contains_any(attr_blob, &["overlay", "backdrop", "mask", "scrim"]);
    let fullscreen_like = style.contains("inset:0")
        || style.contains("inset: 0")
        || (style.contains("top:0")
            && style.contains("left:0")
            && (style.contains("right:0")
                || style.contains("right: 0")
                || style.contains("width:100%")
                || style.contains("width: 100%")));
    let fixed_like = style.contains("position:fixed") || style.contains("position: fixed");
    let dim_like = style.contains("rgba(")
        || style.contains("background: rgb(0, 0, 0")
        || style.contains("background-color: rgb(0, 0, 0")
        || style.contains("opacity:");
    let low_text = text.chars().count() <= 40;

    token_hit && low_text || (fixed_like && fullscreen_like && dim_like && low_text)
}

fn extract_z_index(style: &str) -> Option<i32> {
    style.split(';').find_map(|part| {
        let (name, value) = part.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("z-index") {
            value.trim().parse::<i32>().ok()
        } else {
            None
        }
    })
}

fn remove_style_properties(style: &str, keys: &[&str]) -> String {
    style
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (name, value) = part.split_once(':')?;
            let name = name.trim();
            if keys.iter().any(|key| name.eq_ignore_ascii_case(key)) {
                return None;
            }
            Some(format!("{}: {}", name, value.trim()))
        })
        .collect::<Vec<_>>()
        .join("; ")
}
