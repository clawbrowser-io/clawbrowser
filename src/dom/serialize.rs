use super::arena::Arena;
use super::node::{NodeId, NodeType};

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];

pub fn serialize_to_html(arena: &Arena, node: NodeId) -> String {
    let mut buf = String::with_capacity(1024);
    serialize_node(arena, node, &mut buf);
    buf
}

pub fn serialize_inner_html(arena: &Arena, node: NodeId) -> String {
    let mut buf = String::with_capacity(512);
    let mut child = arena.get(node).first_child;
    while let Some(id) = child {
        serialize_node(arena, id, &mut buf);
        child = arena.get(id).next_sibling;
    }
    buf
}

pub fn serialize_outer_html(arena: &Arena, node: NodeId) -> String {
    serialize_to_html(arena, node)
}

fn serialize_node(arena: &Arena, node: NodeId, buf: &mut String) {
    let data = arena.get(node);
    match &data.node_type {
        NodeType::Document => {
            let mut child = data.first_child;
            while let Some(id) = child {
                serialize_node(arena, id, buf);
                child = arena.get(id).next_sibling;
            }
        }
        NodeType::DocType { name, .. } => {
            buf.push_str("<!DOCTYPE ");
            buf.push_str(name);
            buf.push('>');
        }
        NodeType::Element(elem) => {
            let tag = elem.tag_name();
            buf.push('<');
            buf.push_str(tag);
            for attr in &elem.attributes {
                buf.push(' ');
                buf.push_str(&attr.name.local);
                buf.push_str("=\"");
                buf.push_str(&escape_attr(&attr.value));
                buf.push('"');
            }
            buf.push('>');

            if VOID_ELEMENTS.contains(&tag) {
                return;
            }

            let mut child = data.first_child;
            while let Some(id) = child {
                serialize_node(arena, id, buf);
                child = arena.get(id).next_sibling;
            }

            buf.push_str("</");
            buf.push_str(tag);
            buf.push('>');
        }
        NodeType::Text(text) => {
            let is_raw = data
                .parent
                .and_then(|p| arena.get(p).as_element())
                .map(|e| RAW_TEXT_ELEMENTS.contains(&e.tag_name()))
                .unwrap_or(false);

            if is_raw {
                buf.push_str(text);
            } else {
                buf.push_str(&escape_html(text));
            }
        }
        NodeType::Comment(text) => {
            buf.push_str("<!--");
            buf.push_str(text);
            buf.push_str("-->");
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
