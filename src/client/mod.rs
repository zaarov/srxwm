use x11rb::protocol::xproto::Window;

pub struct Client {
    window: Window,
    mapped: bool,
    focused: bool,
}

impl Client {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            mapped: false,
            focused: false,
        }
    }
    
    pub fn window(&self) -> Window {
        self.window
    }

    pub fn is_mapped(&self) -> bool {
        self.mapped
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn set_mapped(&mut self, mapped: bool) {
        self.mapped = mapped;
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
