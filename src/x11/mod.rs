mod error;
pub use error::X11Error;

mod atoms;
use atoms::Atoms;

mod cursor;
use cursor::Cursors;

use x11rb::connection::Connection;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::Event;
use x11rb::cookie::VoidCookie;
use x11rb::protocol::xproto::{
    AtomEnum,
    ChangeWindowAttributesAux,
    ClientMessageData,
    ClientMessageEvent,
    ConfigureWindowAux,
    ConnectionExt,
    CreateWindowAux,
    EventMask,
    GetPropertyReply,
    GrabMode,
    InputFocus,
    Keycode,
    ModMask,
    PropMode,
    Rectangle,
    Screen,
    Timestamp,
    Window,
    WindowClass,
    ConfigureRequestEvent,
    ButtonIndex,
};

use xkeysym::Keysym;

pub struct XServer {
    conn: RustConnection,
    screen_num: usize,
    atoms: Atoms,
    cursors: Cursors,
}

impl XServer {
    pub fn connect() -> Result<Self, X11Error> {
        let (conn, screen_num) = x11rb::connect(None)?;
        let atoms: Atoms = Atoms::load(&conn)?;
        let cursors = Cursors::new(&conn)?;

        Ok(Self {
            conn,
            screen_num,
            atoms,
            cursors,
        })
    }

    pub fn wait_for_event(&self) -> Result<Event, X11Error> {
        Ok(self.conn.wait_for_event()?)
    }

    pub fn flush(&self) -> Result<(), X11Error> {
        self.conn.flush()?;
        Ok(())
    }

    pub fn root(&self) -> Window {
        let screen: &Screen = self.screen();
        screen.root
    }

    pub fn atoms(&self) -> &Atoms {
        &self.atoms
    }

    pub fn screen(&self) -> &Screen {
        &self.conn.setup().roots[self.screen_num]
    }

    pub fn screen_geometry(&self) -> Rectangle {
        let screen: &Screen = self.screen();

        Rectangle {
            x: 0,
            y: 0,
            width: screen.width_in_pixels,
            height: screen.height_in_pixels,
        }
    }

    pub fn claim_wm(&self) -> Result<(), X11Error> {
        let cookie: VoidCookie<'_, RustConnection> = self.conn.change_window_attributes(
            self.root(),
            &ChangeWindowAttributesAux::new().event_mask(
                EventMask::SUBSTRUCTURE_REDIRECT 
                    | EventMask::SUBSTRUCTURE_NOTIFY,
            ),
        )?;

        cookie.check()?;
        Ok(())
    }

    pub fn setup_wm_check(&self) -> Result<(), X11Error> {
        let wm_check_window: u32 = self.conn.generate_id()?;

        self.conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT as u8,
            wm_check_window,
            self.root(),
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new(),
        )?;

        let window_bytes: [u8; 4] = wm_check_window.to_ne_bytes();

        self.conn.change_property(
            PropMode::REPLACE,
            wm_check_window,
            self.atoms.NetSupportingWmCheck,
            AtomEnum::WINDOW,
            32,
            1,
            &window_bytes,
        )?;

        self.conn.change_property(
            PropMode::REPLACE,
            self.root(),
            self.atoms.NetSupportingWmCheck,
            AtomEnum::WINDOW,
            32,
            1,
            &window_bytes,
        )?;

        let name: &[u8; 5] = b"srxwm";
        
        self.conn.change_property(
            PropMode::REPLACE,
            wm_check_window,
            self.atoms.NetWmName,
            self.atoms.UTF8String,
            8,
            name.len() as u32,
            name,
        )?;

        Ok(())
    }

    pub fn setup_net_supported(&self) -> Result<(), X11Error> {
        let supported = self.atoms.net_supported();

        let data: Vec<u8> = supported
            .iter()
            .flat_map(|&atom| atom.to_ne_bytes())
            .collect();

        self.conn.change_property(
            PropMode::REPLACE,
            self.root(),
            self.atoms.NetSupported,
            AtomEnum::ATOM,
            32,
            supported.len() as u32,
            &data,
        )?;

        Ok(())
    }

    pub fn set_input_focus(&self, window: Window, timestamp: Option<Timestamp>) -> Result<(), X11Error> {
        let timestamp = match timestamp {
            Some(timestamp) => timestamp,
            None => x11rb::CURRENT_TIME,
        };
        
        self.conn.set_input_focus(
            InputFocus::PARENT,
            window,
            timestamp,
        )?;

        Ok(())
    }
    
    pub fn map_window(&self, window: Window) -> Result<(), X11Error> {
        self.conn.map_window(window)?;
        Ok(())
    }

    pub fn unmap_window(&self, window: Window) -> Result<(), X11Error> {
        self.conn.unmap_window(window)?;
        Ok(())
    }

    // Refactor this
    pub fn close_window(&self, window: Window, timestamp: u32) -> Result<(), X11Error> {
        let atoms: &Atoms = self.atoms();

        let reply: GetPropertyReply = self.conn.get_property(
            false,
            window,
            atoms.WmProtocols,
            AtomEnum::ATOM,
            0,
            64,
        )?
            .reply()?;

        let supports_delete: bool = match reply.value32() {
            Some(mut iter) => iter.any(|atom| atom == atoms.WmDeleteWindow),
            None => false,
        };

        if supports_delete {
            let event: ClientMessageEvent = ClientMessageEvent::new(
                32,
                window,
                atoms.WmProtocols,
                ClientMessageData::from([
                    atoms.WmDeleteWindow,
                    timestamp,
                    0,
                    0,
                    0,
                ]),
            );

            self.conn.send_event(
                false, 
                window, 
                EventMask::NO_EVENT, 
                event,
            )?;

        } else {
            self.conn.destroy_window(window)?;
        }

        Ok(())
    }

    pub fn configure_window(&self, window: Window, geometry: Rectangle, border: u32) -> Result<(), X11Error> {
        self.conn.configure_window(
            window,
            &ConfigureWindowAux::new()
                .x(i32::from(geometry.x))
                .y(i32::from(geometry.y))
                .width(u32::from(geometry.width))
                .height(u32::from(geometry.height))
                .border_width(border),
        )?;
        
        Ok(())
    }

    // TODO: refactor this 
    pub fn configure_window_request(&self, event: &ConfigureRequestEvent) -> Result<(), X11Error> {
        let aux = ConfigureWindowAux::from_configure_request(event);

        self.conn.configure_window(
            event.window,
            &aux,
        )?;

        Ok(())
    }
    
    pub fn set_border_color(&self, window: Window, color: u32) -> Result<(), X11Error> {
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .border_pixel(color),
        )?;

        Ok(())
    }

    pub fn grab_button(&self, window: Window, button: ButtonIndex, modifiers: ModMask) -> Result<(), X11Error> {
        self.conn.grab_button(
            false,
            window,
            EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE,
            GrabMode::ASYNC,
            GrabMode::SYNC,
            x11rb::NONE,
            x11rb::NONE,
            button,
            modifiers,
        )?;

        Ok(())
    }

    pub fn grab_pointer_move(&self) -> Result<(), X11Error> {
        self.conn.grab_pointer(
            false,
            self.root(),
            EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            x11rb::NONE,
            self.cursors.move_(),
            x11rb::CURRENT_TIME,
        )?.reply()?;

        Ok(())
    }

    pub fn ungrab_pointer(&self) -> Result<(), X11Error> {
        self.conn.ungrab_pointer(x11rb::CURRENT_TIME)?;
        Ok(())
    }

    pub fn grab_key(&self, keycode: Keycode, modifiers: ModMask) -> Result<(), X11Error> {
        let modifier_variants = [
            modifiers,
            modifiers | ModMask::LOCK,
        ];

        for modifiers in modifier_variants {
            self.conn.grab_key(
                false,
                self.root(),
                modifiers,
                keycode,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )?;
        }

        Ok(())
    }

    pub fn keycodes_for_keysym(&self, keysym: Keysym) -> Result<Vec<Keycode>, X11Error> {
        let raw: u32 = keysym.raw();

        let setup = self.conn.setup();
        let min: Keycode = setup.min_keycode;
        let count: u8 = setup.max_keycode - min + 1;

        let reply = self.conn.get_keyboard_mapping(
            min,
            count,
        )?.reply()?;

        let per: usize = reply.keysyms_per_keycode as usize;

        if per == 0 {
            return Ok(Vec::new());
        }

        let mut keycodes: Vec<Keycode> = Vec::new();

        for (index, chunk) in reply.keysyms.chunks(per).enumerate() {
            if !chunk.contains(&raw) {
                continue;
            }

            let index: u8 = match u8::try_from(index) {
                Ok(index) => index,
                Err(_) => continue,
            };

            let keycode: Keycode = match min.checked_add(index) {
                Some(keycode) => keycode,
                None => continue,
            };

            keycodes.push(keycode);
        }

        Ok(keycodes)
    }
    
    pub fn select_window_events(&self, window: Window) -> Result<(), X11Error> {
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .event_mask(
                    EventMask::ENTER_WINDOW
                        | EventMask::SUBSTRUCTURE_NOTIFY
                        | EventMask::PROPERTY_CHANGE
                ),
        )?;

        Ok(())
    }

    pub fn should_float(&self, window: Window) -> Result<bool, X11Error> {
        let reply = self.conn.get_property(
            false,
            window,
            self.atoms.NetWmWindowType,
            AtomEnum::ATOM,
            0,
            32,
        )?.reply()?;
        
        if reply.format != 32 {
            return Ok(false);
        }
        
        for chunk in reply.value.chunks_exact(4) {
            let atom = u32::from_ne_bytes([
                chunk[0],
                chunk[1],
                chunk[2],
                chunk[3],
            ]);
            
            if atom == self.atoms.NetWmWindowTypeDialog
                || atom == self.atoms.NetWmWindowTypeSplash
                || atom ==self.atoms.NetWmWindowTypeUtility
                || atom == self.atoms.NetWmWindowTypeToolbar {
                    return Ok(true);
                }
        }
        Ok(false)
    }

    pub fn is_dock_window(&self, window: Window) -> Result<bool, X11Error> {
        let reply = self.conn.get_property(
            false,
            window,
            self.atoms.NetWmWindowType,
            AtomEnum::ATOM,
            0,
            32,
        )?.reply()?;

        if reply.format != 32 {
            return Ok(false);
        }

        for chunk in reply.value.chunks_exact(4) {
            let atom = u32::from_ne_bytes([
                chunk[0],
                chunk[1],
                chunk[2],
                chunk[3],
            ]);

            if atom == self.atoms.NetWmWindowTypeDock {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn dock_strut(&self, window: Window) -> Result<u32, X11Error> {
        let reply = self.conn.get_property(
            false,
            window,
            self.atoms.NetWmStrutPartial,
            AtomEnum::CARDINAL,
            0,
            12,
        )?.reply()?;

        if reply.format != 32 {
            return Ok(0);
        }

        let mut values = match reply.value32() {
            Some(values) => values,
            None => return Ok(0),
        };

        let left = match values.next() {
            Some(value) => value,
            None => return Ok(0),
        };

        let right = match values.next() {
            Some(value) => value,
            None => return Ok(0),
        };

        let top = match values.next() {
            Some(value) => value,
            None => return Ok(0),
        };

        let bottom = match values.next() {
            Some(value) => value,
            None => return Ok(0),
        };

        let _ = (left, right, bottom);

        Ok(top)
    }

    pub fn window_geometry(&self, window: Window) -> Result<Rectangle, X11Error> {
        let reply = self.conn.get_geometry(window)?.reply()?;

        Ok(Rectangle {
            x: reply.x,
            y: reply.y,
            width: reply.width,
            height: reply.height,
        })
    }

    pub fn move_window(&self, window: Window, x: i32, y: i32) -> Result<(), X11Error> {
        self.conn.configure_window(
            window,
            &ConfigureWindowAux::new()
                .x(x)
                .y(y),
        )?;

        Ok(())
    }

    pub fn set_normal_cursor(&self, window: Window) -> Result<(), X11Error> {
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .cursor(self.cursors.normal()),
        )?;

        Ok(())
    }

    pub fn set_move_cursor(&self, window: Window) -> Result<(), X11Error> {
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .cursor(self.cursors.move_()),
        )?;

        Ok(())
    }

    pub fn set_resize_cursor(&self, window: Window) -> Result<(), X11Error> {
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .cursor(self.cursors.resize()),
        )?;

        Ok(())
    }

    pub fn setup_desktops(&self, num_workspaces: usize, workspaces_names: &[u8]) -> Result<(), X11Error> {
        let num_workspaces = num_workspaces as u32;
        self.conn.change_property(
            PropMode::REPLACE,
            self.root(),
            self.atoms.NetNumberOfDesktops,
            AtomEnum::CARDINAL,
            32,
            1,
            &num_workspaces.to_ne_bytes(),
        )?;

        self.conn.change_property(
            PropMode::REPLACE,
            self.root(),
            self.atoms.NetDesktopNames,
            self.atoms.UTF8String,
            8,
            workspaces_names.len() as u32,
            workspaces_names,
        )?;

        Ok(())
    }

    pub fn update_current_workspace(&self, current_workspace: u32) -> Result<(), X11Error> {
        self.conn.change_property(
            PropMode::REPLACE,
            self.root(),
            self.atoms.NetCurrentDesktop,
            AtomEnum::CARDINAL,
            32,
            1,
            &current_workspace.to_ne_bytes(),
        )?;

        Ok(())
    }
}
