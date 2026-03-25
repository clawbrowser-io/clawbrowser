use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use html5ever::parse_document;
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute as H5Attribute, ExpandedName, QualName as H5QualName};
use markup5ever::{local_name, namespace_url, ns, LocalName, Namespace};

use crate::dom::{Document, ElementData, NodeId, NodeType, QualName};

struct Inner {
    doc: Document,
    id_map: HashMap<usize, NodeId>,
    next_id: usize,
    elem_names: HashMap<usize, (Namespace, LocalName)>,
}

struct DomSink {
    inner: RefCell<Inner>,
    fallback_ns: Namespace,
    fallback_local: LocalName,
}

impl DomSink {
    fn new(url: Option<String>) -> Self {
        let doc = match url {
            Some(u) => Document::with_url(u),
            None => Document::new(),
        };
        let doc_node = doc.document_node;
        let mut id_map = HashMap::new();
        id_map.insert(0, doc_node);
        Self {
            inner: RefCell::new(Inner {
                doc,
                id_map,
                next_id: 1,
                elem_names: HashMap::new(),
            }),
            fallback_ns: ns!(),
            fallback_local: local_name!(""),
        }
    }
}

impl TreeSink for DomSink {
    type Handle = usize;
    type Output = Document;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> Document {
        self.inner.into_inner().doc
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> usize {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a usize) -> ExpandedName<'a> {
        let inner = self.inner.borrow();
        if inner.elem_names.contains_key(target) {
            drop(inner);
            let inner = self.inner.borrow();
            let (ns, local) = &inner.elem_names[target];
            let ns_ptr: *const Namespace = ns;
            let local_ptr: *const LocalName = local;
            drop(inner);
            // SAFETY: elem_names entries are never removed or moved during parsing
            unsafe {
                ExpandedName {
                    ns: &*ns_ptr,
                    local: &*local_ptr,
                }
            }
        } else {
            ExpandedName {
                ns: &self.fallback_ns,
                local: &self.fallback_local,
            }
        }
    }

    fn create_element(
        &self,
        name: H5QualName,
        attrs: Vec<H5Attribute>,
        _flags: ElementFlags,
    ) -> usize {
        let mut inner = self.inner.borrow_mut();
        let tag = QualName {
            ns: name.ns.to_string(),
            local: name.local.to_string(),
        };
        let mut elem = ElementData::new(tag);
        for attr in &attrs {
            let attr_name = attr.name.local.to_string();
            let value = attr.value.to_string();
            if attr_name == "id" {
                elem.id_cache = Some(value.clone());
            }
            elem.attributes.push(crate::dom::Attribute {
                name: QualName::new(attr_name),
                value,
            });
        }
        let node_id = inner.doc.arena.alloc(NodeType::Element(elem));
        let handle = inner.next_id;
        inner.next_id += 1;
        inner.id_map.insert(handle, node_id);
        inner.elem_names.insert(handle, (name.ns, name.local));
        handle
    }

    fn create_comment(&self, text: StrTendril) -> usize {
        let mut inner = self.inner.borrow_mut();
        let node_id = inner.doc.arena.alloc(NodeType::Comment(text.to_string()));
        let handle = inner.next_id;
        inner.next_id += 1;
        inner.id_map.insert(handle, node_id);
        handle
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> usize {
        let mut inner = self.inner.borrow_mut();
        let node_id = inner.doc.arena.alloc(NodeType::Comment(String::new()));
        let handle = inner.next_id;
        inner.next_id += 1;
        inner.id_map.insert(handle, node_id);
        handle
    }

    fn append(&self, parent: &usize, child: NodeOrText<usize>) {
        let mut inner = self.inner.borrow_mut();
        let parent_id = inner.id_map[parent];
        match child {
            NodeOrText::AppendNode(handle) => {
                let child_id = inner.id_map[&handle];
                inner.doc.arena.append_child(parent_id, child_id);
            }
            NodeOrText::AppendText(text) => {
                let last = inner.doc.arena.get(parent_id).last_child;
                if let Some(last_id) = last {
                    if let NodeType::Text(ref mut existing) =
                        inner.doc.arena.get_mut(last_id).node_type
                    {
                        existing.push_str(&text);
                        return;
                    }
                }
                let node_id = inner.doc.arena.alloc(NodeType::Text(text.to_string()));
                inner.doc.arena.append_child(parent_id, node_id);
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &usize,
        prev_element: &usize,
        child: NodeOrText<usize>,
    ) {
        let has_parent = {
            let inner = self.inner.borrow();
            let elem_id = inner.id_map[element];
            inner.doc.arena.get(elem_id).parent.is_some()
        };
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        let mut inner = self.inner.borrow_mut();
        let node_id = inner.doc.arena.alloc(NodeType::DocType {
            name: name.to_string(),
            public_id: public_id.to_string(),
            system_id: system_id.to_string(),
        });
        let doc = inner.doc.document_node;
        inner.doc.arena.append_child(doc, node_id);
    }

    fn get_template_contents(&self, target: &usize) -> usize {
        *target
    }

    fn same_node(&self, x: &usize, y: &usize) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &usize, new_node: NodeOrText<usize>) {
        let mut inner = self.inner.borrow_mut();
        let sibling_id = inner.id_map[sibling];
        let parent_id = match inner.doc.arena.get(sibling_id).parent {
            Some(p) => p,
            None => return,
        };
        match new_node {
            NodeOrText::AppendNode(handle) => {
                let child_id = inner.id_map[&handle];
                inner
                    .doc
                    .arena
                    .insert_before(parent_id, child_id, sibling_id);
            }
            NodeOrText::AppendText(text) => {
                let prev = inner.doc.arena.get(sibling_id).prev_sibling;
                if let Some(prev_id) = prev {
                    if let NodeType::Text(ref mut existing) =
                        inner.doc.arena.get_mut(prev_id).node_type
                    {
                        existing.push_str(&text);
                        return;
                    }
                }
                let node_id = inner.doc.arena.alloc(NodeType::Text(text.to_string()));
                inner
                    .doc
                    .arena
                    .insert_before(parent_id, node_id, sibling_id);
            }
        }
    }

    fn add_attrs_if_missing(&self, target: &usize, attrs: Vec<H5Attribute>) {
        let mut inner = self.inner.borrow_mut();
        let node_id = inner.id_map[target];
        if let Some(elem) = inner.doc.arena.get_mut(node_id).as_element_mut() {
            for attr in attrs {
                let name = attr.name.local.to_string();
                if elem.get_attribute(&name).is_none() {
                    let value = attr.value.to_string();
                    if name == "id" {
                        elem.id_cache = Some(value.clone());
                    }
                    elem.attributes.push(crate::dom::Attribute {
                        name: QualName::new(name),
                        value,
                    });
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &usize) {
        let mut inner = self.inner.borrow_mut();
        let node_id = inner.id_map[target];
        inner.doc.arena.detach(node_id);
    }

    fn reparent_children(&self, node: &usize, new_parent: &usize) {
        let mut inner = self.inner.borrow_mut();
        let node_id = inner.id_map[node];
        let new_parent_id = inner.id_map[new_parent];
        let children = inner.doc.arena.children(node_id);
        for child in children {
            inner.doc.arena.append_child(new_parent_id, child);
        }
    }
}

pub fn parse_html(html: &str, url: Option<String>) -> Document {
    let sink = DomSink::new(url);
    parse_document(sink, Default::default())
        .from_utf8()
        .one(html.as_bytes())
}
