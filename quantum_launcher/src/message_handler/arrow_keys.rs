use iced::{Task, widget::scrollable};
use ql_core::Instance;

use crate::{
    config::sidebar::{SidebarNode, SidebarNodeKind},
    state::{Launcher, Message, State},
};

pub enum InstSelectOperation {
    Down,
    Up,
    #[allow(unused)] // TODO: Add instance search using this
    Specific(Instance),
}

/// Performs a recursive traversal of the sidebar tree
/// to change the instance selection based on [`InstSelectOperation`].
///
/// The algorithm:
///
/// 1. Finds currently selected instance
/// 2. Finds next selected instance (above/below)
///
/// It also:
/// - Expands folders to reveal the next selected instance,
///   if shift key pressed
/// - Tracks indices to compute scroll position
pub struct SidebarWalker<'a> {
    selected_instance: &'a mut Option<Instance>,

    shift_pressed: bool,
    op: InstSelectOperation,

    state: WalkState,

    current_selected_idx: usize,
    next_selected_idx: usize,
    total_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WalkState {
    FindingCurrentSelected,
    FindingNextSelected,
    Found { force_expanded: bool },
}

impl<'a> SidebarWalker<'a> {
    fn new(
        selected_instance: &'a mut Option<Instance>,
        shift_pressed: bool,
        op: InstSelectOperation,
    ) -> Self {
        Self {
            selected_instance,
            shift_pressed,
            op,
            state: WalkState::FindingCurrentSelected,
            current_selected_idx: 0,
            next_selected_idx: 0,
            total_len: 0,
        }
    }

    fn walk(&mut self, nodes: &mut [SidebarNode]) {
        if nodes.is_empty() {
            return;
        }

        self.reset();
        self.walk_inner(nodes, false);

        if !matches!(self.state, WalkState::Found { .. }) && !self.shift_pressed {
            // Try again, forcing shift press
            self.shift_pressed = true;
            self.reset();
            self.walk_inner(nodes, false);
        }

        if matches!(self.op, InstSelectOperation::Up) {
            // Reverse search_idx and prev_search_idx,
            // since it iterates in reverse order (bottom up)
            self.next_selected_idx = self.total_len - self.next_selected_idx - 1;
            self.current_selected_idx = self.total_len - self.current_selected_idx - 1;
        }

        if matches!(self.state, WalkState::FindingNextSelected) {
            self.next_selected_idx = self.current_selected_idx;
        }
    }

    fn reset(&mut self) {
        self.state = WalkState::FindingCurrentSelected;
        self.next_selected_idx = 0;
        self.current_selected_idx = 0;
        self.total_len = 0;
    }

    fn walk_inner(&mut self, nodes: &mut [SidebarNode], force_expanded: bool) {
        if let InstSelectOperation::Up = self.op {
            for node in nodes.iter_mut().rev() {
                self.walk_node(node, force_expanded);
            }
        } else {
            for node in nodes {
                self.walk_node(node, force_expanded);
            }
        }
    }

    fn walk_node(&mut self, node: &mut SidebarNode, force_expanded: bool) {
        let going_up = matches!(self.op, InstSelectOperation::Up);

        match &mut node.kind {
            SidebarNodeKind::Folder(folder) => {
                self.walk_folder(going_up, folder);
            }
            SidebarNodeKind::Instance(kind) => {
                let kind = *kind;
                self.walk_instance(node, force_expanded, kind);
            }
        }
    }

    fn walk_instance(
        &mut self,
        node: &mut SidebarNode,
        force_expanded: bool,
        kind: ql_core::InstanceKind,
    ) {
        match self.state {
            WalkState::FindingCurrentSelected => {
                if let InstSelectOperation::Specific(inst) = &self.op {
                    if node.name == inst.name && kind == inst.kind {
                        self.next_selected_idx = self.total_len;
                        *self.selected_instance = Some(inst.clone());
                        self.state = WalkState::Found { force_expanded };
                    }
                } else if let Some(instance) = self.selected_instance {
                    if node.name == instance.name && kind == instance.kind {
                        self.current_selected_idx = self.total_len;
                        self.state = WalkState::FindingNextSelected;
                    }
                } else {
                    // If no instance selected, pick the first one
                    self.current_selected_idx = self.total_len;
                    self.state = WalkState::FindingNextSelected;
                }
            }
            WalkState::FindingNextSelected => {
                *self.selected_instance = Some(Instance {
                    name: node.name.clone(),
                    kind,
                });
                self.next_selected_idx = self.total_len;
                self.state = WalkState::Found { force_expanded };
            }
            WalkState::Found { .. } => {
                // Do nothing, we already found the one we want,
                // just counting total length
            }
        }
        self.total_len += 1;
    }

    fn walk_folder(&mut self, going_up: bool, folder: &mut crate::config::sidebar::SidebarFolder) {
        if !going_up {
            // Here folder is displayed above elements,
            // so if moving down, gotta count the folder first
            // before its elements
            //
            // folder/      | <-
            // - elem1      |
            // - elem2      v
            self.total_len += 1;
        }
        if matches!(self.state, WalkState::FindingCurrentSelected)
            || matches!(self.op, InstSelectOperation::Specific(_))
            || folder.is_expanded
            || self.shift_pressed
        {
            let old_state = self.state;

            let old_len = self.total_len;
            self.walk_inner(
                &mut folder.children,
                self.shift_pressed && !folder.is_expanded,
            );

            if old_state != self.state {
                // Found from this folder!
                folder.is_expanded = true;
            } else if !folder.is_expanded {
                // Don't count elements inside a closed folder
                self.total_len = old_len;
            }
        }
        if going_up {
            // If moving up, folder comes last after it's elements
            // folder/      ^ <-
            // - e1         |
            // - e2         |
            self.total_len += 1;
        }
    }
}

impl Launcher {
    pub fn select_instance_recursive(&mut self, op: InstSelectOperation) -> Task<Message> {
        let Some(sidebar) = &mut self.config.sidebar else {
            return Task::none();
        };
        let sidebar_height = {
            let State::Launch(menu) = &self.state else {
                return Task::none();
            };
            menu.sidebar_scroll.remaining
        };
        if sidebar.list.is_empty() {
            return Task::none();
        }

        let mut walker = SidebarWalker::new(
            &mut self.selected_instance,
            self.modifiers_pressed.shift(),
            op,
        );

        walker.walk(&mut sidebar.list);

        if let WalkState::Found { force_expanded } = walker.state {
            if force_expanded {
                self.on_selecting_instance()
            } else {
                let scroll_pos =
                    (walker.next_selected_idx as f32 - 0.7) / (walker.total_len as f32 - 1.0);
                let scroll_pos = scroll_pos.max(0.0) * sidebar_height;

                let scroll_task = scrollable::scroll_to(
                    scrollable::Id::new("MenuLaunch:sidebar"),
                    scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: scroll_pos,
                    },
                );

                Task::batch([scroll_task, self.on_selecting_instance()])
            }
        } else {
            Task::none()
        }
    }
}
