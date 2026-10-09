mod error;
mod client;
mod workspace;

// TODO: add multiple monitor support
// mod monitor;

pub use error::WmError;
pub use client::Client;
pub use workspace::{
    Workspace,
    Layout,
};

// TODO: add multiple monitor support
// pub use monitor::Monitor;

use std::process::Command;

use crate::x11::XServer;
use crate::config::{
    Config,
    Action,
    Direction,
};

use x11rb::protocol::xproto::{
    ButtonPressEvent,
    KeyPressEvent,
    UnmapNotifyEvent,
    EnterNotifyEvent,
    MotionNotifyEvent,
    Rectangle,
    Timestamp,
    Window,
    ConfigureRequestEvent,
    ConfigWindow,
    ModMask,
    KeyButMask,
    NotifyMode,
    NotifyDetail,
    ButtonIndex,
};
use x11rb::protocol::Event;

const WORKSPACE_COUNT: usize = 9;

const WORKSPACE_NAMES: &[u8] = &[
    b'1', 0,
    b'2', 0,
    b'3', 0,
    b'4', 0,
    b'5', 0,
    b'6', 0,
    b'7', 0,
    b'8', 0,
    b'9', 0,
];

struct MoveState {
    window: Window,
    pointer_x: i16,
    pointer_y: i16,
    geometry: Rectangle,
}

pub struct WindowManager {
    xserver: XServer,
    workspaces: Vec<Workspace>,
    focused_workspace: usize,
    config: Config,
    dock_window: Option<Window>,
    reserved_top: u32,
    ignore_enter_notify: bool,
    moving: Option<MoveState>,
    // TODO: add multiple monitor support
    // monitors: Vec<Monitor>,
    // focused_monitor: usize,
}

impl WindowManager {
    pub fn new() -> Result<Self, WmError> {
        let xserver: XServer = XServer::connect()?;
        let mut workspaces: Vec<Workspace> = Vec::with_capacity(WORKSPACE_COUNT);

        for _ in 0..WORKSPACE_COUNT {
            workspaces.push(Workspace::new());
        }
        
        let wm: WindowManager = Self {
            xserver,
            workspaces,
            focused_workspace: 0,
            config: Config::default(),
            dock_window: None,
            reserved_top: 0,
            ignore_enter_notify: false,
            moving: None,

            // TODO: add multiple monitor support
            // monitors: Vec::new(),
            // focused_monitor: 0,
        };

        wm.xserver.claim_wm()?;
        wm.xserver.setup_wm_check()?;
        wm.xserver.setup_net_supported()?;
        wm.xserver.setup_desktops(
            WORKSPACE_COUNT,
            WORKSPACE_NAMES,
        )?;
        wm.xserver.set_normal_cursor(wm.xserver.root())?;
        
        Self::setup_startup_commands(&wm.config)?;
        Self::setup_keybindings(&wm)?;
        
        Ok(wm)
    }
    
    fn setup_startup_commands(config: &Config) -> Result<(), WmError> {
        if let Some(commands) = config.startup_commands() {
            for command in commands {
                let mut process: Command = std::process::Command::new(
                    command.program()
                );

                if let Some(args) = command.args() {
                    process.args(args);
                }
            }
        }

        Ok(())
    }

    fn setup_keybindings(wm: &WindowManager) -> Result<(), WmError> {
        for binding in wm.config.keybindings() {
            let keycodes = wm
                .xserver
                .keycodes_for_keysym(binding.key())?;

            for keycode in keycodes {
                wm.xserver.grab_key(
                    keycode,
                    binding.modifiers(),
                )?;
            }
        }

        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), WmError> {
        loop {
            let event: Event = self.xserver.wait_for_event()?;
            self.hdl_event(event)?;
        }
    }

    fn hdl_event(&mut self, event: Event) -> Result<(), WmError> {
        let sent_event = event.sent_event();
        
        match event {
            Event::MapRequest(e) => self.hdl_map_request(e.window)?,
            Event::UnmapNotify(e) => self.hdl_unmap_notify(e, sent_event)?,
            Event::DestroyNotify(e) => self.hdl_destroy_notify(e.window)?,
            Event::ConfigureRequest(e) => self.hdl_configure_request(e)?,
            Event::EnterNotify(e) => self.hdl_enter_notify(e)?,
            Event::KeyPress(e) => self.hdl_key_press(e)?,
            Event::ButtonPress(e) => self.hdl_button_press(e)?,
            Event::MotionNotify(e) => self.hdl_motion_notify(e)?,
            Event::ButtonRelease(e) => self.hdl_button_release(e)?,
            _ => {},
        }
        
        self.xserver.flush()?;
        
        Ok(())
    }

    fn hdl_map_request(&mut self, window: Window) -> Result<(), WmError> {
        if self.focused_workspace().client(window).is_some() {
            return Ok(());
        }

        self.xserver.select_window_events(window)?;

        if self.xserver.is_dock_window(window)? {
            self.manage_dock(window)?;
            return Ok(());
        }

        let geometry = self.xserver.window_geometry(window)?;
        let floating = self.xserver.should_float(window)?;

        let mut client = Client::new(window, geometry);
        client.set_floating(floating);

        if floating {
            self.xserver.grab_button(
                window,
                ButtonIndex::M1,
                ModMask::M1,
            )?;
        }

        self.focused_workspace_mut().add_client(client);

        self.arrange()?;

        self.xserver.map_window(window)?;
        self.focus_client(window, None)?;

        Ok(())
    }
    
    fn hdl_unmap_notify(&mut self, event: UnmapNotifyEvent, sent_event: bool) -> Result<(), WmError> {
        if sent_event {
            return Ok(());
        }

        let was_focused =
            self.focused_workspace().focused_client() == Some(event.window);

        self.focused_workspace_mut()
            .remove_client(event.window);

        self.arrange()?;

        if was_focused {
            self.focused_workspace_mut()
                .clear_focused_client();

            if let Some(next) = self
                .focused_workspace()
                .clients()
                .first()
                .map(Client::window)
            {
                self.focus_client(next, None)?;
            }
        }

        Ok(())
    }
    
    fn hdl_destroy_notify(&mut self, window: Window) -> Result<(), WmError> {
        let was_focused =
            self.focused_workspace().focused_client() == Some(window);

        self.focused_workspace_mut().remove_client(window);

        self.arrange()?;

        if was_focused {
            self.focused_workspace_mut()
                .clear_focused_client();

            if let Some(next) = self
                .focused_workspace()
                .clients()
                .iter()
                .find(|client| client.is_mapped())
                .map(Client::window)
            {
                self.focus_client(next, None)?;
            }
        }

        Ok(())
    }

    // TODO: refactor this
    fn hdl_configure_request(&mut self, event: ConfigureRequestEvent) -> Result<(), WmError> {
        let floating = match self.focused_workspace().client(event.window) {
            Some(client) => client.is_floating(),
            None => {
                self.xserver.configure_window_request(&event)?;
                return Ok(());
            }
        };

        if !floating {
            self.arrange()?;
            return Ok(());
        }

        let mut geometry = match self.focused_workspace().client(event.window) {
            Some(client) => client.geometry(),
            None => return Ok(()),
        };

        if event.value_mask.contains(ConfigWindow::X) {
            geometry.x = event.x;
        }

        if event.value_mask.contains(ConfigWindow::Y) {
            geometry.y = event.y;
        }

        if event.value_mask.contains(ConfigWindow::WIDTH) {
            geometry.width = event.width;
        }

        if event.value_mask.contains(ConfigWindow::HEIGHT) {
            geometry.height = event.height;
        }

        self.set_client_geometry(event.window, geometry);

        self.xserver.configure_window(
            event.window,
            geometry,
            self.config.border_width(),
        )?;

        Ok(())
    }
    
    fn hdl_enter_notify(&mut self, event: EnterNotifyEvent) -> Result<(), WmError> {
        if self.ignore_enter_notify {
            self.ignore_enter_notify = false;
            return Ok(());
        }

        if event.mode != NotifyMode::NORMAL {
            return Ok(());
        }

        if event.detail == NotifyDetail::INFERIOR {
            return Ok(());
        }

        self.focus_client(event.event, Some(event.time))?;

        Ok(())
    }

    fn hdl_key_press(&mut self, event: KeyPressEvent) -> Result<(), WmError> {
        let event_modifiers = event.state.bits() & !u16::from(ModMask::LOCK);

        for binding in self.config.keybindings() {
            let binding_modifiers = u16::from(binding.modifiers());

            if event_modifiers != binding_modifiers {
                continue;
            }

            let keycodes = self.xserver.keycodes_for_keysym(binding.key())?;

            if !keycodes.contains(&event.detail) {
                continue;
            }

            match binding.action() {
                Action::Spawn(command) => {
                    Command::new(command.program())
                        .args(command.args().unwrap_or(&[]))
                        .spawn()?;
                }

                Action::CloseWindow => {
                    if let Some(window) = self.focused_workspace().focused_client() {
                        self.xserver.close_window(window, event.time)?;
                    }
                }

                Action::FocusWindow(direction) => {
                    self.focus_direction(*direction)?;
                }

                Action::SwapWindow(direction) => {
                    self.swap_direction(*direction)?;
                }

                Action::MoveToWorkspace(workspace) => {
                    self.move_to_workspace(*workspace)?;
                }

                Action::SwitchWorkspace(workspace) => {
                    self.switch_workspace(*workspace)?;
                }
            }

            break;
        }

        Ok(())
    }

    fn hdl_button_press(&mut self, event: ButtonPressEvent) -> Result<(), WmError> {
        if event.detail != u8::from(ButtonIndex::M1) {
            return Ok(());
        }

        if !event.state.contains(KeyButMask::MOD1) {
            return Ok(());
        }

        let window = event.event;

        let geometry = match self.focused_workspace().client(window) {
            Some(client) => {
                if !client.is_floating() {
                    return Ok(());
                }

                client.geometry()
            }
            None => return Ok(()),
        };

        self.focus_client(window, Some(event.time))?;

        self.moving = Some(MoveState {
            window,
            pointer_x: event.root_x,
            pointer_y: event.root_y,
            geometry,
        });

        self.xserver.grab_pointer_move()?;

        Ok(())
    }
    
    fn hdl_motion_notify(&mut self, event: MotionNotifyEvent) -> Result<(), WmError> {
        let (window, pointer_x, pointer_y, geometry) =
            match self.moving.as_ref() {
                Some(state) => (
                    state.window,
                    state.pointer_x,
                    state.pointer_y,
                    state.geometry,
                ),
                None => return Ok(()),
            };

        let delta_x =
            i32::from(event.root_x) - i32::from(pointer_x);

        let delta_y =
            i32::from(event.root_y) - i32::from(pointer_y);

        let geometry = Rectangle {
            x: (i32::from(geometry.x) + delta_x) as i16,
            y: (i32::from(geometry.y) + delta_y) as i16,
            width: geometry.width,
            height: geometry.height,
        };

        self.set_client_geometry(window, geometry);

        self.xserver.configure_window(
            window,
            geometry,
            self.config.border_width(),
        )?;

        Ok(())
    }
    
    fn hdl_button_release(&mut self, event: ButtonPressEvent) -> Result<(), WmError> {
        if event.detail != u8::from(ButtonIndex::M1) {
            return Ok(());
        }

        if self.moving.is_none() {
            return Ok(());
        }

        self.moving = None;

        self.xserver.ungrab_pointer()?;

        Ok(())
    }
    
    fn arrange(&mut self) -> Result<(), WmError> {
        let layout = self.workspaces[self.focused_workspace].layout();

        match layout {
            Layout::MasterStack => self.arrange_master()?,
            Layout::Tree => self.arrange_tree()?,
            Layout::Horizontal => self.arrange_hor()?,
            Layout::Vertical => self.arrange_vert()?,
            Layout::Tabbed => self.arrange_tab()?,
            Layout::Floating => self.arrange_float()?,
        }

        Ok(())
    }

    // Recheck this logic later
    fn arrange_master(&mut self) -> Result<(), WmError> {
        let (tiled_windows, floating_windows) = {
            let clients: &[Client] = self.focused_workspace().clients();

            let mut tiled_windows: Vec<Window> = Vec::new();
            let mut floating_windows: Vec<Window> = Vec::new();

            for client in clients {   
                if client.is_floating() {
                    floating_windows.push(client.window());
                } else {
                    tiled_windows.push(client.window());
                }
            }

            (tiled_windows, floating_windows)
        };

        for window in floating_windows {
            self.arrange_float_win(window)?;
        }

        if tiled_windows.is_empty() {
            return Ok(());
        }

        let screen: Rectangle = self.xserver.screen_geometry();

        let gap: u32 = self.config.gaps();
        let border: u32 = self.config.border_width();

        let screen_x: i32 = i32::from(screen.x);
        let screen_y: i32 = i32::from(screen.y) + self.reserved_top as i32;
        let screen_width: u32 = u32::from(screen.width);
        let screen_height: u32 = u32::from(screen.height)
            .saturating_sub(self.reserved_top);

        let clients_count: usize = tiled_windows.len();

        if clients_count == 1 {
            let window = tiled_windows[0];

            let x = screen_x + gap as i32;
            let y = screen_y + gap as i32;

            let width = screen_width
                .saturating_sub(gap.saturating_mul(2))
                .saturating_sub(border.saturating_mul(2));

            let height = screen_height
                .saturating_sub(gap.saturating_mul(2))
                .saturating_sub(border.saturating_mul(2));

            let geometry = Rectangle {
                x: x as i16,
                y: y as i16,
                width: width as u16,
                height: height as u16,
            };

            self.set_client_geometry(window, geometry);

            self.xserver.configure_window(
                window,
                geometry,
                border,
            )?;

            return Ok(());
        }

        let outer_width = screen_width
            .saturating_sub(gap.saturating_mul(2));

        let outer_height = screen_height
            .saturating_sub(gap.saturating_mul(2));

        let horizontal_overhead = gap
            .saturating_add(border.saturating_mul(4));

        let usable_width = outer_width
            .saturating_sub(horizontal_overhead);

        let master_width = usable_width
            .saturating_mul(self.config.master_ratio())
            / 100;

        let stack_width = usable_width
            .saturating_sub(master_width);

        let master_x = screen_x + gap as i32;
        let master_y = screen_y + gap as i32;

        let master_height = outer_height
            .saturating_sub(border.saturating_mul(2));

        let master_geometry = Rectangle {
            x: master_x as i16,
            y: master_y as i16,
            width: master_width as u16,
            height: master_height as u16,
        };

        let master_window = tiled_windows[0];

        let mut geometries: Vec<(Window, Rectangle)> =
            Vec::with_capacity(clients_count);

        geometries.push((master_window, master_geometry));

        let stack_count = clients_count - 1;

        let stack_gap_total = gap
            .saturating_mul((stack_count - 1) as u32);

        let stack_border_total = border
            .saturating_mul(2)
            .saturating_mul(stack_count as u32);

        let stack_content_height = outer_height
            .saturating_sub(stack_gap_total)
            .saturating_sub(stack_border_total);

        let base_height = stack_content_height
            / stack_count as u32;

        let remainder = stack_content_height
            % stack_count as u32;

        let stack_x = master_x
            + master_width as i32
            + border.saturating_mul(2) as i32
            + gap as i32;

        for index in 0..stack_count {
            let window = tiled_windows[index + 1];

            let y = master_y
                + index as i32
                * (base_height
                   + border.saturating_mul(2)
                   + gap) as i32;

            let height = if index == stack_count - 1 {
                base_height + remainder
            } else {
                base_height
            };

            let geometry = Rectangle {
                x: stack_x as i16,
                y: y as i16,
                width: stack_width as u16,
                height: height as u16,
            };

            geometries.push((window, geometry));
        }

        {
            let workspace = self.focused_workspace_mut();

            for &(window, geometry) in &geometries {
                if let Some(client) = workspace.client_mut(window) {
                    client.set_geometry(geometry);
                }
            }
        }

        for &(window, geometry) in &geometries {
            self.xserver.configure_window(
                window,
                geometry,
                border,
            )?;
        }

        Ok(())
    }
    
    fn arrange_tree(&self) -> Result<(), WmError> {
        todo!("impl tree tiling algorithm")
    }

    fn arrange_hor(&self) -> Result<(), WmError> {
        todo!("impl horizontal tiling algorithm")
    }

    fn arrange_vert(&self) -> Result<(), WmError> {
        todo!("impl vertical tiling algorithm")
    }

    fn arrange_tab(&self) -> Result<(), WmError> {
        todo!("impl tabs tiling algorithm")
    }

    fn arrange_float(&self) -> Result<(), WmError> {
        todo!("impl floating workspace state")
    }

    // Recheck this logic later
    fn arrange_float_win(&mut self, window: Window) -> Result<(), WmError> {
        let geometry = match self.focused_workspace().client(window) {
            Some(client) => client.geometry(),
            None => return Ok(()),
        };

        self.xserver.configure_window(
            window,
            geometry,
            self.config.border_width(),
        )?;

        Ok(())
    }

    // HELPERS
    fn focused_workspace(&self) -> &Workspace {
        &self.workspaces[self.focused_workspace]
    }

    fn focused_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.focused_workspace]
    }

    fn set_client_geometry(&mut self, window: Window, geometry: Rectangle) {
        if let Some(client) = self.focused_workspace_mut().client_mut(window) {
            client.set_geometry(geometry);
        }
    }

    fn focus_client(&mut self, window: Window, time: Option<Timestamp>) -> Result<(), WmError> {
        if self.focused_workspace().client(window).is_none() {
            return Ok(());
        }

        let previous = self.focused_workspace().focused_client();

        if previous != Some(window) {
            if let Some(previous) = previous {
                self.xserver.set_border_color(
                    previous,
                    self.config.unfocused_border_color(),
                )?;
            }

            self.focused_workspace_mut()
                .set_focused_client(window);
        }

        self.xserver.set_border_color(
            window,
            self.config.focused_border_color(),
        )?;

        self.xserver.set_input_focus(window, time)?;

        Ok(())
    }

    fn switch_workspace(&mut self, workspace: usize) -> Result<(), WmError> {
        if workspace >= self.workspaces.len() {
            return Ok(());
        }

        if workspace == self.focused_workspace {
            return Ok(());
        }

        let old_workspace = self.focused_workspace;

        match self.workspaces[old_workspace].focused_client() {
            Some(window) => {
                self.xserver.set_border_color(
                    window,
                    self.config.unfocused_border_color(),
                )?;
            }
            None => {}
        }

        self.hide_workspace(old_workspace)?;

        self.focused_workspace = workspace;

        self.show_workspace(workspace)?;
        
        self.arrange()?;

        self.xserver.update_current_workspace(workspace as u32)?;
        
        match self.workspaces[workspace].focused_client() {
            Some(window) => {
                self.focus_client(window, None)?;
            }

            None => {
                let window = match self.workspaces[workspace].clients().first() {
                    Some(client) => client.window(),
                    None => {
                        self.xserver.set_input_focus(
                            self.xserver.root(),
                            None,
                        )?;

                        return Ok(());
                    }
                };

                self.focus_client(window, None)?;
            }
        }

        Ok(())
    }
    
    fn hide_workspace(&self, workspace: usize) -> Result<(), WmError> {
        let screen = self.xserver.screen_geometry();
        let offscreen_x = -(i32::from(screen.width) * 2);

        for client in self.workspaces[workspace].clients() {
            self.xserver.move_window(
                client.window(),
                offscreen_x,
                i32::from(client.geometry().y),
            )?;
        }

        Ok(())
    }

    fn show_workspace(&self, workspace: usize) -> Result<(), WmError> {
        for client in self.workspaces[workspace].clients() {
            let geometry = client.geometry();

            self.xserver.move_window(
                client.window(),
                i32::from(geometry.x),
                i32::from(geometry.y),
            )?;
        }

        Ok(())
    }

    fn manage_dock(&mut self, window: Window) -> Result<(), WmError> {
        let strut = self.xserver.dock_strut(window)?;

        self.dock_window = Some(window);
        self.reserved_top = strut;

        self.xserver.map_window(window)?;
        self.arrange()?;

        Ok(())
    }

    fn find_direction_target(&self, direction: Direction) -> Option<Window> {
        let current_window = match self.focused_workspace().focused_client() {
            Some(window) => window,
            None => return None,
        };

        let current_geometry = match self
            .focused_workspace()
            .client(current_window)
        {
            Some(client) => client.geometry(),
            None => return None,
        };

        let current_left = i32::from(current_geometry.x);
        let current_top = i32::from(current_geometry.y);

        let current_right =
            current_left + i32::from(current_geometry.width);

        let current_bottom =
            current_top + i32::from(current_geometry.height);

        let current_center_x =
            current_left + i32::from(current_geometry.width) / 2;

        let current_center_y =
            current_top + i32::from(current_geometry.height) / 2;

        let master_window = match self.focused_workspace().layout() {
            Layout::MasterStack => {
                let mut master = None;

                for client in self.focused_workspace().clients() {
                    if client.is_floating() {
                        continue;
                    }

                    master = Some(client.window());
                    break;
                }

                master
            }

            _ => None,
        };

        if master_window == Some(current_window) {
            match direction {
                Direction::Up | Direction::Down => {
                    return None;
                }

                Direction::Left | Direction::Right => {
                    let mut target = None;
                    let mut target_top = i32::MAX;

                    for client in self.focused_workspace().clients() {
                        if client.is_floating() {
                            continue;
                        }

                        let window = client.window();

                        if Some(window) == master_window {
                            continue;
                        }

                        let geometry = client.geometry();
                        let top = i32::from(geometry.y);

                        if top < target_top {
                            target_top = top;
                            target = Some(window);
                        }
                    }

                    return target;
                }
            }
        }

        let mut target = None;

        let mut best_primary_distance = i32::MAX;
        let mut best_overlap = 0;
        let mut best_secondary_distance = i32::MAX;
        let mut best_position = i32::MAX;

        for client in self.focused_workspace().clients() {
            if client.is_floating() {
                continue;
            }

            let window = client.window();

            if window == current_window {
                continue;
            }

            let geometry = client.geometry();

            let left = i32::from(geometry.x);
            let top = i32::from(geometry.y);

            let right =
                left + i32::from(geometry.width);

            let bottom =
                top + i32::from(geometry.height);

            let center_x =
                left + i32::from(geometry.width) / 2;

            let center_y =
                top + i32::from(geometry.height) / 2;

            let primary_distance;
            let overlap;
            let secondary_distance;
            let position;

            match direction {
                Direction::Left => {
                    if right > current_left {
                        continue;
                    }

                    primary_distance = current_left - right;

                    overlap = Self::range_overlap(
                        top,
                        bottom,
                        current_top,
                        current_bottom,
                    );

                    secondary_distance =
                        (current_center_y - center_y).abs();

                    position = top;
                }

                Direction::Right => {
                    if left < current_right {
                        continue;
                    }

                    primary_distance = left - current_right;

                    overlap = Self::range_overlap(
                        top,
                        bottom,
                        current_top,
                        current_bottom,
                    );

                    secondary_distance =
                        (current_center_y - center_y).abs();

                    position = top;
                }

                Direction::Up => {
                    if bottom > current_top {
                        continue;
                    }

                    primary_distance = current_top - bottom;

                    overlap = Self::range_overlap(
                        left,
                        right,
                        current_left,
                        current_right,
                    );

                    secondary_distance =
                        (current_center_x - center_x).abs();

                    position = left;
                }

                Direction::Down => {
                    if top < current_bottom {
                        continue;
                    }

                    primary_distance = top - current_bottom;

                    overlap = Self::range_overlap(
                        left,
                        right,
                        current_left,
                        current_right,
                    );

                    secondary_distance =
                        (current_center_x - center_x).abs();

                    position = left;
                }
            }

            let better = if target.is_none() {
                true
            } else if primary_distance < best_primary_distance {
                true
            } else if primary_distance > best_primary_distance {
                false
            } else if overlap > best_overlap {
                true
            } else if overlap < best_overlap {
                false
            } else if secondary_distance < best_secondary_distance {
                true
            } else if secondary_distance > best_secondary_distance {
                false
            } else {
                position < best_position
            };

            if better {
                target = Some(window);
                best_primary_distance = primary_distance;
                best_overlap = overlap;
                best_secondary_distance = secondary_distance;
                best_position = position;
            }
        }

        if target.is_none() {
            let screen = self.xserver.screen_geometry();

            let screen_left = i32::from(screen.x);
            let screen_top = i32::from(screen.y);

            let screen_right =
                screen_left + i32::from(screen.width);

            let screen_bottom =
                screen_top + i32::from(screen.height);

            let mut best_edge_distance = i32::MAX;
            let mut best_secondary_distance = i32::MAX;
            let mut best_position = i32::MAX;

            for client in self.focused_workspace().clients() {
                if client.is_floating() {
                    continue;
                }

                let window = client.window();

                if window == current_window {
                    continue;
                }

                let geometry = client.geometry();

                let left = i32::from(geometry.x);
                let top = i32::from(geometry.y);

                let right =
                    left + i32::from(geometry.width);

                let bottom =
                    top + i32::from(geometry.height);

                let center_x =
                    left + i32::from(geometry.width) / 2;

                let center_y =
                    top + i32::from(geometry.height) / 2;

                let edge_distance;
                let secondary_distance;
                let position;

                match direction {
                    Direction::Left => {
                        edge_distance = screen_right - right;

                        secondary_distance =
                            (current_center_y - center_y).abs();

                        position = right;
                    }

                    Direction::Right => {
                        edge_distance = left - screen_left;

                        secondary_distance =
                            (current_center_y - center_y).abs();

                        position = left;
                    }

                    Direction::Up => {
                        edge_distance = screen_bottom - bottom;

                        secondary_distance =
                            (current_center_x - center_x).abs();

                        position = bottom;
                    }

                    Direction::Down => {
                        edge_distance = top - screen_top;

                        secondary_distance =
                            (current_center_x - center_x).abs();

                        position = top;
                    }
                }

                let better = if target.is_none() {
                    true
                } else if edge_distance < best_edge_distance {
                    true
                } else if edge_distance > best_edge_distance {
                    false
                } else if secondary_distance < best_secondary_distance {
                    true
                } else if secondary_distance > best_secondary_distance {
                    false
                } else {
                    position < best_position
                };

                if better {
                    target = Some(window);
                    best_edge_distance = edge_distance;
                    best_secondary_distance = secondary_distance;
                    best_position = position;
                }
            }
        }

        target
    }

    fn range_overlap(first_start: i32, first_end: i32, second_start: i32, second_end: i32) -> i32 {
        let start = first_start.max(second_start);
        let end = first_end.min(second_end);

        if end > start {
            end - start
        } else {
            0
        }
    }

    fn focus_direction(&mut self, direction: Direction) -> Result<(), WmError> {
        let target = self.find_direction_target(direction);

        match target {
            Some(window) => {
                self.focus_client(window, None)?;
            }
            None => {}
        }

        Ok(())
    }

    fn swap_direction(&mut self, direction: Direction) -> Result<(), WmError> {
        let current_window = match self.focused_workspace().focused_client() {
            Some(window) => window,
            None => return Ok(()),
        };

        let target_window = match self.find_direction_target(direction) {
            Some(window) => window,
            None => return Ok(()),
        };

        if current_window == target_window {
            return Ok(());
        }

        {
            let workspace = self.focused_workspace_mut();

            let current_index = match workspace
                .clients()
                .iter()
                .position(|client| client.window() == current_window)
            {
                Some(index) => index,
                None => return Ok(()),
            };

            let target_index = match workspace
                .clients()
                .iter()
                .position(|client| client.window() == target_window)
            {
                Some(index) => index,
                None => return Ok(()),
            };

            workspace
                .clients_mut()
                .swap(current_index, target_index);
        }

        self.ignore_enter_notify = true;

        self.arrange()?;

        self.focus_client(current_window, None)?;

        Ok(())
    }

    fn move_to_workspace(&mut self, workspace: usize) -> Result<(), WmError> {
        if workspace >= self.workspaces.len() {
            return Ok(());
        }

        let current_workspace = self.focused_workspace;

        if workspace == current_workspace {
            return Ok(());
        }

        let window = match self
            .focused_workspace()
            .focused_client()
        {
            Some(window) => window,
            None => return Ok(()),
        };

        let client = match self
            .focused_workspace_mut()
            .take_client(window)
        {
            Some(client) => client,
            None => return Ok(()),
        };

        self.workspaces[workspace].add_client(client);

        self.hide_window(window)?;

        self.arrange()?;

        let next_window = match self
            .focused_workspace()
            .clients()
            .first()
        {
            Some(client) => client.window(),
            None => {
                self.xserver.set_input_focus(
                    self.xserver.root(),
                    None,
                )?;
                
                return Ok(());
            }
        };
        
        self.focus_client(next_window, None)?;
        
        Ok(())
    }

    fn hide_window(&self, window: Window) -> Result<(), WmError> {
        let screen = self.xserver.screen_geometry();

        let x = -(i32::from(screen.width) * 2);

        self.xserver.move_window(
            window,
            x,
            0,
        )?;

        Ok(())
    }
}
