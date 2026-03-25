use super::arena::Arena;
use super::node::{NodeId, NodeType};

impl Arena {
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        self.detach(child);

        self.get_mut(child).parent = Some(parent);

        let last_child = self.get(parent).last_child;
        if let Some(last) = last_child {
            self.get_mut(last).next_sibling = Some(child);
            self.get_mut(child).prev_sibling = Some(last);
        } else {
            self.get_mut(parent).first_child = Some(child);
        }
        self.get_mut(parent).last_child = Some(child);
    }

    pub fn insert_before(&mut self, parent: NodeId, new_child: NodeId, ref_child: NodeId) {
        self.detach(new_child);

        let prev = self.get(ref_child).prev_sibling;
        self.get_mut(new_child).parent = Some(parent);
        self.get_mut(new_child).next_sibling = Some(ref_child);
        self.get_mut(new_child).prev_sibling = prev;
        self.get_mut(ref_child).prev_sibling = Some(new_child);

        if let Some(prev_id) = prev {
            self.get_mut(prev_id).next_sibling = Some(new_child);
        } else {
            self.get_mut(parent).first_child = Some(new_child);
        }
    }

    pub fn detach(&mut self, node: NodeId) {
        let parent = self.get(node).parent;
        let prev = self.get(node).prev_sibling;
        let next = self.get(node).next_sibling;

        if let Some(prev_id) = prev {
            self.get_mut(prev_id).next_sibling = next;
        } else if let Some(parent_id) = parent {
            self.get_mut(parent_id).first_child = next;
        }

        if let Some(next_id) = next {
            self.get_mut(next_id).prev_sibling = prev;
        } else if let Some(parent_id) = parent {
            self.get_mut(parent_id).last_child = prev;
        }

        let n = self.get_mut(node);
        n.parent = None;
        n.prev_sibling = None;
        n.next_sibling = None;
    }

    pub fn children(&self, parent: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut cur = self.get(parent).first_child;
        while let Some(id) = cur {
            result.push(id);
            cur = self.get(id).next_sibling;
        }
        result
    }

    pub fn child_elements(&self, parent: NodeId) -> Vec<NodeId> {
        self.children(parent)
            .into_iter()
            .filter(|&id| matches!(self.get(id).node_type, NodeType::Element(_)))
            .collect()
    }

    pub fn descendants(&self, root: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            result.push(id);
            let mut child = self.get(id).last_child;
            while let Some(c) = child {
                stack.push(c);
                child = self.get(c).prev_sibling;
            }
        }
        result
    }

    pub fn remove_all_children(&mut self, parent: NodeId) {
        let children = self.children(parent);
        for child in children {
            self.detach(child);
        }
    }

    pub fn deep_text_content(&self, node: NodeId) -> String {
        let mut result = String::new();
        self.collect_text(node, &mut result);
        result
    }

    fn collect_text(&self, node: NodeId, buf: &mut String) {
        match &self.get(node).node_type {
            NodeType::Text(t) => buf.push_str(t),
            _ => {
                let mut child = self.get(node).first_child;
                while let Some(id) = child {
                    self.collect_text(id, buf);
                    child = self.get(id).next_sibling;
                }
            }
        }
    }

    pub fn clone_node(&mut self, node: NodeId, deep: bool) -> NodeId {
        let node_type = self.get(node).node_type.clone();
        let new_id = self.alloc(node_type);
        if deep {
            let children = self.children(node);
            for child in children {
                let cloned_child = self.clone_node(child, true);
                self.append_child(new_id, cloned_child);
            }
        }
        new_id
    }
}
