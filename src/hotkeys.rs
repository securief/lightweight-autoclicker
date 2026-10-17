use std::sync::atomic::Ordering;
use std::{sync::Arc, time::Duration};

use eframe::egui;
use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};

use crate::{
    memory::BG_THREAD_STACK,
    state::{SharedState, HOTKEY_REBIND_GRACE_MS},
};

pub fn wake_unfocused(ctx: &egui::Context) {
    ctx.request_repaint();
    ctx.request_repaint_after(Duration::from_millis(16));
}

pub fn parse_code(name: &str) -> Code {
    match name.to_uppercase().as_str() {
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "A" | "KEYA" => Code::KeyA,
        "B" | "KEYB" => Code::KeyB,
        "C" | "KEYC" => Code::KeyC,
        "D" | "KEYD" => Code::KeyD,
        "E" | "KEYE" => Code::KeyE,
        "F" | "KEYF" => Code::KeyF,
        "G" | "KEYG" => Code::KeyG,
        "H" | "KEYH" => Code::KeyH,
        "I" | "KEYI" => Code::KeyI,
        "J" | "KEYJ" => Code::KeyJ,
        "K" | "KEYK" => Code::KeyK,
        "L" | "KEYL" => Code::KeyL,
        "M" | "KEYM" => Code::KeyM,
        "N" | "KEYN" => Code::KeyN,
        "O" | "KEYO" => Code::KeyO,
        "P" | "KEYP" => Code::KeyP,
        "Q" | "KEYQ" => Code::KeyQ,
        "R" | "KEYR" => Code::KeyR,
        "S" | "KEYS" => Code::KeyS,
        "T" | "KEYT" => Code::KeyT,
        "U" | "KEYU" => Code::KeyU,
        "V" | "KEYV" => Code::KeyV,
        "W" | "KEYW" => Code::KeyW,
        "X" | "KEYX" => Code::KeyX,
        "Y" | "KEYY" => Code::KeyY,
        "Z" | "KEYZ" => Code::KeyZ,
        "0" | "DIGIT0" => Code::Digit0,
        "1" | "DIGIT1" => Code::Digit1,
        "2" | "DIGIT2" => Code::Digit2,
        "3" | "DIGIT3" => Code::Digit3,
        "4" | "DIGIT4" => Code::Digit4,
        "5" | "DIGIT5" => Code::Digit5,
        "6" | "DIGIT6" => Code::Digit6,
        "7" | "DIGIT7" => Code::Digit7,
        "8" | "DIGIT8" => Code::Digit8,
        "9" | "DIGIT9" => Code::Digit9,
        "SPACE" => Code::Space,
        "TAB" => Code::Tab,
        _ => Code::F6,
    }
}

pub fn short_label(name: &str) -> &'static str {
    match name.to_uppercase().as_str() {
        "F1" => "F1",
        "F2" => "F2",
        "F3" => "F3",
        "F4" => "F4",
        "F5" => "F5",
        "F7" => "F7",
        "F8" => "F8",
        "F9" => "F9",
        "F10" => "F10",
        "F11" => "F11",
        "F12" => "F12",
        "KEYA" | "A" => "A",
        "KEYB" | "B" => "B",
        "KEYC" | "C" => "C",
        "KEYD" | "D" => "D",
        "KEYE" | "E" => "E",
        "KEYF" | "F" => "F",
        "KEYG" | "G" => "G",
        "KEYH" | "H" => "H",
        "KEYI" | "I" => "I",
        "KEYJ" | "J" => "J",
        "KEYK" | "K" => "K",
        "KEYL" | "L" => "L",
        "KEYM" | "M" => "M",
        "KEYN" | "N" => "N",
        "KEYO" | "O" => "O",
        "KEYP" | "P" => "P",
        "KEYQ" | "Q" => "Q",
        "KEYR" | "R" => "R",
        "KEYS" | "S" => "S",
        "KEYT" | "T" => "T",
        "KEYU" | "U" => "U",
        "KEYV" | "V" => "V",
        "KEYW" | "W" => "W",
        "KEYX" | "X" => "X",
        "KEYY" | "Y" => "Y",
        "KEYZ" | "Z" => "Z",
        "DIGIT0" | "0" => "0",
        "DIGIT1" | "1" => "1",
        "DIGIT2" | "2" => "2",
        "DIGIT3" | "3" => "3",
        "DIGIT4" | "4" => "4",
        "DIGIT5" | "5" => "5",
        "DIGIT6" | "6" => "6",
        "DIGIT7" | "7" => "7",
        "DIGIT8" | "8" => "8",
        "DIGIT9" | "9" => "9",
        "ESCAPE" | "ESC" => "ESC",
        "SPACE" => "SPACE",
        "TAB" => "TAB",
        "ENTER" => "ENTER",
        "DELETE" => "DEL",
        "INSERT" => "INS",
        "HOME" => "HOME",
        "END" => "END",
        "PAGEUP" => "PGUP",
        "PAGEDOWN" => "PGDN",
        _ => "F6",
    }
}

pub struct HotkeySlot {
    pub name: String,
}

impl HotkeySlot {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

pub fn register_hotkeys(
    manager: &GlobalHotKeyManager,
    state: Arc<SharedState>,
    ctx: egui::Context,
    hotkey_name: &str,
) -> HotkeySlot {
    let code = parse_code(hotkey_name);
    let hk = HotKey::new(None, code);
    let _ = manager.register(hk);

    let _ = std::thread::Builder::new()
        .name("hotkey".into())
        .stack_size(BG_THREAD_STACK)
        .spawn(move || {
            let receiver = GlobalHotKeyEvent::receiver();
            while let Ok(event) = receiver.recv() {
                if event.state != HotKeyState::Pressed {
                    continue;
                }
                if state.hotkeys_guarded() {
                    state.guard_hotkeys_for(HOTKEY_REBIND_GRACE_MS);
                    continue;
                }
                if state.is_recording_hotkey.load(Ordering::SeqCst)
                    || state.is_picking_location.load(Ordering::SeqCst)
                {
                    continue;
                }
                state.toggle();
                wake_unfocused(&ctx);
            }
        });

    HotkeySlot::new(hotkey_name.to_string())
}

fn needs_rebind(old_name: &str, new_name: &str) -> bool {
    parse_code(old_name) != parse_code(new_name)
}

pub fn rebind_hotkey(
    manager: &GlobalHotKeyManager,
    old: &HotkeySlot,
    new_name: &str,
    state: &SharedState,
) -> HotkeySlot {
    if needs_rebind(&old.name, new_name) {
        let _ = manager.unregister(HotKey::new(None, parse_code(&old.name)));
        let _ = manager.register(HotKey::new(None, parse_code(new_name)));
    }
    while GlobalHotKeyEvent::receiver().try_recv().is_ok() {}
    state.guard_hotkeys_for(HOTKEY_REBIND_GRACE_MS);
    HotkeySlot::new(new_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_code_matches_ui_names() {
        let mut names: Vec<String> = (1..=12).map(|n| format!("F{n}")).collect();
        names.extend(('A'..='Z').map(String::from));
        names.extend(('0'..='9').map(String::from));
        names.push("SPACE".to_string());
        names.push("TAB".to_string());

        for name in names {
            let actual = format!("{:?}", parse_code(&name)).to_uppercase();
            let expected = match name.as_str() {
                "SPACE" | "TAB" => name.clone(),
                _ if name.len() == 1 && name.chars().all(|c| c.is_ascii_digit()) => {
                    format!("DIGIT{name}")
                }
                _ if name.len() == 1 => format!("KEY{name}"),
                _ => name.clone(),
            };
            assert_eq!(actual, expected, "parse_code(\"{name}\") out of sync");
        }
    }

    #[test]
    fn parse_code_aliases_and_fallback() {
        assert_eq!(parse_code("KeyA"), Code::KeyA);
        assert_eq!(parse_code("digit7"), Code::Digit7);
        assert_eq!(parse_code(""), Code::F6);
        assert_eq!(parse_code("unknown-key"), Code::F6);
    }

    #[test]
    fn needs_rebind_only_when_code_changes() {
        assert!(!needs_rebind("F6", "F6"));
        assert!(!needs_rebind("F6", "f6"));
        assert!(!needs_rebind("A", "KeyA"));
        assert!(!needs_rebind("5", "Digit5"));
        assert!(needs_rebind("F6", "F7"));
        assert!(needs_rebind("A", "B"));
        assert!(needs_rebind("SPACE", "TAB"));
    }
}
