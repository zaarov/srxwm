use x11rb::protocol::xproto::Window;

use super::Client;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    MasterStack,
    Tree,
    Horizontal,
    Vertical,
    Tabbed,
    Floating,
}

pub struct Workspace {
    clients: Vec<Client>,
    focused_client: Option<Window>,
    layout: Layout,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
            focused_client: None,
            layout: Layout::MasterStack,
        }
    }

    pub fn clients(&self) -> &[Client] {
        &self.clients
    }

    pub fn clients_mut(&mut self) -> &mut [Client] {
        &mut self.clients
    }

    pub fn focused_client(&self) -> Option<Window> {
        self.focused_client
    }

    pub fn set_focused_client(&mut self, window: Window) {
        if self.client(window).is_some_and(Client::is_mapped) {
            self.focused_client = Some(window);
        }
    }

    pub fn clear_focused_client(&mut self) {
        self.focused_client = None;
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    pub fn client(&self, window: Window) -> Option<&Client> {
        self.clients.iter().find(|client| client.window() == window)
    }

    pub fn client_mut(&mut self, window: Window) -> Option<&mut Client> {
        self.clients
            .iter_mut()
            .find(|client| client.window() == window)
    }

    pub fn add_client(&mut self, client: Client) {
        if self.client(client.window()).is_none() {
            self.clients.push(client);
        }
    }

    pub fn remove_client(&mut self, window: Window) -> Option<Client> {
        let index = self
            .clients
            .iter()
            .position(|client| client.window() == window)?;
        let removed = self.clients.remove(index);

        if self.focused_client == Some(window) {
            self.focused_client = self.clients[index.min(self.clients.len())..]
                .iter()
                .find(|client| client.is_mapped())
                .or_else(|| {
                    self.clients[..index.min(self.clients.len())]
                        .iter()
                        .rev()
                        .find(|client| client.is_mapped())
                })
                .map(Client::window);
        } else if self
            .focused_client
            .is_some_and(|focused| !self.client(focused).is_some_and(Client::is_mapped))
        {
            self.focused_client = self
                .clients
                .iter()
                .find(|client| client.is_mapped())
                .map(Client::window);
        }

        Some(removed)
    }

    pub fn take_client(&mut self, window: Window) -> Option<Client> {
        self.remove_client(window)
    }
}

