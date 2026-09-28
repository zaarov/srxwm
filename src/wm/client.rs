use x11rb::protocol::xproto::{Rectangle, Window};

pub struct Client {
    window: Window,
    geometry: Rectangle,
    mapped: bool,
    floating: bool,
    fullscreen: bool,
}

impl Client {
    pub fn new(window: Window, geometry: Rectangle) -> Self {
        Self { 
            window,
            geometry,
            mapped: false, 
            floating: false, 
            fullscreen: false, 
        }
    }

    pub fn window(&self) -> Window {
        self.window
    }

    pub fn geometry(&self) -> Rectangle {
        self.geometry
    }

    pub fn is_mapped(&self) -> bool {
        self.mapped
    }

    pub fn is_floating(&self) -> bool {
        self.floating
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn set_geometry(&mut self, geometry: Rectangle) {
        self.geometry = geometry;
    }

    pub fn set_mapped(&mut self, mapped: bool) {
        self.mapped = mapped;
    }

    pub fn set_floating(&mut self, floating: bool) {
        self.floating = floating;
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.fullscreen = fullscreen;
    }
}
