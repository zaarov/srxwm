use x11rb::protocol::xproto::Window;

use super::Client;


pub enum Layout {
    MasterStack,
    Tree,
    Horizontal,
    Vertical,
    Tabbed,
    Floating
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
        if self.client(window).is_some() {
            self.focused_client = Some(window);
        }
    }

    pub fn clear_focused_client(&mut self) {
        self.focused_client = None;
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn client(&self, window: Window) -> Option<&Client> {
        self.clients
            .iter()
            .find(|client| client.window() == window)
    }

    pub fn client_mut(&mut self, window: Window) -> Option<&mut Client> {
        self.clients
            .iter_mut()
            .find(|client| client.window() == window)
    }
    
    pub fn add_client(&mut self, client: Client) {
        self.clients.push(client);
    }

    pub fn remove_client(&mut self, window: Window) {
        self.clients.retain(|client| client.window() != window);

        if self.focused_client == Some(window) {
            self.focused_client = self.clients.last().map(|client| client.window());
        }
    }

    pub fn take_client(&mut self, window: Window) -> Option<Client> {
        let index = match self
            .clients
            .iter()
            .position(|client| client.window() == window)
        {
            Some(index) => index,
            None => return None,
        };

        let client = self.clients.remove(index);

        if self.focused_client == Some(window) {
            self.clear_focused_client();
        }

        Some(client)
    }
}
