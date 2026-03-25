use super::node::{NodeData, NodeId, NodeType};

#[derive(Debug)]
pub struct Arena {
    nodes: Vec<NodeData>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(256),
        }
    }

    pub fn alloc(&mut self, node_type: NodeType) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(NodeData::new(id, node_type));
        id
    }

    pub fn get(&self, id: NodeId) -> &NodeData {
        &self.nodes[id.index()]
    }

    pub fn get_mut(&mut self, id: NodeId) -> &mut NodeData {
        &mut self.nodes[id.index()]
    }

    pub fn try_get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id.index())
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &NodeData> {
        self.nodes.iter()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}
