use std::sync::{Arc, Mutex};

use rquickjs::{Ctx, Function, Object, Result};

use crate::dom::{
    query_selector, query_selector_all, serialize_inner_html, serialize_outer_html,
    Document, NodeId, NodeType,
};

pub type SharedDoc = Arc<Mutex<Document>>;

pub fn install(ctx: &Ctx<'_>, doc: SharedDoc) -> Result<()> {
    let globals = ctx.globals();
    let bridge = Object::new(ctx.clone())?;

    let d = doc.clone();
    bridge.set("getElementById", Function::new(ctx.clone(), move |id: String| -> i32 {
        let doc = d.lock().unwrap();
        doc.get_element_by_id(&id).map(|n| n.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("querySelector", Function::new(ctx.clone(), move |root: u32, sel: String| -> i32 {
        let doc = d.lock().unwrap();
        query_selector(&doc.arena, NodeId(root), &sel).map(|n| n.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("querySelectorAll", Function::new(ctx.clone(), move |root: u32, sel: String| -> Vec<u32> {
        let doc = d.lock().unwrap();
        query_selector_all(&doc.arena, NodeId(root), &sel).iter().map(|n| n.0).collect()
    })?)?;

    let d = doc.clone();
    bridge.set("createElement", Function::new(ctx.clone(), move |tag: String| -> u32 {
        let mut doc = d.lock().unwrap();
        doc.create_element(&tag).0
    })?)?;

    let d = doc.clone();
    bridge.set("createTextNode", Function::new(ctx.clone(), move |text: String| -> u32 {
        let mut doc = d.lock().unwrap();
        doc.create_text_node(&text).0
    })?)?;

    let d = doc.clone();
    bridge.set("getElementsByTagName", Function::new(ctx.clone(), move |tag: String| -> Vec<u32> {
        let doc = d.lock().unwrap();
        doc.get_elements_by_tag_name(&tag).iter().map(|n| n.0).collect()
    })?)?;

    let d = doc.clone();
    bridge.set("getElementsByClassName", Function::new(ctx.clone(), move |cls: String| -> Vec<u32> {
        let doc = d.lock().unwrap();
        doc.get_elements_by_class_name(&cls).iter().map(|n| n.0).collect()
    })?)?;

    let d = doc.clone();
    bridge.set("getDocumentNode", Function::new(ctx.clone(), move || -> u32 {
        let doc = d.lock().unwrap();
        doc.document_node.0
    })?)?;

    let d = doc.clone();
    bridge.set("body", Function::new(ctx.clone(), move || -> i32 {
        let doc = d.lock().unwrap();
        doc.body().map(|n| n.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("head", Function::new(ctx.clone(), move || -> i32 {
        let doc = d.lock().unwrap();
        doc.head().map(|n| n.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("documentElement", Function::new(ctx.clone(), move || -> i32 {
        let doc = d.lock().unwrap();
        doc.document_element().map(|n| n.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("title", Function::new(ctx.clone(), move || -> String {
        let doc = d.lock().unwrap();
        doc.title().unwrap_or_default()
    })?)?;

    let d = doc.clone();
    bridge.set("getNodeType", Function::new(ctx.clone(), move |nid: u32| -> u32 {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).node_type.node_type_id()
    })?)?;

    let d = doc.clone();
    bridge.set("getNodeName", Function::new(ctx.clone(), move |nid: u32| -> String {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).node_type.node_name()
    })?)?;

    let d = doc.clone();
    bridge.set("getTagName", Function::new(ctx.clone(), move |nid: u32| -> String {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).node_type.node_name()
    })?)?;

    let d = doc.clone();
    bridge.set("getAttribute", Function::new(ctx.clone(), move |nid: u32, name: String| -> Option<String> {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).as_element().and_then(|e| e.get_attribute(&name)).map(String::from)
    })?)?;

    let d = doc.clone();
    bridge.set("setAttribute", Function::new(ctx.clone(), move |nid: u32, name: String, value: String| {
        let mut doc = d.lock().unwrap();
        if let Some(elem) = doc.arena.get_mut(NodeId(nid)).as_element_mut() {
            elem.set_attribute(&name, value);
        }
    })?)?;

    let d = doc.clone();
    bridge.set("removeAttribute", Function::new(ctx.clone(), move |nid: u32, name: String| {
        let mut doc = d.lock().unwrap();
        if let Some(elem) = doc.arena.get_mut(NodeId(nid)).as_element_mut() {
            elem.remove_attribute(&name);
        }
    })?)?;

    let d = doc.clone();
    bridge.set("hasAttribute", Function::new(ctx.clone(), move |nid: u32, name: String| -> bool {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).as_element().and_then(|e| e.get_attribute(&name)).is_some()
    })?)?;

    let d = doc.clone();
    bridge.set("getInnerHTML", Function::new(ctx.clone(), move |nid: u32| -> String {
        let doc = d.lock().unwrap();
        serialize_inner_html(&doc.arena, NodeId(nid))
    })?)?;

    let d = doc.clone();
    bridge.set("setInnerHTML", Function::new(ctx.clone(), move |nid: u32, html: String| {
        let mut doc = d.lock().unwrap();
        doc.arena.remove_all_children(NodeId(nid));
        let fragment = crate::html::parse_html(&format!("<body>{html}</body>"), None);
        if let Some(body) = fragment.body() {
            let children = fragment.arena.children(body);
            reparent_from_fragment(&mut doc, &fragment, NodeId(nid), &children);
        }
    })?)?;

    let d = doc.clone();
    bridge.set("getOuterHTML", Function::new(ctx.clone(), move |nid: u32| -> String {
        let doc = d.lock().unwrap();
        serialize_outer_html(&doc.arena, NodeId(nid))
    })?)?;

    let d = doc.clone();
    bridge.set("getTextContent", Function::new(ctx.clone(), move |nid: u32| -> String {
        let doc = d.lock().unwrap();
        doc.arena.deep_text_content(NodeId(nid))
    })?)?;

    let d = doc.clone();
    bridge.set("setTextContent", Function::new(ctx.clone(), move |nid: u32, text: String| {
        let mut doc = d.lock().unwrap();
        doc.arena.remove_all_children(NodeId(nid));
        let text_node = doc.create_text_node(&text);
        doc.arena.append_child(NodeId(nid), text_node);
    })?)?;

    let d = doc.clone();
    bridge.set("getParent", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).parent.map(|p| p.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("getChildren", Function::new(ctx.clone(), move |nid: u32| -> Vec<u32> {
        let doc = d.lock().unwrap();
        doc.arena.children(NodeId(nid)).iter().map(|n| n.0).collect()
    })?)?;

    let d = doc.clone();
    bridge.set("getChildElements", Function::new(ctx.clone(), move |nid: u32| -> Vec<u32> {
        let doc = d.lock().unwrap();
        doc.arena.child_elements(NodeId(nid)).iter().map(|n| n.0).collect()
    })?)?;

    let d = doc.clone();
    bridge.set("getFirstChild", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).first_child.map(|c| c.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("getLastChild", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).last_child.map(|c| c.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("getNextSibling", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).next_sibling.map(|s| s.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("getPrevSibling", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        doc.arena.get(NodeId(nid)).prev_sibling.map(|s| s.0 as i32).unwrap_or(-1)
    })?)?;

    let d = doc.clone();
    bridge.set("appendChild", Function::new(ctx.clone(), move |parent: u32, child: u32| {
        let mut doc = d.lock().unwrap();
        doc.arena.append_child(NodeId(parent), NodeId(child));
    })?)?;

    let d = doc.clone();
    bridge.set("removeChild", Function::new(ctx.clone(), move |_parent: u32, child: u32| {
        let mut doc = d.lock().unwrap();
        doc.arena.detach(NodeId(child));
    })?)?;

    let d = doc.clone();
    bridge.set("insertBefore", Function::new(ctx.clone(), move |parent: u32, new_child: u32, ref_child: i32| {
        let mut doc = d.lock().unwrap();
        if ref_child < 0 {
            doc.arena.append_child(NodeId(parent), NodeId(new_child));
        } else {
            doc.arena.insert_before(NodeId(parent), NodeId(new_child), NodeId(ref_child as u32));
        }
    })?)?;

    let d = doc.clone();
    bridge.set("isElement", Function::new(ctx.clone(), move |nid: u32| -> bool {
        let doc = d.lock().unwrap();
        matches!(doc.arena.get(NodeId(nid)).node_type, NodeType::Element(_))
    })?)?;

    let d = doc.clone();
    bridge.set("cloneNode", Function::new(ctx.clone(), move |nid: u32, deep: bool| -> u32 {
        let mut doc = d.lock().unwrap();
        doc.arena.clone_node(NodeId(nid), deep).0
    })?)?;

    let d = doc.clone();
    bridge.set("getDataAttributes", Function::new(ctx.clone(), move |nid: u32| -> Vec<Vec<String>> {
        let doc = d.lock().unwrap();
        let mut result = Vec::new();
        if let Some(elem) = doc.arena.get(NodeId(nid)).as_element() {
            for attr in &elem.attributes {
                if attr.name.local.starts_with("data-") {
                    let key = attr.name.local[5..].to_string();
                    result.push(vec![key, attr.value.clone()]);
                }
            }
        }
        result
    })?)?;

    let d = doc.clone();
    bridge.set("getAllAttributes", Function::new(ctx.clone(), move |nid: u32| -> Vec<Vec<String>> {
        let doc = d.lock().unwrap();
        let mut result = Vec::new();
        if let Some(elem) = doc.arena.get(NodeId(nid)).as_element() {
            for attr in &elem.attributes {
                result.push(vec![attr.name.local.clone(), attr.value.clone()]);
            }
        }
        result
    })?)?;

    let d = doc.clone();
    bridge.set("getNextElementSibling", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        let mut cur = doc.arena.get(NodeId(nid)).next_sibling;
        while let Some(id) = cur {
            if matches!(doc.arena.get(id).node_type, NodeType::Element(_)) {
                return id.0 as i32;
            }
            cur = doc.arena.get(id).next_sibling;
        }
        -1
    })?)?;

    let d = doc.clone();
    bridge.set("getPrevElementSibling", Function::new(ctx.clone(), move |nid: u32| -> i32 {
        let doc = d.lock().unwrap();
        let mut cur = doc.arena.get(NodeId(nid)).prev_sibling;
        while let Some(id) = cur {
            if matches!(doc.arena.get(id).node_type, NodeType::Element(_)) {
                return id.0 as i32;
            }
            cur = doc.arena.get(id).prev_sibling;
        }
        -1
    })?)?;

    globals.set("__dom", bridge)?;
    Ok(())
}

fn reparent_from_fragment(dest: &mut Document, src: &Document, parent: NodeId, src_children: &[NodeId]) {
    for &src_id in src_children {
        let new_type = src.arena.get(src_id).node_type.clone();
        let new_id = dest.arena.alloc(new_type);
        dest.arena.append_child(parent, new_id);
        let gc = src.arena.children(src_id);
        reparent_from_fragment(dest, src, new_id, &gc);
    }
}
