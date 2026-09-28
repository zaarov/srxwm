use x11rb::protocol::xproto::ModMask;
use xkeysym::Keysym;

pub struct Config {
    mod_key: ModMask,
    gaps: u32,
    border: u32,
    focused_border: u32,
    unfocused_border: u32,
    master_width: u32,
    cursor_theme: Option<String>,
    cursor_size: u32,
    keybindings: Vec<KeyBinding>,
    startup_commands: Vec<StartupCommand>,
}

pub enum Action {
    Spawn {
        program: String,
        args: Vec<String>,
    },
    CloseWindow,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    SwapLeft,
    SwapRight,
    SwapUp,
    SwapDown,
    MoveToWorkspace(usize),
    SwitchWorkspace(usize),
}

pub struct KeyBinding {
    pub modifiers: ModMask,
    pub key: Keysym,
    pub action: Action,
}

pub struct StartupCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mod_key: ModMask = ModMask::M4;
        
        Self {
            mod_key,
            gaps: 14,
            border: 2,
            focused_border: 0x8848a8,
            unfocused_border: 0x575757,
            master_width: 50,
            cursor_theme: None,
            cursor_size: 24,
            
            keybindings: vec![
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Return,
                    action: Action::spawn(
                        "alacritty",
                        std::iter::empty::<&str>(),
                    ),
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Q,
                    action: Action::CloseWindow,
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Right,
                    action: Action::FocusRight,
                },
                
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Left,
                    action: Action::FocusLeft,
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Up,
                    action: Action::FocusUp,
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Down,
                    action: Action::FocusDown,
                },

                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Left,
                    action: Action::SwapLeft,
                },
                
                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Right,
                    action: Action::SwapRight,
                },
                
                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Up,
                    action: Action::SwapUp,
                },
                
                KeyBinding {
                    modifiers: mod_key | ModMask::SHIFT,
                    key: Keysym::Down,
                    action: Action::SwapDown,
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
            
            startup_commands: vec![
                StartupCommand::new(
                    "feh",
                    [
                        "--bg-fill",
                        "path/to/bg",
                    ],
                ),                
            ],
        }
    }
}

impl StartupCommand {
    pub fn new(program: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

impl Action {
    pub fn spawn(program: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Action::Spawn {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
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
    
    pub fn border(&self) -> u32 {
        self.border
    }
    
    pub fn focused_border(&self) -> u32 {
        self.focused_border
    }

    pub fn unfocused_border(&self) -> u32 {
        self.unfocused_border
    }

    pub fn master_width(&self) -> u32 {
        self.master_width
    }

    pub fn startup_commands(&self) -> &[StartupCommand] {
        &self.startup_commands
    }
    
    pub fn keybindings(&self) -> &[KeyBinding] {
        &self.keybindings
    }

    pub fn cursor_theme(&self) -> Option<&str> {
        self.cursor_theme.as_deref()
    }

    pub fn cursor_size(&self) -> u32 {
        self.cursor_size
    }
}
