use std::collections::HashMap;

use ql_core::{Instance, InstanceKind};
use serde::{Deserialize, Serialize};

use crate::config::sidebar::SidebarNode;

impl PartialEq<SidebarSelection> for SidebarNode {
    fn eq(&self, other: &SidebarSelection) -> bool {
        match other {
            SidebarSelection::Instance(inst) => {
                if let SidebarNodeKind::Instance(kind) = &self.kind
                    && *kind == inst.kind
                {
                    return self.name == inst.name;
                }
            }
            SidebarSelection::Folder(folder_id) => {
                if let SidebarNodeKind::Folder(f) = &self.kind {
                    return f.id == *folder_id;
                }
            }
        }
        false
    }
}

impl PartialEq<Instance> for SidebarNode {
    fn eq(&self, other: &Instance) -> bool {
        match &self.kind {
            SidebarNodeKind::Instance(kind) => {
                kind.is_server() == other.is_server() && &*self.name == other.get_name()
            }
            SidebarNodeKind::Folder(_) => false,
        }
    }
}

impl PartialEq<Instance> for SidebarSelection {
    fn eq(&self, other: &Instance) -> bool {
        match self {
            SidebarSelection::Instance(inst) => inst == other,
            SidebarSelection::Folder(_) => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub struct SidebarFolder {
    pub id: FolderId,
    pub children: Vec<SidebarNode>,
    pub is_expanded: bool,
    #[serde(flatten)]
    _extra: HashMap<String, serde_json::Value>,
}

impl Default for SidebarFolder {
    fn default() -> Self {
        Self {
            id: FolderId::new(),
            children: Vec::new(),
            is_expanded: true,
            _extra: HashMap::new(),
        }
    }
}

impl PartialEq for SidebarFolder {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub enum SidebarNodeKind {
    Instance(InstanceKind),
    Folder(SidebarFolder),
}

impl PartialEq for SidebarNodeKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Instance(l0), Self::Instance(r0)) => l0 == r0,
            (Self::Folder(l), Self::Folder(r)) => l.id == r.id,
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FolderId(u128);

impl FolderId {
    fn new() -> Self {
        Self(rand::random())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SidebarSelection {
    Instance(Instance),
    Folder(FolderId),
}

impl SidebarSelection {
    pub fn from_node(node: &SidebarNode) -> Self {
        match &node.kind {
            SidebarNodeKind::Instance(instance_kind) => Self::Instance(Instance {
                name: node.name.clone(),
                kind: *instance_kind,
            }),
            SidebarNodeKind::Folder(f) => Self::Folder(f.id),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SDragLocation {
    pub sel: SidebarSelection,
    pub offset: SDragTo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SDragTo {
    Before,
    After,
    Inside,
}
