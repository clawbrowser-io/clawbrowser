use super::arena::Arena;
use super::node::{ElementData, NodeId, NodeType, QualName};

pub struct Document {
    pub arena: Arena,
    pub document_node: NodeId,
    pub url: Option<String>,
}

impl Document {
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let doc = arena.alloc(NodeType::Document);
        Self {
            arena,
            document_node: doc,
            url: None,
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        let mut doc = Self::new();
        doc.url = Some(url.into());
        doc
    }

    pub fn create_element(&mut self, tag: &str) -> NodeId {
        self.arena
            .alloc(NodeType::Element(ElementData::new(QualName::html(
                tag.to_lowercase(),
            ))))
    }

    pub fn create_text_node(&mut self, text: &str) -> NodeId {
        self.arena.alloc(NodeType::Text(text.to_string()))
    }

    pub fn create_comment(&mut self, text: &str) -> NodeId {
        self.arena.alloc(NodeType::Comment(text.to_string()))
    }

    pub fn document_element(&self) -> Option<NodeId> {
        let mut child = self.arena.get(self.document_node).first_child;
        while let Some(id) = child {
            if matches!(self.arena.get(id).node_type, NodeType::Element(_)) {
                return Some(id);
            }
            child = self.arena.get(id).next_sibling;
        }
        None
    }

    pub fn body(&self) -> Option<NodeId> {
        let html = self.document_element()?;
        let mut child = self.arena.get(html).first_child;
        while let Some(id) = child {
            if let NodeType::Element(e) = &self.arena.get(id).node_type {
                if e.tag_name() == "body" {
                    return Some(id);
                }
            }
            child = self.arena.get(id).next_sibling;
        }
        None
    }

    pub fn head(&self) -> Option<NodeId> {
        let html = self.document_element()?;
        let mut child = self.arena.get(html).first_child;
        while let Some(id) = child {
            if let NodeType::Element(e) = &self.arena.get(id).node_type {
                if e.tag_name() == "head" {
                    return Some(id);
                }
            }
            child = self.arena.get(id).next_sibling;
        }
        None
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<NodeId> {
        for node in self.arena.iter() {
            if let Some(elem) = node.as_element() {
                if elem.id_cache.as_deref() == Some(id) {
                    return Some(node.id);
                }
                if elem.get_attribute("id") == Some(id) {
                    return Some(node.id);
                }
            }
        }
        None
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<NodeId> {
        let tag_lower = tag.to_lowercase();
        let mut result = Vec::new();
        for node in self.arena.iter() {
            if let Some(elem) = node.as_element() {
                if tag == "*" || elem.tag_name() == tag_lower {
                    result.push(node.id);
                }
            }
        }
        result
    }

    pub fn get_elements_by_class_name(&self, class_name: &str) -> Vec<NodeId> {
        let target_classes: Vec<&str> = class_name.split_whitespace().collect();
        if target_classes.is_empty() {
            return Vec::new();
        }
        let mut result = Vec::new();
        for node in self.arena.iter() {
            if let Some(elem) = node.as_element() {
                if let Some(cls) = elem.get_attribute("class") {
                    let node_classes: Vec<&str> = cls.split_whitespace().collect();
                    if target_classes
                        .iter()
                        .all(|tc| node_classes.contains(tc))
                    {
                        result.push(node.id);
                    }
                }
            }
        }
        result
    }

    pub fn title(&self) -> Option<String> {
        let head = self.head()?;
        let mut child = self.arena.get(head).first_child;
        while let Some(id) = child {
            if let NodeType::Element(e) = &self.arena.get(id).node_type {
                if e.tag_name() == "title" {
                    return Some(self.arena.deep_text_content(id));
                }
            }
            child = self.arena.get(id).next_sibling;
        }
        None
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
