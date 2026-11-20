#![windows_subsystem = "windows"]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Input::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

mod audio;
mod autostart;
mod config;
mod gui;
mod icon;
mod media;
mod overlay;

static FN_PRESSED: AtomicBool = AtomicBool::new(false);
static CONFIG: Mutex<Option<config::AppConfig>> = Mutex::new(None);
static mut HOOK_HANDLE: HHOOK = HHOOK(core::ptr::null_mut());
static AUDIO_CONTROLLER: Mutex<Option<audio::AudioController>> = Mutex::new(None);

const WM_TRAYICON: u32 = WM_USER + 1;
const IDM_SETTINGS: usize = 1000;
const IDM_TOGGLE_MODE: usize = 1001;
const IDM_EXIT: usize = 1003;

fn execute_action(action: config::KeyAction) {
    let show_overlay = {
        let guard = CONFIG.lock().unwrap();
        guard.as_ref().map(|c| c.show_overlay).unwrap_or(true)
    };

    match action {
        config::KeyAction::DefaultKey => {}
        config::KeyAction::ToggleFnLock => {
            let mut guard = CONFIG.lock().unwrap();
            if let Some(ref mut c) = *guard {
                c.mac_mode = !c.mac_mode;
                let _ = c.save();
                if show_overlay {
                    overlay::show_action(if c.mac_mode { "Режим: Mac (Медиа)" } else { "Режим: PC (F-клавиши)" });
                }
            }
        }
        config::KeyAction::VolumeUp => {
            if let Ok(guard) = AUDIO_CONTROLLER.lock() {
                if let Some(ref audio) = *guard {
                    if let Ok(vol) = audio.volume_up(0.02) {
                        if show_overlay {
                            overlay::show_volume(vol, false);
                        }
                    }
                }
            }
        }
        config::KeyAction::VolumeDown => {
            if let Ok(guard) = AUDIO_CONTROLLER.lock() {
                if let Some(ref audio) = *guard {
                    if let Ok(vol) = audio.volume_down(0.02) {
                        if show_overlay {
                            overlay::show_volume(vol, false);
                        }
                    }
                }
            }
        }
        config::KeyAction::VolumeMute => {
            if let Ok(guard) = AUDIO_CONTROLLER.lock() {
                if let Some(ref audio) = *guard {
                    if let Ok(muted) = audio.toggle_mute() {
                        let vol = audio.get_volume().unwrap_or(0.0);
                        if show_overlay {
                            overlay::show_volume(vol, muted);
                        }
                    }
                }
            }
        }
        config::KeyAction::PlayPause => {
            media::play_pause();
            if show_overlay {
                overlay::show_action("Воспроизведение / Пауза");
            }
        }
        config::KeyAction::NextTrack => {
            media::next_track();
            if show_overlay {
                overlay::show_action("Следующий трек");
            }
        }
        config::KeyAction::PrevTrack => {
            media::prev_track();
            if show_overlay {
                overlay::show_action("Предыдущий трек");
            }
        }
        config::KeyAction::TaskView => {
            media::task_view();
            if show_overlay {
                overlay::show_action("Task View (Обзор окон)");
            }
        }
        config::KeyAction::Search => {
            media::search();
            if show_overlay {
                overlay::show_action("Поиск Windows");
            }
        }
        config::KeyAction::LockScreen => {
            if show_overlay {
                overlay::show_action("Блокировка экрана");
            }
            media::lock_workstation();
        }
        config::KeyAction::Screenshot => {
            if show_overlay {
                overlay::show_action("Скриншот экрана");
            }
            media::screenshot();
        }
        config::KeyAction::NativeF(num) => {
            if show_overlay {
                show_f_overlay(num);
            }
            media::send_f_key(num);
        }
        config::KeyAction::Calculator => {
            if show_overlay {
                overlay::show_action("Калькулятор");
            }
            media::calculator();
        }
        config::KeyAction::Hotkey { ctrl, alt, shift, win, vk, ref display } => {
            if show_overlay {
                overlay::show_action(&format!("Хоткей: {}", display));
            }
            media::send_hotkey(ctrl, alt, shift, win, vk);
        }
    }
}

fn show_f_overlay(target_f: u8) {
    let name = match target_f {
        1 => "F1 (Справка)",
        2 => "F2 (Переименовать)",
        3 => "F3 (Поиск)",
        4 => "F4 (Адресная строка)",
        5 => "F5 (Обновить)",
        6 => "F6 (Фокус)",
        7 => "F7",
        8 => "F8",
        9 => "F9",
        10 => "F10 (Меню)",
        11 => "F11 (Полный экран)",
        12 => "F12 (DevTools)",
        _ => "",
    };
    if name.is_empty() {
        overlay::show_action(&format!("F{}", target_f));
    } else {
        overlay::show_action(name);
    }
}

fn vk_to_key_id(vk: VIRTUAL_KEY) -> Option<(&'static str, &'static str)> {
    match vk.0 {
        0x70 => Some(("f1", "F1")),
        0x71 => Some(("f2", "F2")),
        0x72 => Some(("f3", "F3")),
        0x73 => Some(("f4", "F4")),
        0x74 => Some(("f5", "F5")),
        0x75 => Some(("f6", "F6")),
        0x76 => Some(("f7", "F7")),
        0x77 => Some(("f8", "F8")),
        0x78 => Some(("f9", "F9")),
        0x79 => Some(("f10", "F10")),
        0x7A => Some(("f11", "F11")),
        0x7B => Some(("f12", "F12")),
        0x7C => Some(("f13", "F13")),
        0x7D => Some(("f14", "F14")),
        0x7E => Some(("f15", "F15")),
        0x7F => Some(("f16", "F16")),
        0x80 => Some(("f17", "F17")),
        0x81 => Some(("f18", "F18")),
        0x82 => Some(("f19", "F19")),
        _ => None,
    }
}

unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let is_down = w_param.0 as u32 == WM_KEYDOWN || w_param.0 as u32 == WM_SYSKEYDOWN;
        let is_up = w_param.0 as u32 == WM_KEYUP || w_param.0 as u32 == WM_SYSKEYUP;

        if is_down || is_up {
            let kbd = unsafe { *(l_param.0 as *const KBDLLHOOKSTRUCT) };

            // Ignore injected / synthetic events from SendInput to avoid infinite loops
            if (kbd.flags.0 & 0x10) != 0 {
                return unsafe { CallNextHookEx(Some(HOOK_HANDLE), n_code, w_param, l_param) };
            }

            let vk = VIRTUAL_KEY(kbd.vkCode as u16);

            if is_down {
                let name = match vk_to_key_id(vk) {
                    Some((_, n)) => n,
                    None => "Other",
                };
                gui::update_key_tester(vk.0, kbd.scanCode, name);
            }

            let mac_mode = {
                let guard = CONFIG.lock().unwrap();
                guard.as_ref().map(|c| c.mac_mode).unwrap_or(true)
            };

            // Hardware volume keys interception (avoids dropping volume)
            if vk == VK_VOLUME_MUTE && is_down {
                execute_action(config::KeyAction::VolumeMute);
                return LRESULT(1);
            } else if vk == VK_VOLUME_DOWN && is_down {
                execute_action(config::KeyAction::VolumeDown);
                return LRESULT(1);
            } else if vk == VK_VOLUME_UP && is_down {
                execute_action(config::KeyAction::VolumeUp);
                return LRESULT(1);
            }

            // Check if key is F1..F19
            if let Some((key_id, _)) = vk_to_key_id(vk) {
                let action = {
                    let guard = CONFIG.lock().unwrap();
                    guard.as_ref().map(|c| c.get_key_action(key_id)).unwrap_or(config::KeyAction::DefaultKey)
                };

                let fn_down = FN_PRESSED.load(Ordering::SeqCst);

                // For F1-F12: honor Mac/PC mode
                // For F13-F19: execute configured action directly
                let is_f1_to_f12 = (0x70..=0x7B).contains(&vk.0);
                let should_trigger = if is_f1_to_f12 {
                    if mac_mode { !fn_down } else { fn_down }
                } else {
                    true
                };

                if should_trigger {
                    match action {
                        config::KeyAction::DefaultKey => {
                            // Let the physical key pass through to the system
                        }
                        config::KeyAction::NativeF(target_f) => {
                            let target_vk = 0x70 + (target_f as u16 - 1);
                            if vk.0 == target_vk {
                                // Physical key already matches target F-key!
                                // Pass through with 0 latency and native modifier support (Alt+F4, Ctrl+F5...)
                                if is_down {
                                    let show_overlay = {
                                        let guard = CONFIG.lock().unwrap();
                                        guard.as_ref().map(|c| c.show_overlay).unwrap_or(true)
                                    };
                                    if show_overlay {
                                        show_f_overlay(target_f);
                                    }
                                }
                                // Do not return LRESULT(1); fall through to CallNextHookEx
                            } else {
                                // Physical key is different (e.g. F13 -> F5, or F11 -> F12)
                                if is_down {
                                    execute_action(config::KeyAction::NativeF(target_f));
                                }
                                return LRESULT(1); // Consume the physical key
                            }
                        }
                        _ => {
                            // Media action, hotkey, or fn lock toggle
                            if is_down {
                                execute_action(action);
                            }
                            return LRESULT(1); // Consume the physical key
                        }
                    }
                }
            }
        }
    }

    unsafe { CallNextHookEx(Some(HOOK_HANDLE), n_code, w_param, l_param) }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_INPUT => {
            let mut size: u32 = 0;
            let _ = unsafe {
                GetRawInputData(
                    HRAWINPUT(l_param.0 as _),
                    RID_INPUT,
                    None,
                    &mut size,
                    std::mem::size_of::<RAWINPUTHEADER>() as u32,
                )
            };

            if size > 0 {
                let mut buffer = vec![0u8; size as usize];
                if unsafe {
                    GetRawInputData(
                        HRAWINPUT(l_param.0 as _),
                        RID_INPUT,
                        Some(buffer.as_mut_ptr() as _),
                        &mut size,
                        std::mem::size_of::<RAWINPUTHEADER>() as u32,
                    )
                } != u32::MAX
                {
                    let raw = unsafe { &*(buffer.as_ptr() as *const RAWINPUT) };
                    if raw.header.dwType == RIM_TYPEHID.0 {
                        let hid = unsafe { &raw.data.hid };
                        let slice = unsafe {
                            std::slice::from_raw_parts(
                                &hid.bRawData as *const u8,
                                hid.dwSizeHid as usize,
                            )
                        };

                        // Check vendor-specific Fn bit on UsagePage 0xFF00
                        if slice.len() >= 2 {
                            let fn_down = slice.iter().any(|&b| b == 1 || b == 8);
                            FN_PRESSED.store(fn_down, Ordering::SeqCst);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            let event = (l_param.0 & 0xFFFF) as u32;
            if event == WM_LBUTTONDBLCLK {
                let cfg = {
                    let guard = CONFIG.lock().unwrap();
                    guard.clone().unwrap_or_default()
                };
                gui::open_settings_window(cfg);
            } else if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                let mut pt = POINT::default();
                let _ = unsafe { GetCursorPos(&mut pt) };
                let menu = unsafe { CreatePopupMenu().unwrap() };

                let mac_mode = {
                    let guard = CONFIG.lock().unwrap();
                    guard.as_ref().map(|c| c.mac_mode).unwrap_or(true)
                };

                let _ = unsafe { AppendMenuW(menu, MF_STRING, IDM_SETTINGS, w!("Настройки (F1-F19 & Хоткеи)...")) };
                let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };

                let mode_text = if mac_mode {
                    w!("Режим: Mac (Медиа по умолчанию)")
                } else {
                    w!("Режим: PC (Стандартные F-клавиши)")
                };
                let _ = unsafe { AppendMenuW(menu, MF_STRING, IDM_TOGGLE_MODE, mode_text) };
                let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };

                // Current volume display
                if let Ok(guard) = AUDIO_CONTROLLER.lock() {
                    if let Some(ref audio) = *guard {
                        if let Ok(vol) = audio.get_volume() {
                            let vol_str = format!("Громкость: {:.0}%\0", vol * 100.0);
                            let vol_w: Vec<u16> = vol_str.encode_utf16().collect();
                            let _ = unsafe { AppendMenuW(menu, MF_GRAYED | MF_STRING, 0, PCWSTR(vol_w.as_ptr())) };
                            let _ = unsafe { AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) };
                        }
                    }
                }

                let _ = unsafe { AppendMenuW(menu, MF_STRING, IDM_EXIT, w!("Выход")) };

                unsafe {
                    let _ = SetForegroundWindow(hwnd);
                    let _ = TrackPopupMenu(
                        menu,
                        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
                        pt.x,
                        pt.y,
                        Some(0),
                        hwnd,
                        None,
                    );
                    let _ = DestroyMenu(menu);
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = w_param.0 as usize;
            match id {
                IDM_SETTINGS => {
                    let cfg = {
                        let guard = CONFIG.lock().unwrap();
                        guard.clone().unwrap_or_default()
                    };
                    gui::open_settings_window(cfg);
                }
                IDM_TOGGLE_MODE => {
                    let mut guard = CONFIG.lock().unwrap();
                    if let Some(ref mut c) = *guard {
                        c.mac_mode = !c.mac_mode;
                        let _ = c.save();
                        overlay::show_action(if c.mac_mode { "Режим Mac (Медиа)" } else { "Режим PC (F-клавиши)" });
                    }
                }
                IDM_EXIT => {
                    unsafe {
                        let _ = DestroyWindow(hwnd);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

fn create_tray_icon(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        ..Default::default()
    };

    unsafe {
        if let Ok(icon) = icon::create_tray_keyboard_icon() {
            nid.hIcon = icon;
        } else {
            nid.hIcon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
        }

        let tip = "MagicKeyWin — Apple Keyboard Companion\0";
        for (i, c) in tip.encode_utf16().enumerate() {
            if i < nid.szTip.len() {
                nid.szTip[i] = c;
            }
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }

    nid
}

fn main() -> Result<()> {
    // Load persisted configuration
    let initial_config = config::AppConfig::load();
    if let Ok(mut guard) = CONFIG.lock() {
        *guard = Some(initial_config);
    }

    // Set callback from GUI window to update active configuration
    gui::set_on_save(|new_cfg| {
        if let Ok(mut guard) = CONFIG.lock() {
            *guard = Some(new_cfg);
        }
    });

    // Initialize Core Audio
    if let Ok(ac) = audio::AudioController::new() {
        if let Ok(mut guard) = AUDIO_CONTROLLER.lock() {
            *guard = Some(ac);
        }
    }

    // Initialize OSD Overlay window
    let _ = overlay::init_overlay();

    unsafe {
        let class_name = w!("MagicKeyWinReceiverClassV3");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("MagicKeyWinCore"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        )?;

        let nid = create_tray_icon(hwnd);

        // Register RawInput for Apple vendor collection (0xFF00)
        let rids = [
            RAWINPUTDEVICE {
                usUsagePage: 0xFF00,
                usUsage: 0x0000,
                dwFlags: RIDEV_INPUTSINK | RIDEV_PAGEONLY,
                hwndTarget: hwnd,
            },
        ];

        let _ = RegisterRawInputDevices(&rids, std::mem::size_of::<RAWINPUTDEVICE>() as u32);

        // Global low-level keyboard hook
        HOOK_HANDLE = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)?;

        // Message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let _ = UnhookWindowsHookEx(HOOK_HANDLE);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }

    Ok(())
}
