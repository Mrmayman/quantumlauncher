use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarConfig {
    pub instances: Vec<SidebarNode>,
    pub servers: Vec<SidebarNode>,
}

impl SidebarConfig {
    pub fn contains_name(&self, name: &str, is_server: bool) -> bool {
        let list = if is_server {
            &self.servers
        } else {
            &self.instances
        };

        for node in list {
            if node.contains_name(name) {
                return true;
            }
        }
        false
    }

    pub fn get_list(&self, is_server: bool) -> &Vec<SidebarNode> {
        if is_server {
            &self.servers
        } else {
            &self.instances
        }
    }

    pub fn get_list_mut(&mut self, is_server: bool) -> &mut Vec<SidebarNode> {
        if is_server {
            &mut self.servers
        } else {
            &mut self.instances
        }
    }

    pub fn walk_mut<F: FnMut(&mut SidebarNode) -> bool>(&mut self, is_server: bool, mut f: F) {
        let list = self.get_list_mut(is_server);
        let f = &mut f;
        list.retain_mut(|node| node.walk_mut(f));
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
    fn contains_name(&self, name: &str) -> bool {
        if self.name == name {
            return true;
        }
        if let SidebarNodeKind::Folder(children) = &self.kind {
            for child in children {
                if child.contains_name(name) {
                    return true;
                }
            }
        }
        false
    }

    fn walk_mut<F: FnMut(&mut SidebarNode) -> bool>(&mut self, f: &mut F) -> bool {
        if !f(self) {
            return false;
        }
        if let SidebarNodeKind::Folder(list) = &mut self.kind {
            list.retain_mut(|node| node.walk_mut(f));
        }
        true
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SidebarNodeKind {
    Instance,
    Folder(Vec<SidebarNode>),
}
