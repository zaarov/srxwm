use x11rb::protocol::xproto::ModMask;
use xkeysym::Keysym;

#[derive(Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub enum Action {
    Spawn(Command),
    CloseWindow,
    FocusWindow(Direction),
    SwapWindow(Direction),
    MoveToWorkspace(usize),
    SwitchWorkspace(usize),
}

pub struct Command {
    program: String,
    args: Option<Vec<String>>,
}

impl Command {
    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> Option<&[String]> {
        self.args.as_deref()
    }
}

pub struct KeyBinding {
    modifiers: ModMask,
    key: Keysym,
    action: Action,
}

impl KeyBinding {
    pub fn modifiers(&self) -> ModMask {
        self.modifiers
    }

    pub fn key(&self) -> Keysym {
        self.key
    }

    pub fn action(&self) -> &Action {
        &self.action
    }
}

pub struct Config {
    mod_key: ModMask,
    gaps: u32,
    border_width: u32,
    focused_border_color: u32,
    unfocused_border_color: u32,
    master_ratio: u32,
    startup_commands: Option<Vec<Command>>,
    keybindings: Vec<KeyBinding>,
}

impl Default for Config {
    fn default() -> Self {
        let mod_key: ModMask = ModMask::M4;
        
        Self {
            mod_key,
            gaps: 14,
            border_width: 2,
            focused_border_color: 0x8848a8,
            unfocused_border_color: 0x575757,
            master_ratio: 50,
            
            // Startup commands should be an Option(Some, None)
            // args are Option(Some, None) as well
            //
            // Example:
            // startup_commands: Some(vec![Command {
            //    program: String::from("feh"),
            //     args: Some(vec![
            //         String::from("--bg-fill"),
            //         String::from("path/to/bg"),
            //     ]),
            // }]),
            startup_commands: None,

            keybindings: vec![
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Return,
                    action: Action::Spawn(Command {
                        program: String::from("alacritty"),
                        args: None,
                    }),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Q,
                    action: Action::CloseWindow,
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Right,
                    action: Action::FocusWindow(Direction::Right),
                },
                
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Left,
                    action: Action::FocusWindow(Direction::Left),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Up,
                    action: Action::FocusWindow(Direction::Up),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Down,
                    action: Action::FocusWindow(Direction::Down),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Left,
                    action: Action::SwapWindow(Direction::Left),
                },
                
                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Right,
                    action: Action::SwapWindow(Direction::Right),
                },
                
                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Up,
                    action: Action::SwapWindow(Direction::Up),
                },
                
                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Down,
                    action: Action::SwapWindow(Direction::Down),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_1,
                    action: Action::MoveToWorkspace(0),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_2,
                    action: Action::MoveToWorkspace(1),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_3,
                    action: Action::MoveToWorkspace(2),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_4,
                    action: Action::MoveToWorkspace(3),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_5,
                    action: Action::MoveToWorkspace(4),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_6,
                    action: Action::MoveToWorkspace(5),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_7,
                    action: Action::MoveToWorkspace(6),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_8,
                    action: Action::MoveToWorkspace(7),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::_9,
                    action: Action::MoveToWorkspace(8),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_1,
                    action: Action::SwitchWorkspace(0),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_2,
                    action: Action::SwitchWorkspace(1),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_3,
                    action: Action::SwitchWorkspace(2),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_4,
                    action: Action::SwitchWorkspace(3),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_5,
                    action: Action::SwitchWorkspace(4),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_6,
                    action: Action::SwitchWorkspace(5),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_7,
                    action: Action::SwitchWorkspace(6),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_8,
                    action: Action::SwitchWorkspace(7),
                },


                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::_9,
                    action: Action::SwitchWorkspace(8),
                },
            ],
        }
    }
}

impl Config {
    pub fn mod_key(&self) -> ModMask {
        self.mod_key
    }
    
    pub fn gaps(&self) -> u32 {
        self.gaps
    }
    
    pub fn border_width(&self) -> u32 {
        self.border_width
    }
    
    pub fn focused_border_color(&self) -> u32 {
        self.focused_border_color
    }
    
    pub fn unfocused_border_color(&self) -> u32 {
        self.unfocused_border_color
    }
    
    pub fn master_ratio(&self) -> u32 {
        self.master_ratio
    }

    pub fn startup_commands(&self) -> Option<&[Command]> {
        self.startup_commands.as_deref()
    }
    
    pub fn keybindings(&self) -> &[KeyBinding] {
        &self.keybindings
    }
}
