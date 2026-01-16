use std::collections::HashSet;

use serde::{Deserialize, Serialize};

mod drag_drop;
mod types;

pub use types::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarConfig {
    pub list: Vec<SidebarNode>,
}

impl SidebarConfig {
    pub fn contains_instance(&self, name: &str, instance_kind: InstanceKind) -> bool {
        for node in &self.list {
            if node.contains_instance(name, instance_kind) {
                return true;
            }
        }
        false
    }

    pub fn retain_instances<F: FnMut(&SidebarNode) -> bool>(&mut self, mut f: F) {
        let f = &mut f;
        self.list.retain_mut(|node| node.retain_instances(f));
    }

    pub fn new_folder_at(&mut self, selection: Option<SidebarSelection>, name: &str) {
        fn walk(node: &mut SidebarNode, selection: &SidebarSelection, name: &str) -> bool {
            let SidebarNodeKind::Folder(f) = &mut node.kind else {
                return false;
            };
            let mut index = None;
            for (i, child) in f.children.iter_mut().enumerate() {
                if child == selection {
                    index = Some(i + 1);
                    break;
                }
                if walk(child, selection, name) {
                    return true;
                }
            }
            let Some(index) = index else { return false };

            f.children
                .insert(index, SidebarNode::new_folder(name.to_owned()));
            true
        }

        if let Some(selection) = selection {
            for (i, child) in self.list.iter_mut().enumerate() {
                if *child == selection {
                    self.list
                        .insert(i + 1, SidebarNode::new_folder(name.to_owned()));
                    return;
                }
                if walk(child, &selection, name) {
                    return;
                }
            }
        }
        self.list.push(SidebarNode::new_folder(name.to_owned()));
    }

    pub fn delete_folder(&mut self, folder_id: FolderId) {
        fn walk(node: &mut SidebarNode, folder_id: FolderId, temp: &mut Vec<SidebarNode>) -> bool {
            if let SidebarNodeKind::Folder(f) = &mut node.kind {
                if f.id == folder_id {
                    temp.append(&mut f.children);
                    return false;
                }
                let mut temp = Vec::new();
                f.children.retain_mut(|n| walk(n, folder_id, &mut temp));
                f.children.extend(temp);
            }
            true
        }

        let mut temp = Vec::new();
        self.list.retain_mut(|n| walk(n, folder_id, &mut temp));
        self.list.extend(temp);
    }

    pub fn toggle_visibility(&mut self, id: FolderId) {
        fn walk(node: &mut SidebarNode, folder_id: FolderId) {
            if let SidebarNodeKind::Folder(f) = &mut node.kind {
                if folder_id == f.id {
                    f.is_expanded = !f.is_expanded;
                } else {
                    for child in &mut f.children {
                        walk(child, folder_id);
                    }
                }
            }
        }

        for child in &mut self.list {
            walk(child, id);
        }
    }

    pub fn get_node(&self, selection: &SidebarSelection) -> Option<&SidebarNode> {
        fn walk<'a>(
            child: &'a SidebarNode,
            selection: &SidebarSelection,
        ) -> Option<&'a SidebarNode> {
            if child == selection {
                return Some(child);
            }
            if let SidebarNodeKind::Folder(f) = &child.kind {
                for child in &f.children {
                    if let Some(sel) = walk(child, selection) {
                        return Some(sel);
                    }
                }
            }
            None
        }

        for child in &self.list {
            if let Some(node) = walk(child, selection) {
                return Some(node);
            }
        }
        None
    }

    // Look, it's 11 PM as I type this. Instances are duplicating themselves.
    // I don't know what causes it but this should sidestep the issue.
    pub fn fix(&mut self) {
        fn visit(n: &mut SidebarNode, visited: &mut HashSet<SidebarSelection>) {
            if let SidebarNodeKind::Folder(f) = &mut n.kind {
                f.children.retain_mut(|n| {
                    visit(n, visited);
                    visited.insert(SidebarSelection::from_node(n))
                });
            }
        }

        let mut visited = HashSet::new();
        self.list.retain_mut(|n| {
            visit(n, &mut visited);
            visited.insert(SidebarSelection::from_node(n))
        });
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SidebarNode {
    pub name: String,
    // icon: Option<String>
    pub kind: SidebarNodeKind,
}

impl SidebarNode {
    fn contains_instance(&self, name: &str, instance_kind: InstanceKind) -> bool {
        match &self.kind {
            SidebarNodeKind::Instance(kind) => {
                if *kind == instance_kind && self.name == name {
                    return true;
                }
            }
            SidebarNodeKind::Folder(f) => {
                for child in &f.children {
                    if !child.is_folder() && child.contains_instance(name, instance_kind) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn retain_instances<F: FnMut(&SidebarNode) -> bool>(&mut self, f: &mut F) -> bool {
        if let SidebarNodeKind::Folder(folder) = &mut self.kind {
            folder.children.retain_mut(|node| node.retain_instances(f));
        } else if !f(self) {
            return false;
        }
        true
    }

    pub fn is_folder(&self) -> bool {
        matches!(self.kind, SidebarNodeKind::Folder { .. })
    }

    pub fn new_folder(name: String) -> Self {
        SidebarNode {
            name: name.to_owned(),
            kind: SidebarNodeKind::Folder(SidebarFolder {
                id: FolderId::new(),
                children: Vec::new(),
                is_expanded: true,
            }),
        }
    }
}
