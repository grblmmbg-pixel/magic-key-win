use crate::autostart;
use crate::config::{AppConfig, KeyAction};
use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static SETTINGS_HWND: Mutex<Option<isize>> = Mutex::new(None);
static CONFIG_CALLBACK: Mutex<Option<Box<dyn Fn(AppConfig) + Send + Sync>>> = Mutex::new(None);
static CURRENT_CONFIG: Mutex<Option<AppConfig>> = Mutex::new(None);
static RECORDING_TARGET: Mutex<Option<usize>> = Mutex::new(None);

const ID_BTN_SAVE: usize = 3001;
const ID_BTN_RESET: usize = 3002;
const ID_BTN_CANCEL: usize = 3003;
const ID_CHK_MAC_MODE: usize = 3004;
const ID_CHK_AUTOSTART: usize = 3005;
const ID_CHK_OVERLAY: usize = 3006;
const ID_STATIC_KEY_TESTER: usize = 3007;

const COMBO_BASE_ID: usize = 3100; // 3100..3118 (19 keys)
const BTN_HOTKEY_BASE_ID: usize = 3200; // 3200..3218
const BTN_APP_BASE_ID: usize = 3300; // 3300..3318

pub const KEY_LIST: &[(&str, &str)] = &[
    ("f1", "F1"),
    ("f2", "F2"),
    ("f3", "F3"),
    ("f4", "F4"),
    ("f5", "F5"),
    ("f6", "F6"),
    ("f7", "F7"),
    ("f8", "F8"),
    ("f9", "F9"),
    ("f10", "F10"),
    ("f11", "F11"),
    ("f12", "F12"),
    ("f13", "F13"),
    ("f14", "F14"),
    ("f15", "F15"),
    ("f16", "F16"),
    ("f17", "F17"),
    ("f18", "F18"),
    ("f19", "F19"),
];

pub fn set_on_save<F>(f: F)
where
    F: Fn(AppConfig) + Send + Sync + 'static,
{
    if let Ok(mut cb) = CONFIG_CALLBACK.lock() {
        *cb = Some(Box::new(f));
    }
}

pub fn update_key_tester(vk: u16, scan: u32, name: &str) {
    if let Ok(guard) = SETTINGS_HWND.lock() {
        if let Some(h) = *guard {
            let hwnd = HWND(h as *mut _);
            unsafe {
                if let Ok(lbl) = GetDlgItem(Some(hwnd), ID_STATIC_KEY_TESTER as i32) {
                    let text = format!("Последняя нажатая клавиша: [{}] (VK: 0x{:02X}, Scan: 0x{:02X})\0", name, vk, scan);
                    let wide: Vec<u16> = text.encode_utf16().collect();
                    let _ = SetWindowTextW(lbl, PCWSTR(wide.as_ptr()));
                }
            }
        }
    }
}

pub fn open_settings_window(mut current_config: AppConfig) {
    current_config.autostart = autostart::is_autostart_enabled();

    if let Ok(mut guard) = CURRENT_CONFIG.lock() {
        *guard = Some(current_config.clone());
    }

    if let Ok(mut guard) = SETTINGS_HWND.lock() {
        if let Some(h) = *guard {
            let hwnd = HWND(h as *mut _);
            unsafe {
                let _ = SetForegroundWindow(hwnd);
                let _ = ShowWindow(hwnd, SW_SHOW);
                return;
            }
        }

        unsafe {
            let class_name = w!("MagicKeyWinSettingsClassV3");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(settings_wndproc),
                lpszClassName: class_name,
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut _),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let win_w = 880;
            let win_h = 540;
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let x = (screen_w - win_w) / 2;
            let y = (screen_h - win_h) / 2;

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                w!("Конфигуратор MagicKeyWin (F1 - F19 & Хоткеи)"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                x,
                y,
                win_w,
                win_h,
                None,
                None,
                None,
                None,
            );

            if let Ok(hwnd) = hwnd {
                build_controls(hwnd, &current_config);
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
                *guard = Some(hwnd.0 as isize);
            }
        }
    }
}

pub fn pick_program_file(hwnd_owner: HWND) -> Option<(String, String)> {
    let mut file_buf = [0u16; 1024];
    let filter = w!("Программы и ярлыки (*.exe;*.lnk;*.bat;*.cmd)\0*.exe;*.lnk;*.bat;*.cmd\0Все файлы (*.*)\0*.*\0\0");
    let title = w!("Выберите приложение для запуска кнопкой");

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd_owner,
        lpstrFilter: filter,
        lpstrFile: PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: file_buf.len() as u32,
        lpstrTitle: title,
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
        ..Default::default()
    };

    unsafe {
        if GetOpenFileNameW(&mut ofn).as_bool() {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            let path = String::from_utf16_lossy(&file_buf[..len]);
            let display = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
                .to_string();
            Some((path, display))
        } else {
            None
        }
    }
}

unsafe fn build_controls(hwnd: HWND, cfg: &AppConfig) {
    let font = unsafe { GetStockObject(DEFAULT_GUI_FONT) };

    // Checkboxes at top
    let chk_mac = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Режим Mac (Медиа-действия по умолчанию на F1-F12)"),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x00000003),
            20,
            12,
            420,
            20,
            Some(hwnd),
            Some(HMENU(ID_CHK_MAC_MODE as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(chk_mac, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        if cfg.mac_mode {
            SendMessageW(chk_mac, 0x00F1, Some(WPARAM(1)), Some(LPARAM(0)));
        }
    }

    let chk_auto = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Запускать вместе с Windows (Автозагрузка)"),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x00000003),
            20,
            34,
            420,
            20,
            Some(hwnd),
            Some(HMENU(ID_CHK_AUTOSTART as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(chk_auto, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        if cfg.autostart {
            SendMessageW(chk_auto, 0x00F1, Some(WPARAM(1)), Some(LPARAM(0)));
        }
    }

    let chk_osd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Показывать всплывающий оверлей действий (OSD)"),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x00000003),
            460,
            12,
            380,
            20,
            Some(hwnd),
            Some(HMENU(ID_CHK_OVERLAY as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(chk_osd, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        if cfg.show_overlay {
            SendMessageW(chk_osd, 0x00F1, Some(WPARAM(1)), Some(LPARAM(0)));
        }
    }

    // Key tester label at top right
    let lbl_tester = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("Последняя нажатая клавиша: [ Ожидание ввода... ]"),
            WS_CHILD | WS_VISIBLE,
            460,
            34,
            380,
            20,
            Some(hwnd),
            Some(HMENU(ID_STATIC_KEY_TESTER as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(lbl_tester, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
    }

    let presets = KeyAction::standard_presets();

    // 2-Column layout:
    // Left column: 0..10 (F1..F10)
    // Right column: 10..19 (F11..F19)
    for (i, &(key_id, key_name)) in KEY_LIST.iter().enumerate() {
        let is_col2 = i >= 10;
        let row_idx = if is_col2 { i - 10 } else { i } as i32;

        let col_x = if is_col2 { 450 } else { 20 };
        let y_pos = 65 + row_idx * 33;

        let key_label = format!("{}:\0", key_name);
        let wide_lbl: Vec<u16> = key_label.encode_utf16().collect();
        let lbl = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                PCWSTR(wide_lbl.as_ptr()),
                WS_CHILD | WS_VISIBLE,
                col_x,
                y_pos + 4,
                38,
                20,
                Some(hwnd),
                None,
                None,
                None,
            ).unwrap()
        };
        unsafe {
            SendMessageW(lbl, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        }

        let combo = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("COMBOBOX"),
                None,
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | WINDOW_STYLE(0x0003),
                col_x + 40,
                y_pos,
                236,
                350,
                Some(hwnd),
                Some(HMENU((COMBO_BASE_ID + i) as _)),
                None,
                None,
            ).unwrap()
        };
        unsafe {
            SendMessageW(combo, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        }

        let current_act = cfg.get_key_action(key_id);
        let mut select_idx = 0;

        for (idx, act) in presets.iter().enumerate() {
            let act_name = format!("{}\0", act.name());
            let wide_act: Vec<u16> = act_name.encode_utf16().collect();
            unsafe {
                let _ = SendMessageW(
                    combo,
                    0x0143,
                    Some(WPARAM(0)),
                    Some(LPARAM(wide_act.as_ptr() as _)),
                );
            }
            if *act == current_act {
                select_idx = idx;
            }
        }

        let custom_item_text = match &current_act {
            KeyAction::Hotkey { display, .. } => format!("Хоткей: {}\0", display),
            KeyAction::LaunchApp { display, .. } => format!("Запуск: {}\0", display),
            _ => "Задать свой хоткей...\0".to_string(),
        };
        let wide_custom: Vec<u16> = custom_item_text.encode_utf16().collect();
        unsafe {
            let _ = SendMessageW(
                combo,
                0x0143,
                Some(WPARAM(0)),
                Some(LPARAM(wide_custom.as_ptr() as _)),
            );
        }

        if matches!(current_act, KeyAction::Hotkey { .. } | KeyAction::LaunchApp { .. }) {
            select_idx = presets.len();
        }

        unsafe {
            let _ = SendMessageW(
                combo,
                0x014E,
                Some(WPARAM(select_idx)),
                Some(LPARAM(0)),
            );
        }

        let btn_hk = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Хоткей"),
                WS_CHILD | WS_VISIBLE,
                col_x + 280,
                y_pos,
                64,
                24,
                Some(hwnd),
                Some(HMENU((BTN_HOTKEY_BASE_ID + i) as _)),
                None,
                None,
            ).unwrap()
        };
        unsafe {
            SendMessageW(btn_hk, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        }

        let btn_app = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("BUTTON"),
                w!("Файл..."),
                WS_CHILD | WS_VISIBLE,
                col_x + 348,
                y_pos,
                64,
                24,
                Some(hwnd),
                Some(HMENU((BTN_APP_BASE_ID + i) as _)),
                None,
                None,
            ).unwrap()
        };
        unsafe {
            SendMessageW(btn_app, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
        }
    }

    // Save, Reset, Cancel buttons
    let btn_save = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Сохранить"),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x0001),
            490,
            455,
            115,
            32,
            Some(hwnd),
            Some(HMENU(ID_BTN_SAVE as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(btn_save, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
    }

    let btn_reset = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Сбросить"),
            WS_CHILD | WS_VISIBLE,
            620,
            455,
            115,
            32,
            Some(hwnd),
            Some(HMENU(ID_BTN_RESET as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(btn_reset, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
    }

    let btn_cancel = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("BUTTON"),
            w!("Закрыть"),
            WS_CHILD | WS_VISIBLE,
            750,
            455,
            100,
            32,
            Some(hwnd),
            Some(HMENU(ID_BTN_CANCEL as _)),
            None,
            None,
        ).unwrap()
    };
    unsafe {
        SendMessageW(btn_cancel, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));
    }
}

unsafe fn read_gui_config(hwnd: HWND) -> AppConfig {
    let mut cfg = {
        let guard = CURRENT_CONFIG.lock().unwrap();
        guard.clone().unwrap_or_default()
    };

    let chk_mac = unsafe { GetDlgItem(Some(hwnd), ID_CHK_MAC_MODE as i32).unwrap() };
    cfg.mac_mode = unsafe { SendMessageW(chk_mac, 0x00F0, None, None).0 == 1 };

    let chk_auto = unsafe { GetDlgItem(Some(hwnd), ID_CHK_AUTOSTART as i32).unwrap() };
    cfg.autostart = unsafe { SendMessageW(chk_auto, 0x00F0, None, None).0 == 1 };

    let chk_osd = unsafe { GetDlgItem(Some(hwnd), ID_CHK_OVERLAY as i32).unwrap() };
    cfg.show_overlay = unsafe { SendMessageW(chk_osd, 0x00F0, None, None).0 == 1 };

    let presets = KeyAction::standard_presets();

    for (i, &(key_id, _)) in KEY_LIST.iter().enumerate() {
        if let Ok(combo) = unsafe { GetDlgItem(Some(hwnd), (COMBO_BASE_ID + i) as i32) } {
            let cur_sel = unsafe { SendMessageW(combo, 0x0147, None, None).0 as usize };
            if cur_sel < presets.len() {
                cfg.set_key_action(key_id, presets[cur_sel].clone());
            }
        }
    }

    cfg
}

unsafe extern "system" fn hotkey_dlg_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = w_param.0 as u16;

            if vk == 0x10 || vk == 0x11 || vk == 0x12 || vk == 0x5B || vk == 0x5C {
                return LRESULT(0);
            }

            let (ctrl, shift, alt, win) = unsafe {
                let c = (GetKeyState(0x11) as u16 & 0x8000) != 0;
                let s = (GetKeyState(0x10) as u16 & 0x8000) != 0;
                let a = (GetKeyState(0x12) as u16 & 0x8000) != 0;
                let w = (GetKeyState(0x5B) as u16 & 0x8000) != 0 || (GetKeyState(0x5C) as u16 & 0x8000) != 0;
                (c, s, a, w)
            };

            let mut parts = Vec::new();
            if ctrl { parts.push("Ctrl"); }
            if alt { parts.push("Alt"); }
            if shift { parts.push("Shift"); }
            if win { parts.push("Win"); }

            let key_str = match vk {
                0x20 => "Space".to_string(),
                0x0D => "Enter".to_string(),
                0x09 => "Tab".to_string(),
                0x1B => "Esc".to_string(),
                0x2E => "Delete".to_string(),
                0x2D => "Insert".to_string(),
                0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
                0x41..=0x5A => format!("{}", (vk as u8) as char),
                0x30..=0x39 => format!("{}", (vk as u8) as char),
                _ => format!("Key(0x{:02X})", vk),
            };
            parts.push(&key_str);

            let display = parts.join(" + ");

            let action = KeyAction::Hotkey {
                ctrl,
                alt,
                shift,
                win,
                vk,
                display: display.clone(),
            };

            if let Ok(guard) = RECORDING_TARGET.lock() {
                if let Some(target_idx) = *guard {
                    let &(key_id, _) = &KEY_LIST[target_idx];
                    if let Ok(mut cfg_guard) = CURRENT_CONFIG.lock() {
                        if let Some(ref mut cfg) = *cfg_guard {
                            cfg.set_key_action(key_id, action);
                        }
                    }

                    if let Ok(shwnd_guard) = SETTINGS_HWND.lock() {
                        if let Some(sh) = *shwnd_guard {
                            let parent = HWND(sh as *mut _);
                            unsafe {
                                if let Ok(combo) = GetDlgItem(Some(parent), (COMBO_BASE_ID + target_idx) as i32) {
                                    let last_idx = KeyAction::standard_presets().len();
                                    let combo_text = format!("Хоткей: {}\0", display);
                                    let wide: Vec<u16> = combo_text.encode_utf16().collect();
                                    let _ = SendMessageW(combo, 0x0144, Some(WPARAM(last_idx)), Some(LPARAM(0)));
                                    let _ = SendMessageW(combo, 0x0143, Some(WPARAM(0)), Some(LPARAM(wide.as_ptr() as _)));
                                    let _ = SendMessageW(combo, 0x014E, Some(WPARAM(last_idx)), Some(LPARAM(0)));
                                }
                            }
                        }
                    }
                }
            }

            crate::overlay::show_action(&format!("Задан хоткей: {}", display));
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

fn open_hotkey_recorder(target_idx: usize, key_name: &str) {
    if let Ok(mut guard) = RECORDING_TARGET.lock() {
        *guard = Some(target_idx);
    }

    unsafe {
        let class_name = w!("MagicKeyWinRecorderClassV3");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(hotkey_dlg_proc),
            lpszClassName: class_name,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut _),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let win_w = 400;
        let win_h = 160;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let title = format!("Запись хоткея для {}\0", key_name);
        let wide_title: Vec<u16> = title.encode_utf16().collect();

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            class_name,
            PCWSTR(wide_title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            x,
            y,
            win_w,
            win_h,
            None,
            None,
            None,
            None,
        );

        if let Ok(hwnd) = hwnd {
            let font = GetStockObject(DEFAULT_GUI_FONT);
            let lbl = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("Нажмите на клавиатуре любую комбинацию клавиш:\n(например: Ctrl + C, Win + D, Alt + F4, Shift + F1...)"),
                WS_CHILD | WS_VISIBLE,
                20,
                30,
                350,
                50,
                Some(hwnd),
                None,
                None,
                None,
            ).unwrap();
            SendMessageW(lbl, WM_SETFONT, Some(WPARAM(font.0 as _)), Some(LPARAM(1)));

            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (w_param.0 & 0xFFFF) as usize;
            match id {
                ID_BTN_SAVE => {
                    let new_cfg = unsafe { read_gui_config(hwnd) };
                    let _ = new_cfg.save();
                    let _ = autostart::set_autostart(new_cfg.autostart);

                    if let Ok(cb) = CONFIG_CALLBACK.lock() {
                        if let Some(ref f) = *cb {
                            f(new_cfg);
                        }
                    }

                    crate::overlay::show_action("Настройки сохранены");
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                }
                ID_BTN_RESET => {
                    let default_cfg = AppConfig::default();
                    if let Ok(mut guard) = CURRENT_CONFIG.lock() {
                        *guard = Some(default_cfg.clone());
                    }

                    let presets = KeyAction::standard_presets();

                    unsafe {
                        let chk_mac = GetDlgItem(Some(hwnd), ID_CHK_MAC_MODE as i32).unwrap();
                        let _ = SendMessageW(
                            chk_mac,
                            0x00F1,
                            Some(WPARAM(if default_cfg.mac_mode { 1 } else { 0 })),
                            Some(LPARAM(0)),
                        );

                        let chk_auto = GetDlgItem(Some(hwnd), ID_CHK_AUTOSTART as i32).unwrap();
                        let _ = SendMessageW(
                            chk_auto,
                            0x00F1,
                            Some(WPARAM(if default_cfg.autostart { 1 } else { 0 })),
                            Some(LPARAM(0)),
                        );

                        let chk_osd = GetDlgItem(Some(hwnd), ID_CHK_OVERLAY as i32).unwrap();
                        let _ = SendMessageW(
                            chk_osd,
                            0x00F1,
                            Some(WPARAM(if default_cfg.show_overlay { 1 } else { 0 })),
                            Some(LPARAM(0)),
                        );

                        for (i, &(key_id, _)) in KEY_LIST.iter().enumerate() {
                            if let Ok(combo) = GetDlgItem(Some(hwnd), (COMBO_BASE_ID + i) as i32) {
                                let act = default_cfg.get_key_action(key_id);
                                let sel = presets.iter().position(|a| *a == act).unwrap_or(0);
                                let _ = SendMessageW(
                                    combo,
                                    0x014E,
                                    Some(WPARAM(sel)),
                                    Some(LPARAM(0)),
                                );
                            }
                        }
                    }
                    crate::overlay::show_action("Настройки сброшены");
                }
                ID_BTN_CANCEL => {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                }
                btn_id if (BTN_HOTKEY_BASE_ID..BTN_HOTKEY_BASE_ID + KEY_LIST.len()).contains(&btn_id) => {
                    let target_idx = btn_id - BTN_HOTKEY_BASE_ID;
                    let &(_, key_name) = &KEY_LIST[target_idx];
                    open_hotkey_recorder(target_idx, key_name);
                }
                btn_id if (BTN_APP_BASE_ID..BTN_APP_BASE_ID + KEY_LIST.len()).contains(&btn_id) => {
                    let target_idx = btn_id - BTN_APP_BASE_ID;
                    let &(key_id, key_name) = &KEY_LIST[target_idx];
                    if let Some((path, display)) = pick_program_file(hwnd) {
                        let action = KeyAction::LaunchApp {
                            path: path.clone(),
                            display: display.clone(),
                        };
                        if let Ok(mut cfg_guard) = CURRENT_CONFIG.lock() {
                            if let Some(ref mut cfg) = *cfg_guard {
                                cfg.set_key_action(key_id, action);
                            }
                        }

                        // Update combobox custom item text
                        unsafe {
                            if let Ok(combo) = GetDlgItem(Some(hwnd), (COMBO_BASE_ID + target_idx) as i32) {
                                let last_idx = KeyAction::standard_presets().len();
                                let combo_text = format!("Запуск: {}\0", display);
                                let wide: Vec<u16> = combo_text.encode_utf16().collect();
                                let _ = SendMessageW(combo, 0x0144, Some(WPARAM(last_idx)), Some(LPARAM(0))); // CB_DELETESTRING
                                let _ = SendMessageW(combo, 0x0143, Some(WPARAM(0)), Some(LPARAM(wide.as_ptr() as _))); // CB_ADDSTRING
                                let _ = SendMessageW(combo, 0x014E, Some(WPARAM(last_idx)), Some(LPARAM(0))); // CB_SETCURSEL
                            }
                        }

                        crate::overlay::show_action(&format!("{}: запуск {}", key_name, display));
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Ok(mut guard) = SETTINGS_HWND.lock() {
                *guard = None;
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}
