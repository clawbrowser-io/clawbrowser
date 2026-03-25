use super::arena::Arena;
use super::node::{NodeId, NodeType};

pub fn query_selector(arena: &Arena, root: NodeId, selector: &str) -> Option<NodeId> {
    let matcher = SimpleSelector::parse(selector)?;
    find_matching(arena, root, &matcher, true).into_iter().next()
}

pub fn query_selector_all(arena: &Arena, root: NodeId, selector: &str) -> Vec<NodeId> {
    let matcher = match SimpleSelector::parse(selector) {
        Some(m) => m,
        None => return Vec::new(),
    };
    find_matching(arena, root, &matcher, false)
}

fn find_matching(arena: &Arena, root: NodeId, matcher: &SimpleSelector, first_only: bool) -> Vec<NodeId> {
    let mut result = Vec::new();
    let mut stack = Vec::new();

    let mut child = arena.get(root).first_child;
    while let Some(id) = child {
        stack.push(id);
        child = arena.get(id).next_sibling;
    }
    stack.reverse();

    while let Some(id) = stack.pop() {
        if matcher.matches(arena, id) {
            result.push(id);
            if first_only {
                return result;
            }
        }
        let mut child = arena.get(id).last_child;
        while let Some(c) = child {
            stack.push(c);
            child = arena.get(c).prev_sibling;
        }
    }
    result
}

#[derive(Debug)]
enum SimpleSelector {
    Tag(String),
    Id(String),
    Class(String),
    Attribute(String),
    Universal,
    Compound(Vec<SimpleSelector>),
    DescendantCombinator(Box<SimpleSelector>, Box<SimpleSelector>),
}

impl SimpleSelector {
    fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() == 1 {
            Self::parse_simple(parts[0])
        } else if parts.len() >= 2 {
            let right = Self::parse_simple(parts.last()?)?;
            let left_str = parts[..parts.len() - 1].join(" ");
            let left = Self::parse(&left_str)?;
            Some(SimpleSelector::DescendantCombinator(
                Box::new(left),
                Box::new(right),
            ))
        } else {
            None
        }
    }

    fn parse_simple(input: &str) -> Option<Self> {
        if input == "*" {
            return Some(SimpleSelector::Universal);
        }

        let mut parts = Vec::new();
        let mut chars = input.chars().peekable();
        let mut current = String::new();
        let mut current_type: Option<char> = None;

        while let Some(&ch) = chars.peek() {
            match ch {
                '#' | '.' => {
                    if !current.is_empty() {
                        parts.push(Self::make_part(current_type, &current));
                        current.clear();
                    }
                    current_type = Some(ch);
                    chars.next();
                }
                '[' => {
                    if !current.is_empty() {
                        parts.push(Self::make_part(current_type, &current));
                        current.clear();
                    }
                    chars.next();
                    let mut attr = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            chars.next();
                            break;
                        }
                        attr.push(c);
                        chars.next();
                    }
                    parts.push(SimpleSelector::Attribute(attr));
                    current_type = None;
                }
                _ => {
                    current.push(ch);
                    chars.next();
                }
            }
        }

        if !current.is_empty() {
            parts.push(Self::make_part(current_type, &current));
        }

        match parts.len() {
            0 => None,
            1 => Some(parts.into_iter().next().unwrap()),
            _ => Some(SimpleSelector::Compound(parts)),
        }
    }

    fn make_part(kind: Option<char>, value: &str) -> Self {
        match kind {
            Some('#') => SimpleSelector::Id(value.to_string()),
            Some('.') => SimpleSelector::Class(value.to_string()),
            _ => SimpleSelector::Tag(value.to_lowercase()),
        }
    }

    fn matches(&self, arena: &Arena, node: NodeId) -> bool {
        let data = arena.get(node);
        let elem = match &data.node_type {
            NodeType::Element(e) => e,
            _ => return false,
        };

        match self {
            SimpleSelector::Universal => true,
            SimpleSelector::Tag(tag) => elem.tag_name() == *tag,
            SimpleSelector::Id(id) => elem.get_attribute("id") == Some(id.as_str()),
            SimpleSelector::Class(cls) => {
                elem.get_attribute("class")
                    .map(|c| c.split_whitespace().any(|w| w == cls.as_str()))
                    .unwrap_or(false)
            }
            SimpleSelector::Attribute(attr) => {
                if let Some((name, val)) = attr.split_once('=') {
                    let val = val.trim_matches(|c| c == '"' || c == '\'');
                    elem.get_attribute(name.trim()) == Some(val)
                } else {
                    elem.get_attribute(attr.trim()).is_some()
                }
            }
            SimpleSelector::Compound(parts) => parts.iter().all(|p| p.matches(arena, node)),
            SimpleSelector::DescendantCombinator(ancestor_sel, self_sel) => {
                if !self_sel.matches(arena, node) {
                    return false;
                }
                let mut parent = data.parent;
                while let Some(pid) = parent {
                    if ancestor_sel.matches(arena, pid) {
                        return true;
                    }
                    parent = arena.get(pid).parent;
                }
                false
            }
        }
    }
}
