use x11rb::protocol::xproto::ModMask;
use xkeysym::Keysym;

pub struct Config {
    mod_key: ModMask,
    gaps: u32,
    border: u32,
    focused_border: u32,
    unfocused_border: u32,
    keybindings: Vec<KeyBinding>,
    startup_commands: Vec<StartupCommand>,
}

impl Default for Config {
    fn default() -> Self {
        let mod_key: ModMask = ModMask::M4;
        
        Self {
            mod_key,
            gaps: 14,
            border: 2,
            focused_border: 0x8848a8,
            unfocused_border: 0x4c566a,
            
            keybindings: vec![
                /*
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Return,
                    action: Action::spawn(
                        "alacritty",
                        [
                            "-o",
                            "window.opacity=0.85",

                            "-o",
                            "colors.primary.background=\"#080808\"",

                            "-o",
                            "colors.primary.foreground=\"#bdbdbd\"",
                        ],
                    ),
                },
                 */
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Return,
                    action: Action::spawn(
                        "st",
                        std::iter::empty::<&str>(),
                    ),
                },

                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::D,
                    action: Action::spawn(
                        "rofi",
                        [
                            "-show", "drun",
                            "-show-icons",
                            "-theme-str", "configuration { font: \"GohuFont 14 Nerd Font 10\"; }",
                            "-theme-str", "* { background-color: #080808; text-color: #dcdcdc; }",
                            "-theme-str", "window { location: center; anchor: center; width: 40%; border: 2px; border-color: #8848a8; border-radius: 0px; padding: 8px; background-color: #080808; }",
                            "-theme-str", "prompt { text-color: #8cc85f; }",
                            "-theme-str", "textbox-prompt-colon { text-color: #dcdcdc; str: \":\"; }",
                            "-theme-str", "entry { text-color: #8848a8; }",
                            "-theme-str", "listview { lines: 12; border: 2px 0px 0px 0px; border-color: #dcdcdc; scrollbar: true; }",
                            "-theme-str", "scrollbar { handle-color: #dcdcdc; background-color: #080808; width: 4px; }",
                            "-theme-str", "element normal.normal, element alternate.normal { background-color: #080808; text-color: #dcdcdc; }",
                            "-theme-str", "element selected.normal { background-color: #242424; text-color: #8848a8; }",
                            "-theme-str", "element-text, element-icon { background-color: inherit; text-color: inherit; }",
                        ]
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
                    action: Action::FocusNext,
                },
                
                KeyBinding {
                    modifiers: mod_key,
                    key: Keysym::Left,
                    action: Action::FocusPrevious,
                },
            ],
            
            startup_commands: vec![
                StartupCommand::new(
                    "feh",
                    [
                        "--bg-fill",
                        "/home/zarov/Pictures/cherry.jpg",
                    ],
                ),

                StartupCommand::new(
                    "polybar",
                    [
                        "--config=/run/media/zarov/secondary/learning_projects/simp-wm/thirdparty_configs/poly_bar/config.ini",
                        "example",
                    ],
                )
                
                /*
                StartupCommand::new(
                    "picom",
                    [
                        "--backend",
                        "xrender",
                    ],
                ),
                */
            ],
        }
    }
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

impl StartupCommand {
    pub fn new(program: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

pub enum Action {
    Spawn {
        program: String,
        args: Vec<String>,
    },
    CloseWindow,
    FocusNext,
    FocusPrevious,
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

    pub fn startup_commands(&self) -> &[StartupCommand] {
        &self.startup_commands
    }
    
    pub fn keybindings(&self) -> &[KeyBinding] {
        &self.keybindings
    }
}
