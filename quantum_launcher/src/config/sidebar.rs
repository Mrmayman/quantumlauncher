use ql_core::InstanceSelection;
use serde::{Deserialize, Serialize};

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
        if let Some(selection) = selection {
            for (i, child) in self.list.iter_mut().enumerate() {
                if *child == selection {
                    self.list
                        .insert(i + 1, SidebarNode::new_folder(name.to_owned()));
                    return;
                }
                if child.new_folder_at(&selection, name) {
                    return;
                }
            }
        }
        self.list.push(SidebarNode::new_folder(name.to_owned()));
    }

    pub fn toggle_visibility(&mut self, id: FolderId) {
        for child in &mut self.list {
            child.toggle_visibility(id);
        }
    }

    pub fn get_node_from_selection(&self, selection: &SidebarSelection) -> Option<&SidebarNode> {
        for child in &self.list {
            if let Some(node) = child.get_from_selection(selection) {
                return Some(node);
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
            SidebarNodeKind::Folder { children, .. } => {
                for child in children {
                    if !child.is_folder() && child.contains_instance(name, instance_kind) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn retain_instances<F: FnMut(&SidebarNode) -> bool>(&mut self, f: &mut F) -> bool {
        if let SidebarNodeKind::Folder { children, .. } = &mut self.kind {
            children.retain_mut(|node| node.retain_instances(f));
        } else if !f(self) {
            return false;
        }
        true
    }

    fn new_folder_at(&mut self, selection: &SidebarSelection, name: &str) -> bool {
        let SidebarNodeKind::Folder { children, .. } = &mut self.kind else {
            return false;
        };
        let mut index = None;
        for (i, child) in children.iter_mut().enumerate() {
            if child == selection {
                index = Some(i + 1);
                break;
            }
            if child.new_folder_at(selection, name) {
                return true;
            }
        }
        let Some(index) = index else { return false };

        children.insert(index, Self::new_folder(name.to_owned()));
        true
    }

    fn toggle_visibility(&mut self, folder_id: FolderId) {
        if let SidebarNodeKind::Folder {
            id,
            children,
            is_expanded,
        } = &mut self.kind
        {
            if folder_id == *id {
                *is_expanded = !*is_expanded;
            } else {
                for child in children {
                    child.toggle_visibility(folder_id);
                }
            }
        }
    }

    fn get_from_selection(&self, selection: &SidebarSelection) -> Option<&Self> {
        if self == selection {
            return Some(self);
        }
        if let SidebarNodeKind::Folder { children, .. } = &self.kind {
            for child in children {
                if let Some(sel) = child.get_from_selection(selection) {
                    return Some(sel);
                }
            }
        }
        None
    }
}

impl SidebarNode {
    pub fn is_folder(&self) -> bool {
        matches!(self.kind, SidebarNodeKind::Folder { .. })
    }

    pub fn new_folder(name: String) -> Self {
        SidebarNode {
            name: name.to_owned(),
            kind: SidebarNodeKind::Folder {
                id: FolderId::new(),
                children: Vec::new(),
                is_expanded: true,
            },
        }
    }
}

impl PartialEq<SidebarSelection> for SidebarNode {
    fn eq(&self, other: &SidebarSelection) -> bool {
        match other {
            SidebarSelection::Instance(name, instance_kind) => {
                if let SidebarNodeKind::Instance(kind) = &self.kind {
                    if kind == instance_kind {
                        return self.name == *name;
                    }
                }
            }
            SidebarSelection::Folder(folder_id) => {
                if let SidebarNodeKind::Folder { id, .. } = &self.kind {
                    return id == folder_id;
                }
            }
        }
        false
    }
}

impl PartialEq<InstanceSelection> for SidebarNode {
    fn eq(&self, other: &InstanceSelection) -> bool {
        match &self.kind {
            SidebarNodeKind::Instance(kind) => {
                kind.is_server() == other.is_server() && self.name == other.get_name()
            }
            SidebarNodeKind::Folder { .. } => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SidebarNodeKind {
    Instance(InstanceKind),
    Folder {
        id: FolderId,
        children: Vec<SidebarNode>,
        is_expanded: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct FolderId(usize);

impl FolderId {
    pub fn new() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as usize,
        )
    }
}

// TODO: Refactor the entire launcher to use this
// instead of `is_server: bool`
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceKind {
    Client,
    Server,
}

impl InstanceKind {
    pub fn is_server(self) -> bool {
        matches!(self, Self::Server)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarSelection {
    Instance(String, InstanceKind),
    Folder(FolderId),
}

impl SidebarSelection {
    pub fn from_node(node: &SidebarNode) -> Self {
        match node.kind {
            SidebarNodeKind::Instance(instance_kind) => {
                Self::Instance(node.name.clone(), instance_kind)
            }
            SidebarNodeKind::Folder { id, .. } => Self::Folder(id),
        }
    }
}
