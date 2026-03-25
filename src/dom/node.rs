use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub(crate) u32);

impl NodeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualName {
    pub ns: String,
    pub local: String,
}

impl QualName {
    pub fn new(local: impl Into<String>) -> Self {
        Self {
            ns: String::new(),
            local: local.into(),
        }
    }

    pub fn html(local: impl Into<String>) -> Self {
        Self {
            ns: "http://www.w3.org/1999/xhtml".into(),
            local: local.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: QualName,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag: QualName,
    pub attributes: Vec<Attribute>,
    pub id_cache: Option<String>,
}

impl ElementData {
    pub fn new(tag: QualName) -> Self {
        Self {
            tag,
            attributes: Vec::new(),
            id_cache: None,
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.name.local == name)
            .map(|a| a.value.as_str())
    }

    pub fn set_attribute(&mut self, name: &str, value: String) {
        if name == "id" {
            self.id_cache = Some(value.clone());
        }
        if let Some(attr) = self.attributes.iter_mut().find(|a| a.name.local == name) {
            attr.value = value;
        } else {
            self.attributes.push(Attribute {
                name: QualName::new(name),
                value,
            });
        }
    }

    pub fn remove_attribute(&mut self, name: &str) -> bool {
        if name == "id" {
            self.id_cache = None;
        }
        let len_before = self.attributes.len();
        self.attributes.retain(|a| a.name.local != name);
        self.attributes.len() != len_before
    }

    pub fn tag_name(&self) -> &str {
        &self.tag.local
    }
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Document,
    Element(ElementData),
    Text(String),
    Comment(String),
    DocType {
        name: String,
        public_id: String,
        system_id: String,
    },
}

impl NodeType {
    pub fn node_type_id(&self) -> u32 {
        match self {
            NodeType::Element(_) => 1,
            NodeType::Text(_) => 3,
            NodeType::Comment(_) => 8,
            NodeType::Document => 9,
            NodeType::DocType { .. } => 10,
        }
    }

    pub fn node_name(&self) -> String {
        match self {
            NodeType::Document => "#document".into(),
            NodeType::Element(e) => e.tag.local.to_uppercase(),
            NodeType::Text(_) => "#text".into(),
            NodeType::Comment(_) => "#comment".into(),
            NodeType::DocType { name, .. } => name.clone(),
        }
    }
}

#[derive(Debug)]
pub struct NodeData {
    pub id: NodeId,
    pub node_type: NodeType,
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,
}

impl NodeData {
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            node_type,
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
        }
    }

    pub fn as_element(&self) -> Option<&ElementData> {
        match &self.node_type {
            NodeType::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn as_element_mut(&mut self) -> Option<&mut ElementData> {
        match &mut self.node_type {
            NodeType::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn text_content(&self) -> Option<&str> {
        match &self.node_type {
            NodeType::Text(t) => Some(t),
            NodeType::Comment(c) => Some(c),
            _ => None,
        }
    }
}
