mod client;
mod error;
mod layout;
mod workarea;
mod workspace;

pub use client::Client;
pub use error::WmError;
pub use workspace::{Layout, Workspace};

use std::cmp::Reverse;
use std::collections::HashMap;
use std::process::Command as ProcessCommand;

use crate::config::{Action, Config, Direction};
use crate::x11::{DockStrut, XServer};
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    ButtonIndex,
    ButtonPressEvent,
    ButtonReleaseEvent,
    ClientMessageEvent,
    ConfigWindow,
    ConfigureRequestEvent,
    EnterNotifyEvent,
    KeyPressEvent,
    ModMask,
    MotionNotifyEvent,
    NotifyDetail,
    NotifyMode,
    PropertyNotifyEvent,
    Rectangle,
    Timestamp,
    Window,
};

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

#[derive(Clone, Copy)]
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
    docks: HashMap<Window, DockStrut>,
    moving: Option<MoveState>,
    ignore_enter_notify: bool,
    keycodes_by_keysym: HashMap<u32, Vec<u8>>,
    grabbed_key_combinations: Vec<(u8, ModMask)>,
}

impl WindowManager {
    pub fn new() -> Result<Self, WmError> {
        let xserver = XServer::connect()?;
        let workspaces = (0..WORKSPACE_COUNT).map(|_| Workspace::new()).collect();
        let config = Config::default();

        let mut wm = Self {
            xserver,
            workspaces,
            focused_workspace: 0,
            config,
            docks: HashMap::new(),
            moving: None,
            ignore_enter_notify: false,
            keycodes_by_keysym: HashMap::new(),
            grabbed_key_combinations: Vec::new(),
        };

        wm.xserver.claim_wm()?;
        wm.xserver.setup_wm_check()?;
        wm.xserver.setup_net_supported()?;
        wm.xserver
            .setup_desktops(WORKSPACE_COUNT, WORKSPACE_NAMES)?;
        wm.xserver.update_current_workspace(0)?;
        wm.xserver.set_normal_cursor(wm.xserver.root())?;
        wm.setup_keybindings()?;
        wm.update_workarea_property()?;
        Self::setup_startup_commands(&wm.config)?;

        Ok(wm)
    }

    fn setup_startup_commands(config: &Config) -> Result<(), WmError> {
        if let Some(commands) = config.startup_commands() {
            for command in commands {
                ProcessCommand::new(command.program())
                    .args(command.args().unwrap_or(&[]))
                    .spawn()?;
            }
        }
        Ok(())
    }

    fn setup_keybindings(&mut self) -> Result<(), WmError> {
        for (keycode, modifiers) in std::mem::take(&mut self.grabbed_key_combinations) {
            self.xserver.ungrab_key(keycode, modifiers)?;
        }

        let mut keycodes_by_keysym: HashMap<u32, Vec<u8>> = HashMap::new();
        let mut grabbed = std::collections::HashSet::new();
        let mut grabbed_combinations = Vec::new();

        for binding in self.config.keybindings() {
            let key = binding.key();
            let keysym = key.raw();
            let modifiers = binding.modifiers();

            if !keycodes_by_keysym.contains_key(&keysym) {
                let keycodes = self.xserver.keycodes_for_keysym(key)?;
                keycodes_by_keysym.insert(keysym, keycodes);
            }

            let keycodes = keycodes_by_keysym
                .get(&keysym)
                .expect("keycodes are inserted before being read");
            for &keycode in keycodes {
                if grabbed.insert((keycode, modifiers.bits())) {
                    self.xserver.grab_key(keycode, modifiers)?;
                    grabbed_combinations.push((keycode, modifiers));
                }
            }
        }

        self.keycodes_by_keysym = keycodes_by_keysym;
        self.grabbed_key_combinations = grabbed_combinations;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), WmError> {
        loop {
            let event = self.xserver.wait_for_event()?;
            self.handle_event(event)?;
        }
    }

    fn handle_event(&mut self, event: Event) -> Result<(), WmError> {
        let sent_event = event.sent_event();
        match event {
            Event::MapRequest(event) => self.handle_map_request(event.window)?,
            Event::UnmapNotify(event) => self.handle_unmap_notify(event.window, sent_event)?,
            Event::DestroyNotify(event) => self.handle_destroy_notify(event.window)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::EnterNotify(event) => self.handle_enter_notify(event)?,
            Event::PropertyNotify(event) => self.handle_property_notify(event)?,
            Event::MappingNotify(_) => self.setup_keybindings()?,
            Event::KeyPress(event) => self.handle_key_press(event)?,
            Event::ButtonPress(event) => self.handle_button_press(event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::ClientMessage(event) => self.hdl_client_message(event)?,
            _ => {}
        }
        self.xserver.flush()?;
        Ok(())
    }

    fn handle_map_request(&mut self, window: Window) -> Result<(), WmError> {
        if self.workspace_index_for(window).is_some() {
            return Ok(());
        }

        self.xserver.select_window_events(window)?;
        if self.xserver.is_dock_window(window)? {
            return self.manage_dock(window);
        }

        let geometry = self.xserver.window_geometry(window)?;
        let floating = self.xserver.should_float(window)?;
        let mut client = Client::new(window, geometry);
        client.set_floating(floating);
        self.focused_workspace_mut().add_client(client);

        if floating {
            self.xserver
                .grab_button(window, ButtonIndex::M1, self.config.mod_key())?;
        }

        self.xserver.map_window(window)?;
        if let Some(client) = self.focused_workspace_mut().client_mut(window) {
            client.set_mapped(true);
        }
        self.arrange()?;
        self.focus_client(window, None)?;
        Ok(())
    }

    fn handle_unmap_notify(&mut self, window: Window, sent_event: bool) -> Result<(), WmError> {
        if sent_event {
            return Ok(());
        }
        self.remove_window(window)
    }

    fn handle_destroy_notify(&mut self, window: Window) -> Result<(), WmError> {
        self.remove_window(window)
    }

    fn remove_window(&mut self, window: Window) -> Result<(), WmError> {
        if self.moving.is_some_and(|moving| moving.window == window) {
            self.cancel_move()?;
        }

        if self.docks.remove(&window).is_some() {
            self.update_workarea_property()?;
            self.arrange()?;
            return Ok(());
        }

        let Some(workspace_index) = self.workspace_index_for(window) else {
            return Ok(());
        };
        
        let was_focused = self.workspaces[workspace_index].focused_client() == Some(window);
        self.workspaces[workspace_index].take_client(window);

        if workspace_index != self.focused_workspace {
            return Ok(());
        }

        self.arrange()?;
        if was_focused {
            self.focus_fallback(None)?;
        }
        Ok(())
    }

    fn handle_configure_request(&mut self, event: ConfigureRequestEvent) -> Result<(), WmError> {
        let Some(workspace_index) = self.workspace_index_for(event.window) else {
            self.xserver.configure_window_request(&event)?;
            return Ok(());
        };

        let Some(client) = self.workspaces[workspace_index].client(event.window) else {
            return Ok(());
        };

        if !client.is_floating() {
            if workspace_index == self.focused_workspace {
                self.arrange()?;
            }
            return Ok(());
        }

        let mut geometry = client.geometry();

        if event.value_mask.contains(ConfigWindow::X) {
            geometry.x = event.x;
        }

        if event.value_mask.contains(ConfigWindow::Y) {
            geometry.y = event.y;
        }

        if event.value_mask.contains(ConfigWindow::WIDTH) {
            geometry.width = event.width.max(1);
        }

        if event.value_mask.contains(ConfigWindow::HEIGHT) {
            geometry.height = event.height.max(1);
        }

        if let Some(client) = self.workspaces[workspace_index].client_mut(event.window) {
            client.set_geometry(geometry);
        }

        if workspace_index == self.focused_workspace {
            self.xserver
                .configure_window(event.window, geometry, self.config.border_width())?;
        }
        Ok(())
    }

    fn handle_property_notify(&mut self, event: PropertyNotifyEvent) -> Result<(), WmError> {
        if !self.docks.contains_key(&event.window) {
            return Ok(());
        }

        let atoms = self.xserver.atoms();
        if event.atom != atoms.NetWmStrutPartial && event.atom != atoms.NetWmStrut {
            return Ok(());
        }

        let strut = self.xserver.dock_strut(event.window)?;
        self.docks.insert(event.window, strut);
        self.update_workarea_property()?;
        self.arrange()?;
        Ok(())
    }

    fn handle_enter_notify(&mut self, event: EnterNotifyEvent) -> Result<(), WmError> {
        if self.ignore_enter_notify {
            self.ignore_enter_notify = false;
            return Ok(());
        }

        if event.mode != NotifyMode::NORMAL || event.detail == NotifyDetail::INFERIOR {
            return Ok(());
        }

        self.focus_client(event.event, Some(event.time))
    }

    fn handle_key_press(&mut self, event: KeyPressEvent) -> Result<(), WmError> {
        let state = event.state.bits() & !x11rb::protocol::xproto::ModMask::LOCK.bits();

        for binding in self.config.keybindings() {
            if state != binding.modifiers().bits() {
                continue;
            }

            let keysym = binding.key().raw();
            let matches_keycode = self
                .keycodes_by_keysym
                .get(&keysym)
                .is_some_and(|keycodes| keycodes.contains(&event.detail));
            if !matches_keycode {
                continue;
            }

            match binding.action() {
                Action::Spawn(command) => {
                    ProcessCommand::new(command.program())
                        .args(command.args().unwrap_or(&[]))
                        .spawn()?;
                }
                Action::CloseWindow => {
                    if let Some(window) = self.focused_workspace().focused_client() {
                        self.xserver.close_window(window, event.time)?;
                    }
                }
                Action::FocusWindow(direction) => self.focus_direction(*direction)?,
                Action::SwapWindow(direction) => self.swap_direction(*direction)?,
                Action::MoveToWorkspace(workspace) => self.move_to_workspace(*workspace)?,
                Action::SwitchWorkspace(workspace) => self.switch_workspace(*workspace)?,
                Action::ToggleFullscreen => self.toggle_fullscreen()?,
            }
            break;
        }

        Ok(())
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) -> Result<(), WmError> {
        if event.detail != u8::from(ButtonIndex::M1) {
            return Ok(());
        }
        let configured_modifier = self.config.mod_key().bits();
        if event.state.bits() & configured_modifier != configured_modifier {
            return Ok(());
        }

        let window = event.event;
        let Some(client) = self.focused_workspace().client(window) else {
            return Ok(());
        };
        if !client.is_floating() || !client.is_mapped() {
            return Ok(());
        }
        let geometry = client.geometry();

        self.focus_client(window, Some(event.time))?;
        self.xserver.grab_pointer_move()?;
        self.moving = Some(MoveState {
            window,
            pointer_x: event.root_x,
            pointer_y: event.root_y,
            geometry,
        });

        Ok(())
    }

    fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> Result<(), WmError> {
        let Some(state) = self.moving else {
            return Ok(());
        };
        let dx = i32::from(event.root_x) - i32::from(state.pointer_x);
        let dy = i32::from(event.root_y) - i32::from(state.pointer_y);
        let geometry = Rectangle {
            x: clamp_i16(i32::from(state.geometry.x) + dx),
            y: clamp_i16(i32::from(state.geometry.y) + dy),
            width: state.geometry.width.max(1),
            height: state.geometry.height.max(1),
        };

        if let Some(client) = self.focused_workspace_mut().client_mut(state.window) {
            client.set_geometry(geometry);
        } else {
            return Ok(());
        }
        self.xserver
            .configure_window(state.window, geometry, self.config.border_width())?;
        Ok(())
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> Result<(), WmError> {
        if event.detail != u8::from(ButtonIndex::M1) || self.moving.is_none() {
            return Ok(());
        }
        self.moving = None;
        self.xserver.ungrab_pointer()?;
        Ok(())
    }

    fn arrange(&mut self) -> Result<(), WmError> {
        let placements = layout::calculate(
            self.focused_workspace().layout(),
            self.focused_workspace().clients(),
            self.work_area(),
            self.config.gaps(),
            self.config.border_width(),
            self.config.master_ratio(),
        );

        for &(window, geometry) in &placements {
            if let Some(client) = self.focused_workspace_mut().client_mut(window) {
                client.set_geometry(geometry);
            }
        }

        let fullscreen_window = self
            .focused_workspace()
            .clients()
            .iter()
            .find(|client| client.is_mapped() && client.is_fullscreen())
            .map(Client::window);

        for (window, geometry) in placements {
            if Some(window) == fullscreen_window {
                continue;
            }

            self.xserver
                .configure_window(window, geometry, self.config.border_width())?;
        }

        if let Some(window) = fullscreen_window {
            let geometry = self.xserver.screen_geometry();

            self.xserver.configure_window(window, geometry, 0)?;
            self.xserver.raise_window(window)?;
        }

        Ok(())
    }

    fn hdl_client_message(&mut self, event: ClientMessageEvent) -> Result<(), WmError> {
        if event.type_ != self.xserver.atoms().NetWmState {
            return Ok(());
        }

        let data = event.data.as_data32();
        let fullscreen_atom = self.xserver.atoms().NetWmStateFullscreen;

        if data[1] != fullscreen_atom && data[2] != fullscreen_atom {
            return Ok(());
        }

        let current_state = match self
            .workspaces
            .iter()
            .find_map(|workspace| workspace.client(event.window).map(Client::is_fullscreen))
        {
            Some(state) => state,
            None => return Ok(()),
        };

        let fullscreen = match data[0] {
            0 => false,
            1 => true,
            2 => !current_state,
            _ => return Ok(()),
        };

        self.set_client_fullscreen(event.window, fullscreen)
    }

    fn focus_client(&mut self, window: Window, time: Option<Timestamp>) -> Result<(), WmError> {
        let Some(client) = self.focused_workspace().client(window) else {
            return Ok(());
        };
        if !client.is_mapped() {
            return Ok(());
        }
        let should_raise =
            client.is_floating() || self.focused_workspace().layout() == Layout::Tabbed;
        let previous = self.focused_workspace().focused_client();

        if previous != Some(window) {
            if let Some(previous) = previous.filter(|previous| {
                self.focused_workspace()
                    .client(*previous)
                    .is_some_and(Client::is_mapped)
            }) {
                self.xserver
                    .set_border_color(previous, self.config.unfocused_border_color())?;
            }
            self.focused_workspace_mut().set_focused_client(window);
        }

        self.xserver
            .set_border_color(window, self.config.focused_border_color())?;
        if should_raise {
            self.xserver.raise_window(window)?;
        }
        self.xserver.set_input_focus(window, time)?;
        Ok(())
    }

    fn focus_fallback(&mut self, time: Option<Timestamp>) -> Result<(), WmError> {
        let next = self
            .focused_workspace()
            .focused_client()
            .filter(|window| {
                self.focused_workspace()
                    .client(*window)
                    .is_some_and(Client::is_mapped)
            })
            .or_else(|| {
                self.focused_workspace()
                    .clients()
                    .iter()
                    .find(|client| client.is_mapped())
                    .map(Client::window)
            });

        if let Some(window) = next {
            self.focus_client(window, time)
        } else {
            self.focused_workspace_mut().clear_focused_client();
            self.xserver.set_input_focus(self.xserver.root(), time)?;
            Ok(())
        }
    }

    fn switch_workspace(&mut self, workspace: usize) -> Result<(), WmError> {
        if workspace >= self.workspaces.len() || workspace == self.focused_workspace {
            return Ok(());
        }
        self.cancel_move()?;

        let old_workspace = self.focused_workspace;
        if let Some(window) = self.workspaces[old_workspace].focused_client() {
            self.xserver
                .set_border_color(window, self.config.unfocused_border_color())?;
        }
        self.hide_workspace(old_workspace)?;
        self.focused_workspace = workspace;
        self.show_workspace(workspace)?;
        self.arrange()?;
        self.xserver.update_current_workspace(workspace as u32)?;
        self.focus_fallback(None)
    }

    fn hide_workspace(&self, workspace: usize) -> Result<(), WmError> {
        let screen = self.xserver.screen_geometry();
        let offscreen_x = -i32::from(screen.width).saturating_mul(2);
        for client in self.workspaces[workspace]
            .clients()
            .iter()
            .filter(|client| client.is_mapped())
        {
            self.xserver.move_window(
                client.window(),
                offscreen_x,
                i32::from(client.geometry().y),
            )?;
        }
        Ok(())
    }

    fn show_workspace(&self, workspace: usize) -> Result<(), WmError> {
        for client in self.workspaces[workspace]
            .clients()
            .iter()
            .filter(|client| client.is_mapped())
        {
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
        self.docks.insert(window, strut);
        self.xserver.map_window(window)?;
        self.update_workarea_property()?;
        self.arrange()?;
        Ok(())
    }

    fn work_area(&self) -> Rectangle {
        workarea::calculate(self.xserver.screen_geometry(), self.docks.values().copied())
    }

    fn update_workarea_property(&self) -> Result<(), WmError> {
        self.xserver
            .update_workarea(self.work_area(), WORKSPACE_COUNT)?;
        Ok(())
    }

    fn workspace_index_for(&self, window: Window) -> Option<usize> {
        self.workspaces
            .iter()
            .position(|workspace| workspace.client(window).is_some())
    }

    fn focused_workspace(&self) -> &Workspace {
        &self.workspaces[self.focused_workspace]
    }

    fn focused_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.focused_workspace]
    }

    fn find_direction_target(
        &self,
        direction: Direction,
        include_floating: bool,
    ) -> Option<Window> {
        let workspace = self.focused_workspace();
        let current_window = workspace.focused_client()?;
        let current = workspace.client(current_window)?;

        if !current.is_mapped() || (!include_floating && current.is_floating()) {
            return None;
        }

        let current_geometry = current.geometry();

        if workspace.layout() == Layout::Tabbed {
            let eligible = |client: &&Client| {
                client.is_mapped() && (include_floating || !client.is_floating())
            };

            let mut count = 0;
            let mut current_index = None;

            for client in workspace.clients().iter().filter(eligible) {
                if client.window() == current_window {
                    current_index = Some(count);
                }

                count += 1;
            }

            let index = current_index?;

            if count < 2 {
                return None;
            }

            let next = match direction {
                Direction::Left | Direction::Up => (index + count - 1) % count,
                Direction::Right | Direction::Down => (index + 1) % count,
            };

            return workspace
                .clients()
                .iter()
                .filter(eligible)
                .nth(next)
                .map(Client::window);
        }

        if workspace.layout() == Layout::MasterStack && !current.is_floating() {
            return self.find_master_stack_target(current_window, direction);
        }

        let candidates = workspace.clients().iter().filter(|client| {
            client.window() != current_window
                && client.is_mapped()
                && (include_floating || !client.is_floating())
        });

        let best_in_direction = candidates
            .clone()
            .filter(|candidate| is_in_direction(current_geometry, candidate.geometry(), direction))
            .min_by_key(|candidate| {
                direction_score(
                    current_geometry,
                    candidate.geometry(),
                    candidate.window(),
                    direction,
                )
            });

        if let Some(candidate) = best_in_direction {
            return Some(candidate.window());
        }

        candidates
            .min_by_key(|candidate| wrap_score(candidate.geometry(), candidate.window(), direction))
            .map(Client::window)
    }

    fn focus_direction(&mut self, direction: Direction) -> Result<(), WmError> {
        if let Some(window) = self.find_direction_target(direction, true) {
            self.focus_client(window, None)?;
        }
        Ok(())
    }

    fn swap_direction(&mut self, direction: Direction) -> Result<(), WmError> {
        let Some(current_window) = self.focused_workspace().focused_client() else {
            return Ok(());
        };
        let Some(target_window) = self.find_direction_target(direction, false) else {
            return Ok(());
        };
        if current_window == target_window {
            return Ok(());
        }

        let workspace = self.focused_workspace_mut();
        let Some(current_index) = workspace
            .clients()
            .iter()
            .position(|client| client.window() == current_window)
        else {
            return Ok(());
        };
        let Some(target_index) = workspace
            .clients()
            .iter()
            .position(|client| client.window() == target_window)
        else {
            return Ok(());
        };
        workspace.clients_mut().swap(current_index, target_index);

        self.ignore_enter_notify = true;
        self.arrange()?;
        self.focus_client(current_window, None)?;
        Ok(())
    }

    fn move_to_workspace(&mut self, target_workspace: usize) -> Result<(), WmError> {
        if target_workspace >= self.workspaces.len() || target_workspace == self.focused_workspace {
            return Ok(());
        }
        self.cancel_move()?;

        let Some(window) = self.focused_workspace().focused_client() else {
            return Ok(());
        };

        self.xserver
            .set_border_color(window, self.config.unfocused_border_color())?;

        let Some(client) = self.focused_workspace_mut().take_client(window) else {
            return Ok(());
        };

        self.workspaces[target_workspace].add_client(client);
        self.hide_window(window)?;
        self.arrange()?;
        self.focus_fallback(None)
    }

    fn hide_window(&self, window: Window) -> Result<(), WmError> {
        let screen = self.xserver.screen_geometry();
        let offscreen_x = -i32::from(screen.width).saturating_mul(2);
        self.xserver.move_window(window, offscreen_x, 0)?;
        Ok(())
    }

    fn cancel_move(&mut self) -> Result<(), WmError> {
        if self.moving.take().is_some() {
            self.xserver.ungrab_pointer()?;
        }
        Ok(())
    }

    fn find_master_stack_target(
        &self,
        current_window: Window,
        direction: Direction,
    ) -> Option<Window> {
        let tiled_windows: Vec<Window> = self
            .focused_workspace()
            .clients()
            .iter()
            .filter(|client| !client.is_floating())
            .map(Client::window)
            .collect();

        let current_index = tiled_windows
            .iter()
            .position(|&window| window == current_window)?;

        if current_index == 0 {
            return match direction {
                Direction::Right => tiled_windows.get(1).copied(),
                Direction::Left | Direction::Up | Direction::Down => None,
            };
        }

        let master = *tiled_windows.first()?;

        match direction {
            Direction::Left => Some(master),

            Direction::Right => None,

            Direction::Up if current_index > 1 => tiled_windows.get(current_index - 1).copied(),

            Direction::Up => None,

            Direction::Down => tiled_windows.get(current_index + 1).copied(),
        }
    }

    fn toggle_fullscreen(&mut self) -> Result<(), WmError> {
        let window = match self.focused_workspace().focused_client() {
            Some(window) => window,
            None => return Ok(()),
        };

        let fullscreen = match self.focused_workspace().client(window) {
            Some(client) => !client.is_fullscreen(),
            None => return Ok(()),
        };

        self.set_client_fullscreen(window, fullscreen)
    }

    fn set_client_fullscreen(&mut self, window: Window, fullscreen: bool) -> Result<(), WmError> {
        let workspace_index = match self
            .workspaces
            .iter()
            .position(|workspace| workspace.client(window).is_some())
        {
            Some(index) => index,
            None => return Ok(()),
        };

        let current_state = self.workspaces[workspace_index]
            .client(window)
            .is_some_and(Client::is_fullscreen);

        if current_state == fullscreen {
            return Ok(());
        }

        if fullscreen {
            let other_fullscreen_windows: Vec<Window> = self.workspaces[workspace_index]
                .clients()
                .iter()
                .filter(|client| client.window() != window && client.is_fullscreen())
                .map(Client::window)
                .collect();

            for other in other_fullscreen_windows {
                if let Some(client) = self.workspaces[workspace_index].client_mut(other) {
                    client.set_fullscreen(false);
                }

                self.xserver.set_fullscreen_state(other, false)?;
            }
        }

        let client = match self.workspaces[workspace_index].client_mut(window) {
            Some(client) => client,
            None => return Ok(()),
        };

        client.set_fullscreen(fullscreen);

        self.xserver.set_fullscreen_state(window, fullscreen)?;

        if workspace_index == self.focused_workspace {
            self.arrange()?;

            if self.focused_workspace().focused_client() == Some(window) {
                self.focus_client(window, None)?;
            }
        }

        Ok(())
    }
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn rect_edges(rect: Rectangle) -> (i32, i32, i32, i32) {
    let left = i32::from(rect.x);
    let top = i32::from(rect.y);
    (
        left,
        top,
        left + i32::from(rect.width),
        top + i32::from(rect.height),
    )
}

fn is_in_direction(current: Rectangle, candidate: Rectangle, direction: Direction) -> bool {
    let (cl, ct, cr, cb) = rect_edges(current);
    let (nl, nt, nr, nb) = rect_edges(candidate);
    let current_cx = cl + (cr - cl) / 2;
    let current_cy = ct + (cb - ct) / 2;
    let next_cx = nl + (nr - nl) / 2;
    let next_cy = nt + (nb - nt) / 2;

    match direction {
        Direction::Left => next_cx < current_cx,
        Direction::Right => next_cx > current_cx,
        Direction::Up => next_cy < current_cy,
        Direction::Down => next_cy > current_cy,
    }
}

fn direction_score(
    current: Rectangle,
    candidate: Rectangle,
    window: Window,
    direction: Direction,
) -> (i32, u8, i32, Reverse<i32>, i32, Window) {
    let (cl, ct, cr, cb) = rect_edges(current);
    let (nl, nt, nr, nb) = rect_edges(candidate);
    let (primary_gap, overlap, secondary_distance, position) = match direction {
        Direction::Left => (
            (cl - nr).max(0),
            overlap_len(ct, cb, nt, nb),
            ((ct + cb) / 2 - (nt + nb) / 2).abs(),
            nt,
        ),
        Direction::Right => (
            (nl - cr).max(0),
            overlap_len(ct, cb, nt, nb),
            ((ct + cb) / 2 - (nt + nb) / 2).abs(),
            nt,
        ),
        Direction::Up => (
            (ct - nb).max(0),
            overlap_len(cl, cr, nl, nr),
            ((cl + cr) / 2 - (nl + nr) / 2).abs(),
            nl,
        ),
        Direction::Down => (
            (nt - cb).max(0),
            overlap_len(cl, cr, nl, nr),
            ((cl + cr) / 2 - (nl + nr) / 2).abs(),
            nl,
        ),
    };
    (
        primary_gap,
        u8::from(overlap == 0),
        secondary_distance,
        Reverse(overlap),
        position,
        window,
    )
}

fn wrap_score(
    candidate: Rectangle,
    window: Window,
    direction: Direction,
) -> (Reverse<i32>, i32, Reverse<i32>, Window) {
    let (left, top, right, bottom) = rect_edges(candidate);
    let center_x = left + (right - left) / 2;
    let center_y = top + (bottom - top) / 2;
    match direction {
        Direction::Left => (Reverse(center_x), top, Reverse(right - left), window),
        Direction::Right => (Reverse(-center_x), top, Reverse(right - left), window),
        Direction::Up => (Reverse(center_y), left, Reverse(bottom - top), window),
        Direction::Down => (Reverse(-center_y), left, Reverse(bottom - top), window),
    }
}

fn overlap_len(first_start: i32, first_end: i32, second_start: i32, second_end: i32) -> i32 {
    (first_end.min(second_end) - first_start.max(second_start)).max(0)
}
