// TODO: add multiple monitor support

use x11rb::protocol::xproto::Rectangle;

use super::Workspace;

pub struct Monitor {
    geometry: Rectangle,
    workspaces: Vec<Workspace>,
    focused_workspace: usize,
}

impl Monitor {
    pub fn new(geometry: Rectangle, workspace_count: usize) -> Self {
        let workspaces = (0..workspace_count)
            .map(|_| Workspace::new())
            .collect();

        Self {
            geometry,
            workspaces,
            focused_workspace: 0,
        }
    }

    pub fn focused_workspace(&self) -> &Workspace {
        &self.workspaces[self.focused_workspace]
    }

    pub fn focused_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.focused_workspace]
    }
}
