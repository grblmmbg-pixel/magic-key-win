use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    DefaultKey, // Standard pass-through
    NativeF(u8), // Native function key (1..=19, e.g. F1..F12 / F13..F19)
    ToggleFnLock, // Switch Mac/PC mode (toggle media vs F-keys)
    VolumeUp,
    VolumeDown,
    VolumeMute,
    PlayPause,
    NextTrack,
    PrevTrack,
    TaskView,
    Search,
    LockScreen,
    Screenshot,
    Calculator,
    Hotkey {
        ctrl: bool,
        alt: bool,
        shift: bool,
        win: bool,
        vk: u16,
        display: String,
    },
    LaunchApp {
        path: String,
        display: String,
    },
}

impl KeyAction {
    pub fn name(&self) -> String {
        match self {
            KeyAction::DefaultKey => "Стандартная клавиша (Без действия)".to_string(),
            KeyAction::ToggleFnLock => "Переключить режим (Медиа / F-клавиши)".to_string(),
            KeyAction::NativeF(n) => match n {
                1 => "F1 (Справка / Help)".to_string(),
                2 => "F2 (Переименовать)".to_string(),
                3 => "F3 (Поиск)".to_string(),
                4 => "F4 (Адресная строка / Закрыть)".to_string(),
                5 => "F5 (Обновить страницу)".to_string(),
                6 => "F6 (Фокус адресной строки)".to_string(),
                7 => "F7 (Курсорная навигация)".to_string(),
                8 => "F8 (Выделение)".to_string(),
                9 => "F9".to_string(),
                10 => "F10 (Меню программы)".to_string(),
                11 => "F11 (Полноэкранный режим)".to_string(),
                12 => "F12 (Инструменты разработчика)".to_string(),
                _ => format!("F{}", n),
            },
            KeyAction::VolumeUp => "Громкость + (Увеличить)".to_string(),
            KeyAction::VolumeDown => "Громкость - (Уменьшить)".to_string(),
            KeyAction::VolumeMute => "Без звука (Mute)".to_string(),
            KeyAction::PlayPause => "Воспроизведение / Пауза".to_string(),
            KeyAction::NextTrack => "Следующий трек".to_string(),
            KeyAction::PrevTrack => "Предыдущий трек".to_string(),
            KeyAction::TaskView => "Task View (Обзор окон Win+Tab)".to_string(),
            KeyAction::Search => "Поиск Windows (Win+S)".to_string(),
            KeyAction::LockScreen => "Блокировка экрана (Win+L)".to_string(),
            KeyAction::Screenshot => "Скриншот экрана (Win+Shift+S)".to_string(),
            KeyAction::Calculator => "Калькулятор".to_string(),
            KeyAction::Hotkey { display, .. } => format!("Хоткей: {}", display),
            KeyAction::LaunchApp { display, .. } => format!("Запуск: {}", display),
        }
    }

    pub fn standard_presets() -> &'static [KeyAction] {
        &[
            KeyAction::NativeF(1),
            KeyAction::NativeF(2),
            KeyAction::NativeF(3),
            KeyAction::NativeF(4),
            KeyAction::NativeF(5),
            KeyAction::NativeF(6),
            KeyAction::NativeF(7),
            KeyAction::NativeF(8),
            KeyAction::NativeF(9),
            KeyAction::NativeF(10),
            KeyAction::NativeF(11),
            KeyAction::NativeF(12),
            KeyAction::NativeF(13),
            KeyAction::NativeF(14),
            KeyAction::NativeF(15),
            KeyAction::NativeF(16),
            KeyAction::NativeF(17),
            KeyAction::NativeF(18),
            KeyAction::NativeF(19),
            KeyAction::VolumeUp,
            KeyAction::VolumeDown,
            KeyAction::VolumeMute,
            KeyAction::PlayPause,
            KeyAction::NextTrack,
            KeyAction::PrevTrack,
            KeyAction::TaskView,
            KeyAction::Search,
            KeyAction::Screenshot,
            KeyAction::Calculator,
            KeyAction::LockScreen,
            KeyAction::ToggleFnLock,
            KeyAction::DefaultKey,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mac_mode: bool,         // true = Media keys by default
    pub autostart: bool,        // true = Start with Windows
    pub show_overlay: bool,     // true = Show on-screen HUD overlay
    pub f1: KeyAction,
    pub f2: KeyAction,
    pub f3: KeyAction,
    pub f4: KeyAction,
    pub f5: KeyAction,
    pub f6: KeyAction,
    pub f7: KeyAction,
    pub f8: KeyAction,
    pub f9: KeyAction,
    pub f10: KeyAction,
    pub f11: KeyAction,
    pub f12: KeyAction,
    pub f13: KeyAction,
    pub f14: KeyAction,
    pub f15: KeyAction,
    pub f16: KeyAction,
    pub f17: KeyAction,
    pub f18: KeyAction,
    pub f19: KeyAction,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mac_mode: true,
            autostart: false,
            show_overlay: true,
            f1: KeyAction::NativeF(1),
            f2: KeyAction::NativeF(2),
            f3: KeyAction::TaskView,
            f4: KeyAction::Search,
            f5: KeyAction::NativeF(5),
            f6: KeyAction::NativeF(6),
            f7: KeyAction::PrevTrack,
            f8: KeyAction::PlayPause,
            f9: KeyAction::NextTrack,
            f10: KeyAction::VolumeMute,
            f11: KeyAction::VolumeDown,
            f12: KeyAction::VolumeUp,
            f13: KeyAction::Screenshot,
            f14: KeyAction::TaskView,
            f15: KeyAction::Calculator,
            f16: KeyAction::NativeF(16),
            f17: KeyAction::NativeF(17),
            f18: KeyAction::NativeF(18),
            f19: KeyAction::LockScreen,
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return dir.join("magic_key_config.json");
            }
        }
        PathBuf::from("magic_key_config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str(&content) {
                return cfg;
            }
        }
        let default = Self::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn get_key_action(&self, id: &str) -> KeyAction {
        match id {
            "f1" => self.f1.clone(),
            "f2" => self.f2.clone(),
            "f3" => self.f3.clone(),
            "f4" => self.f4.clone(),
            "f5" => self.f5.clone(),
            "f6" => self.f6.clone(),
            "f7" => self.f7.clone(),
            "f8" => self.f8.clone(),
            "f9" => self.f9.clone(),
            "f10" => self.f10.clone(),
            "f11" => self.f11.clone(),
            "f12" => self.f12.clone(),
            "f13" => self.f13.clone(),
            "f14" => self.f14.clone(),
            "f15" => self.f15.clone(),
            "f16" => self.f16.clone(),
            "f17" => self.f17.clone(),
            "f18" => self.f18.clone(),
            "f19" => self.f19.clone(),
            _ => KeyAction::DefaultKey,
        }
    }

    pub fn set_key_action(&mut self, id: &str, action: KeyAction) {
        match id {
            "f1" => self.f1 = action,
            "f2" => self.f2 = action,
            "f3" => self.f3 = action,
            "f4" => self.f4 = action,
            "f5" => self.f5 = action,
            "f6" => self.f6 = action,
            "f7" => self.f7 = action,
            "f8" => self.f8 = action,
            "f9" => self.f9 = action,
            "f10" => self.f10 = action,
            "f11" => self.f11 = action,
            "f12" => self.f12 = action,
            "f13" => self.f13 = action,
            "f14" => self.f14 = action,
            "f15" => self.f15 = action,
            "f16" => self.f16 = action,
            "f17" => self.f17 = action,
            "f18" => self.f18 = action,
            "f19" => self.f19 = action,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_f_presets() {
        let presets = KeyAction::standard_presets();
        assert!(presets.contains(&KeyAction::NativeF(1)));
        assert!(presets.contains(&KeyAction::NativeF(5)));
        assert!(presets.contains(&KeyAction::NativeF(11)));
        assert!(presets.contains(&KeyAction::NativeF(12)));
        assert!(presets.contains(&KeyAction::NativeF(19)));

        assert_eq!(KeyAction::NativeF(5).name(), "F5 (Обновить страницу)");
        assert_eq!(KeyAction::NativeF(11).name(), "F11 (Полноэкранный режим)");
        assert_eq!(KeyAction::NativeF(12).name(), "F12 (Инструменты разработчика)");
    }

    #[test]
    fn test_json_serialization_compatibility() {
        let json_input = r#"{
            "mac_mode": true,
            "autostart": false,
            "show_overlay": true,
            "f1": "DefaultKey",
            "f2": "VolumeUp",
            "f3": "TaskView",
            "f4": "Search",
            "f5": { "NativeF": 5 },
            "f6": { "NativeF": 6 },
            "f7": "PrevTrack",
            "f8": "PlayPause",
            "f9": "NextTrack",
            "f10": "VolumeMute",
            "f11": { "NativeF": 11 },
            "f12": { "NativeF": 12 },
            "f13": { "NativeF": 5 },
            "f14": "TaskView",
            "f15": "Calculator",
            "f16": "DefaultKey",
            "f17": "DefaultKey",
            "f18": "DefaultKey",
            "f19": "LockScreen"
        }"#;

        let parsed: AppConfig = serde_json::from_str(json_input).expect("Should deserialize cleanly");
        assert_eq!(parsed.f1, KeyAction::DefaultKey);
        assert_eq!(parsed.f2, KeyAction::VolumeUp);
        assert_eq!(parsed.f5, KeyAction::NativeF(5));
        assert_eq!(parsed.f11, KeyAction::NativeF(11));
        assert_eq!(parsed.f12, KeyAction::NativeF(12));
        assert_eq!(parsed.f13, KeyAction::NativeF(5));

        let reserialized = serde_json::to_string(&parsed).expect("Should serialize cleanly");
        let re_parsed: AppConfig = serde_json::from_str(&reserialized).expect("Should deserialize roundtrip");
        assert_eq!(re_parsed.f5, KeyAction::NativeF(5));
        assert_eq!(re_parsed.f11, KeyAction::NativeF(11));
    }

    #[test]
    fn test_set_get_key_action() {
        let mut cfg = AppConfig::default();
        cfg.set_key_action("f11", KeyAction::NativeF(11));
        assert_eq!(cfg.get_key_action("f11"), KeyAction::NativeF(11));

        cfg.set_key_action("f13", KeyAction::NativeF(5));
        assert_eq!(cfg.get_key_action("f13"), KeyAction::NativeF(5));
    }

    #[test]
    fn test_launch_app_serialization() {
        let action = KeyAction::LaunchApp {
            path: r"C:\Windows\System32\notepad.exe".to_string(),
            display: "notepad.exe".to_string(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: KeyAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
        assert_eq!(deserialized.name(), "Запуск: notepad.exe");
    }
}
