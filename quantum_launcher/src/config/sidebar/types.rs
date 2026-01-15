use ql_core::InstanceSelection;
use serde::{Deserialize, Serialize};

use crate::config::sidebar::SidebarNode;

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

impl PartialEq<InstanceSelection> for SidebarSelection {
    fn eq(&self, other: &InstanceSelection) -> bool {
        match self {
            SidebarSelection::Instance(name, instance_kind) => {
                instance_kind.is_server() == other.is_server() && name == other.get_name()
            }
            SidebarSelection::Folder(_) => false,
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

impl PartialEq for SidebarNodeKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Instance(l0), Self::Instance(r0)) => l0 == r0,
            (Self::Folder { id: l_id, .. }, Self::Folder { id: r_id, .. }) => l_id == r_id,
            _ => false,
        }
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct SDragLocation {
    pub sel: SidebarSelection,
    pub offset: bool,
}
