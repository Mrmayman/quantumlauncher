use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use ql_core::InstanceKind;
use serde::{Deserialize, Serialize};

mod drag_drop;
mod types;

pub use types::*;

// Since: v0.5.1
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarConfig {
    pub list: Vec<SidebarNode>,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
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

    pub fn new_folder_at(&mut self, selection: Option<SidebarSelection>, name: &str) -> FolderId {
        fn walk(
            node: &mut SidebarNode,
            selection: &SidebarSelection,
            name: &str,
        ) -> Option<FolderId> {
            let SidebarNodeKind::Folder(f) = &mut node.kind else {
                return None;
            };
            let mut index = None;
            for (i, child) in f.children.iter_mut().enumerate() {
                if child == selection {
                    index = Some(i + 1);
                    break;
                }
                if let Some(f) = walk(child, selection, name) {
                    return Some(f);
                }
            }

            let index = index?;
            let folder = SidebarNode::new_folder(Arc::from(name));
            let id = folder
                .get_folder_id()
                .expect("should be folder, not instance");
            f.children.insert(index, folder);
            Some(id)
        }

        if let Some(selection) = selection {
            for (i, child) in self.list.iter_mut().enumerate() {
                if *child == selection {
                    let folder = SidebarNode::new_folder(Arc::from(name));
                    let id = folder
                        .get_folder_id()
                        .expect("should be folder, not instance");
                    self.list.insert(i + 1, folder);
                    return id;
                }
                if let Some(f) = walk(child, &selection, name) {
                    return f;
                }
            }
        }

        let folder = SidebarNode::new_folder(Arc::from(name));
        let id = folder
            .get_folder_id()
            .expect("should be folder, not instance");
        self.list.push(folder);
        id
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

    pub fn rename(&mut self, selection: &SidebarSelection, new_name: &str) {
        fn walk(node: &mut SidebarNode, selection: &SidebarSelection, new_name: &str) -> bool {
            if node == selection {
                node.name = Arc::from(new_name);
                return true;
            }

            if let SidebarNodeKind::Folder(f) = &mut node.kind {
                for child in &mut f.children {
                    if walk(child, selection, new_name) {
                        return true;
                    }
                }
            }
            false
        }

        for child in &mut self.list {
            if walk(child, selection, new_name) {
                break;
            }
        }
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

    #[must_use]
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

    // I've tried fixing an entry duplication bug
    // (most likely caused by an incorrect `contains_instance` implementation).
    // But never can be too safe!
    pub fn fix(&mut self) {
        fn visit(n: &mut SidebarNode, visited: &mut HashSet<SidebarSelection>) {
            if let SidebarNodeKind::Folder(f) = &mut n.kind {
                f.children.retain_mut(|n| {
                    if !visited.insert(SidebarSelection::from_node(n)) {
                        return false;
                    }
                    visit(n, visited);
                    true
                });
            }
        }

        let mut visited = HashSet::new();
        self.list.retain_mut(|n| {
            if !visited.insert(SidebarSelection::from_node(n)) {
                return false;
            }
            visit(n, &mut visited);
            true
        });
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SidebarNode {
    pub name: Arc<str>,
    // icon: Option<String>
    pub kind: SidebarNodeKind,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl SidebarNode {
    #[must_use]
    fn contains_instance(&self, name: &str, instance_kind: InstanceKind) -> bool {
        match &self.kind {
            SidebarNodeKind::Instance(kind) => {
                if *kind == instance_kind && &*self.name == name {
                    return true;
                }
            }
            SidebarNodeKind::Folder(f) => {
                for child in &f.children {
                    if child.contains_instance(name, instance_kind) {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[must_use]
    fn retain_instances<F: FnMut(&SidebarNode) -> bool>(&mut self, f: &mut F) -> bool {
        if let SidebarNodeKind::Folder(folder) = &mut self.kind {
            folder.children.retain_mut(|node| node.retain_instances(f));
        } else if !f(self) {
            return false;
        }
        true
    }

    #[must_use]
    pub fn new_folder(name: Arc<str>) -> Self {
        SidebarNode {
            name,
            kind: SidebarNodeKind::Folder(SidebarFolder::default()),
            _extra: HashMap::new(),
        }
    }

    #[must_use]
    pub fn new_instance(name: Arc<str>, kind: InstanceKind) -> Self {
        SidebarNode {
            name,
            kind: SidebarNodeKind::Instance(kind),
            _extra: HashMap::new(),
        }
    }

    #[must_use]
    pub fn get_folder_id(&self) -> Option<FolderId> {
        if let SidebarNodeKind::Folder(f) = &self.kind {
            Some(f.id)
        } else {
            None
        }
    }
}
