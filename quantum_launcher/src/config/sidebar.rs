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

    pub fn walk_mut<F: FnMut(&mut SidebarNode) -> bool>(&mut self, mut f: F) {
        let f = &mut f;
        self.list.retain_mut(|node| node.walk_mut(f));
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SidebarNode {
    pub name: String,
    // icon: Option<String>
    pub kind: SidebarNodeKind,
    #[serde(skip)]
    pub is_being_dragged: bool,
}

impl SidebarNode {
    fn contains_instance(&self, name: &str, instance_kind: InstanceKind) -> bool {
        match &self.kind {
            SidebarNodeKind::Instance(kind) => {
                if *kind == instance_kind && self.name == name {
                    return true;
                }
            }
            SidebarNodeKind::Folder(_, children) => {
                for child in children {
                    if !child.is_folder() && child.contains_instance(name, instance_kind) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn walk_mut<F: FnMut(&mut SidebarNode) -> bool>(&mut self, f: &mut F) -> bool {
        if let SidebarNodeKind::Folder(_, list) = &mut self.kind {
            list.retain_mut(|node| node.walk_mut(f));
        } else if !f(self) {
            return false;
        }
        true
    }

    fn new_folder_at(&mut self, selection: &SidebarSelection, name: &str) -> bool {
        let SidebarNodeKind::Folder(_, children) = &mut self.kind else {
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

    pub fn is_folder(&self) -> bool {
        matches!(self.kind, SidebarNodeKind::Folder(_, _))
    }

    pub fn new_folder(name: String) -> Self {
        SidebarNode {
            name: name.to_owned(),
            kind: SidebarNodeKind::Folder(FolderId::new(), Vec::new()),
            is_being_dragged: false,
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
            SidebarSelection::Folder(name, folder_id) => {
                if let SidebarNodeKind::Folder(id, _) = &self.kind {
                    if id == folder_id {
                        return self.name == *name;
                    }
                }
            }
        }
        false
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SidebarNodeKind {
    Instance(InstanceKind),
    Folder(FolderId, Vec<SidebarNode>),
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
    Folder(String, FolderId),
}
