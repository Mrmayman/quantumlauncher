use crate::config::sidebar::{
    SDragLocation, SidebarConfig, SidebarNode, SidebarNodeKind, SidebarSelection,
};

impl SidebarConfig {
    pub fn drag_drop(&mut self, selection: &SidebarSelection, location: Option<SDragLocation>) {
        if self.is_illegal_location(selection, location.as_ref()) {
            return;
        }
        let Some(yoinked) = self.remove(selection) else {
            return;
        };

        self.insert_at(yoinked, location);
    }

    fn insert_at(&mut self, yoinked: SidebarNode, location: Option<SDragLocation>) {
        let Some(location) = location else {
            // Dragged to empty space, push at end
            self.list.push(yoinked);
            return;
        };
        if let Some((index, folder)) = self
            .list
            .iter_mut()
            .enumerate()
            .find(|(_, n)| **n == location.sel)
        {
            if let SidebarNodeKind::Folder {
                children,
                is_expanded,
                ..
            } = &mut folder.kind
            {
                if location.offset && *is_expanded && children.is_empty() {
                    children.push(yoinked);
                    return;
                }
            }
            self.list.insert(index + location.offset as usize, yoinked);
            return;
        }
        for item in &mut self.list {
            if item.insert_at(&yoinked, &location) {
                return;
            }
        }
        self.list.push(yoinked);
    }

    pub fn remove(&mut self, selection: &SidebarSelection) -> Option<SidebarNode> {
        if let Some(index) = self.list.iter().position(|n| n == selection) {
            return Some(self.list.remove(index));
        }

        for item in &mut self.list {
            if let Some(found) = item.remove(selection) {
                return Some(found);
            }
        }

        None
    }

    fn is_illegal_location(
        &self,
        selection: &SidebarSelection,
        location: Option<&SDragLocation>,
    ) -> bool {
        if let Some(location) = location {
            if let (Some(selection), Some(location)) =
                (self.get_node(selection), self.get_node(&location.sel))
            {
                if location.is_contained_by(selection) {
                    return true;
                }
            }
        }
        false
    }
}

impl SidebarNode {
    fn remove(&mut self, selection: &SidebarSelection) -> Option<SidebarNode> {
        let SidebarNodeKind::Folder { children, .. } = &mut self.kind else {
            return None;
        };
        if let Some(pos) = children.iter().position(|n| n == selection) {
            return Some(children.remove(pos));
        }
        for child in children {
            if let Some(node) = child.remove(selection) {
                return Some(node);
            }
        }
        None
    }

    pub fn insert_at(&mut self, node: &SidebarNode, location: &SDragLocation) -> bool {
        let offset = location.offset as usize;
        let SidebarNodeKind::Folder {
            children,
            id,
            is_expanded,
        } = &mut self.kind
        else {
            return false;
        };
        if let SidebarNodeKind::Folder { id: id2, .. } = &node.kind {
            if id2 == id {
                return false;
            }
        }

        if let Some((index, folder)) = children
            .iter_mut()
            .enumerate()
            .find(|(_, n)| **n == location.sel)
        {
            if let SidebarNodeKind::Folder {
                children,
                is_expanded,
                ..
            } = &mut folder.kind
            {
                if location.offset && *is_expanded && children.is_empty() {
                    children.push(node.clone());
                    return true;
                }
            }
            children.insert(index + offset, node.clone());
            *is_expanded = true;
            return true;
        }

        children.iter_mut().any(|c| c.insert_at(node, location))
    }

    pub fn is_contained_by(&self, node: &Self) -> bool {
        if self == node {
            return true;
        }
        let SidebarNodeKind::Folder { children, .. } = &node.kind else {
            return false;
        };
        children.iter().any(|c| self.is_contained_by(c))
    }
}
