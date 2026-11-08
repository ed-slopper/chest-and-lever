//! Settings and key bindings (board rows E6, X4), saved to a settings file in the user's data folder.
//!
//! REF R24: actions are decoupled from raw input, so keys can be rebound. OURS: the default keys, and which
//! gamepad buttons do what.

use crate::palette::Vision;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What a key can mean: a sim action, or something for the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Command {
    Forward,
    Back,
    StrafeLeft,
    StrafeRight,
    TurnLeft,
    TurnRight,
    Jump,
    LongJump,
    Use,
    Attack,
    Wait,
    PickUp,
    Inventory,
    Spells,
    Map,
    Debug,
    Menu,
}

impl Command {
    pub const ALL: [Command; 17] = [
        Command::Forward,
        Command::Back,
        Command::StrafeLeft,
        Command::StrafeRight,
        Command::TurnLeft,
        Command::TurnRight,
        Command::Jump,
        Command::LongJump,
        Command::Use,
        Command::Attack,
        Command::Wait,
        Command::PickUp,
        Command::Inventory,
        Command::Spells,
        Command::Map,
        Command::Debug,
        Command::Menu,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Command::Forward => "step forward",
            Command::Back => "step back",
            Command::StrafeLeft => "step left",
            Command::StrafeRight => "step right",
            Command::TurnLeft => "turn left",
            Command::TurnRight => "turn right",
            Command::Jump => "jump",
            Command::LongJump => "long jump",
            Command::Use => "use / search",
            Command::Attack => "attack",
            Command::Wait => "wait",
            Command::PickUp => "pick up",
            Command::Inventory => "inventory",
            Command::Spells => "spells",
            Command::Map => "big map",
            Command::Debug => "debug overlay",
            Command::Menu => "menu",
        }
    }
    pub fn action(self) -> Option<lac_sim::Action> {
        use lac_sim::Action as A;
        Some(match self {
            Command::Forward => A::StepForward,
            Command::Back => A::StepBack,
            Command::StrafeLeft => A::StepLeft,
            Command::StrafeRight => A::StepRight,
            Command::TurnLeft => A::TurnLeft,
            Command::TurnRight => A::TurnRight,
            Command::Jump => A::Jump,
            Command::LongJump => A::LongJump,
            Command::Use => A::Use,
            Command::Attack => A::Attack,
            Command::Wait => A::Wait,
            Command::PickUp => A::PickUp,
            _ => return None,
        })
    }
}

/// The keys we know by name (for the settings file and the rebinding screen).
pub const KEYS: &[(&str, KeyCode)] = &[
    ("A", KeyCode::KeyA), ("B", KeyCode::KeyB), ("C", KeyCode::KeyC), ("D", KeyCode::KeyD), ("E", KeyCode::KeyE),
    ("F", KeyCode::KeyF), ("G", KeyCode::KeyG), ("H", KeyCode::KeyH), ("I", KeyCode::KeyI), ("J", KeyCode::KeyJ),
    ("K", KeyCode::KeyK), ("L", KeyCode::KeyL), ("M", KeyCode::KeyM), ("N", KeyCode::KeyN), ("O", KeyCode::KeyO),
    ("P", KeyCode::KeyP), ("Q", KeyCode::KeyQ), ("R", KeyCode::KeyR), ("S", KeyCode::KeyS), ("T", KeyCode::KeyT),
    ("U", KeyCode::KeyU), ("V", KeyCode::KeyV), ("W", KeyCode::KeyW), ("X", KeyCode::KeyX), ("Y", KeyCode::KeyY),
    ("Z", KeyCode::KeyZ), ("5", KeyCode::Digit5), ("6", KeyCode::Digit6), ("7", KeyCode::Digit7), ("8", KeyCode::Digit8),
    ("9", KeyCode::Digit9), ("0", KeyCode::Digit0), ("Up", KeyCode::ArrowUp), ("Down", KeyCode::ArrowDown),
    ("Left", KeyCode::ArrowLeft), ("Right", KeyCode::ArrowRight), ("Space", KeyCode::Space), ("Enter", KeyCode::Enter),
    ("Tab", KeyCode::Tab), ("Escape", KeyCode::Escape), ("Backspace", KeyCode::Backspace),
    ("LShift", KeyCode::ShiftLeft), ("RShift", KeyCode::ShiftRight), ("LCtrl", KeyCode::ControlLeft),
    ("RCtrl", KeyCode::ControlRight), ("LAlt", KeyCode::AltLeft), ("Period", KeyCode::Period), ("Comma", KeyCode::Comma),
    ("Slash", KeyCode::Slash), ("Semicolon", KeyCode::Semicolon), ("Quote", KeyCode::Quote),
    ("LBracket", KeyCode::BracketLeft), ("RBracket", KeyCode::BracketRight), ("Minus", KeyCode::Minus),
    ("Equal", KeyCode::Equal), ("Backquote", KeyCode::Backquote), ("Insert", KeyCode::Insert), ("Delete", KeyCode::Delete),
    ("Home", KeyCode::Home), ("End", KeyCode::End), ("PageUp", KeyCode::PageUp), ("PageDown", KeyCode::PageDown),
    ("Num0", KeyCode::Numpad0), ("Num1", KeyCode::Numpad1), ("Num2", KeyCode::Numpad2), ("Num3", KeyCode::Numpad3),
    ("Num4", KeyCode::Numpad4), ("Num5", KeyCode::Numpad5), ("Num6", KeyCode::Numpad6), ("Num7", KeyCode::Numpad7),
    ("Num8", KeyCode::Numpad8), ("Num9", KeyCode::Numpad9), ("F1", KeyCode::F1), ("F2", KeyCode::F2), ("F3", KeyCode::F3),
    ("F4", KeyCode::F4), ("F5", KeyCode::F5), ("F6", KeyCode::F6), ("F7", KeyCode::F7), ("F8", KeyCode::F8),
];

pub fn key_name(k: KeyCode) -> &'static str {
    KEYS.iter().find(|(_, c)| *c == k).map(|(n, _)| *n).unwrap_or("?")
}

pub fn key_from_name(n: &str) -> Option<KeyCode> {
    KEYS.iter().find(|(name, _)| *name == n).map(|(_, c)| *c)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsFile {
    pub volume: f32,
    pub music: f32,
    pub spider_free: bool,
    pub vision: Vision,
    /// 1 = 480×270 (F8), 2 = 960×540.
    pub res_scale: u8,
    pub ssao: bool,
    /// Command to key names.
    pub bindings: Vec<(Command, Vec<String>)>,
}

impl Default for SettingsFile {
    fn default() -> Self {
        let b = |c: Command, keys: &[&str]| (c, keys.iter().map(|s| s.to_string()).collect());
        SettingsFile {
            volume: 0.7,
            music: 0.4,
            spider_free: false,
            vision: Vision::Normal,
            res_scale: 1,
            ssao: true,
            bindings: vec![
                b(Command::Forward, &["W", "Up", "Num8"]),
                b(Command::Back, &["S", "Down", "Num2"]),
                b(Command::StrafeLeft, &["A", "Num1"]),
                b(Command::StrafeRight, &["D", "Num3"]),
                b(Command::TurnLeft, &["Q", "Left", "Num4"]),
                b(Command::TurnRight, &["E", "Right", "Num6"]),
                b(Command::Jump, &["Space"]),
                b(Command::LongJump, &["LShift", "RShift"]),
                b(Command::Use, &["F", "Enter"]),
                b(Command::Attack, &["R", "LCtrl"]),
                b(Command::Wait, &["Period", "Num5", "Z"]),
                b(Command::PickUp, &["G"]),
                b(Command::Inventory, &["I", "Tab"]),
                b(Command::Spells, &["C"]),
                b(Command::Map, &["M"]),
                b(Command::Debug, &["F3"]),
                b(Command::Menu, &["Escape"]),
            ],
        }
    }
}

/// The settings in use, with the bindings parsed.
#[derive(Resource, Clone, Debug)]
pub struct Settings {
    pub file: SettingsFile,
}

impl Settings {
    pub fn keys(&self, c: Command) -> Vec<KeyCode> {
        self.file
            .bindings
            .iter()
            .find(|(cmd, _)| *cmd == c)
            .map(|(_, ks)| ks.iter().filter_map(|k| key_from_name(k)).collect())
            .unwrap_or_default()
    }
    pub fn just(&self, c: Command, keys: &ButtonInput<KeyCode>) -> bool {
        self.keys(c).iter().any(|k| keys.just_pressed(*k))
    }
    /// Set the only key of a command, and take it off any other.
    pub fn rebind(&mut self, c: Command, k: KeyCode) {
        let name = key_name(k).to_string();
        for (cmd, ks) in &mut self.file.bindings {
            if *cmd == c {
                *ks = vec![name.clone()];
            } else {
                ks.retain(|n| *n != name);
            }
        }
        if !self.file.bindings.iter().any(|(cmd, _)| *cmd == c) {
            self.file.bindings.push((c, vec![name]));
        }
    }
    pub fn describe(&self, c: Command) -> String {
        let ks = self.keys(c);
        if ks.is_empty() {
            "-".into()
        } else {
            ks.iter().map(|k| key_name(*k)).collect::<Vec<_>>().join(" / ")
        }
    }
    pub fn load() -> Settings {
        let file = std::fs::read_to_string(settings_path()).ok().and_then(|t| ron::from_str::<SettingsFile>(&t).ok()).unwrap_or_default();
        let mut s = Settings { file };
        // a command missing from an older file gets its default keys
        for (c, ks) in SettingsFile::default().bindings {
            if !s.file.bindings.iter().any(|(cmd, _)| *cmd == c) {
                s.file.bindings.push((c, ks));
            }
        }
        s
    }
    pub fn save(&self) {
        let p = settings_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = ron::ser::to_string_pretty(&self.file, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(p, text);
        }
    }
}

/// Where settings and saves live: `%APPDATA%/chest-and-lever` on Windows, `~/.local/share/chest-and-lever`
/// elsewhere, or the working folder if neither is known.
pub fn data_dir() -> PathBuf {
    if let Ok(a) = std::env::var("APPDATA") {
        return PathBuf::from(a).join("chest-and-lever");
    }
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h).join(".local").join("share").join("chest-and-lever");
    }
    PathBuf::from(".")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.ron")
}

pub fn save_path() -> PathBuf {
    data_dir().join("run.ron")
}

/// OURS: the gamepad layout. D-pad moves and turns, shoulders strafe, A jump, B long jump, X use, Y attack,
/// Start the menu, Select the inventory.
pub fn gamepad_commands(pads: &Query<&Gamepad>) -> Vec<Command> {
    let mut out = Vec::new();
    for pad in pads.iter() {
        for (b, c) in [
            (GamepadButton::DPadUp, Command::Forward),
            (GamepadButton::DPadDown, Command::Back),
            (GamepadButton::DPadLeft, Command::TurnLeft),
            (GamepadButton::DPadRight, Command::TurnRight),
            (GamepadButton::LeftTrigger, Command::StrafeLeft),
            (GamepadButton::RightTrigger, Command::StrafeRight),
            (GamepadButton::South, Command::Jump),
            (GamepadButton::East, Command::LongJump),
            (GamepadButton::West, Command::Use),
            (GamepadButton::North, Command::Attack),
            (GamepadButton::Start, Command::Menu),
            (GamepadButton::Select, Command::Inventory),
            (GamepadButton::LeftTrigger2, Command::Wait),
            (GamepadButton::RightTrigger2, Command::PickUp),
        ] {
            if pad.just_pressed(b) {
                out.push(c);
            }
        }
    }
    out
}
