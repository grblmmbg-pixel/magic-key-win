use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static OVERLAY_STATE: Mutex<Option<OsdData>> = Mutex::new(None);
static OVERLAY_HWND: Mutex<Option<isize>> = Mutex::new(None);

const TIMER_HIDE: usize = 901;

#[derive(Clone)]
pub struct OsdData {
    pub title: String,
    pub progress: Option<f32>, // 0.0 .. 1.0 (for volume bar)
    pub is_muted: bool,
}

unsafe extern "system" fn osd_wndproc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER => {
            if w_param.0 == TIMER_HIDE {
                unsafe {
                    let _ = KillTimer(Some(hwnd), TIMER_HIDE);
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            unsafe {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                let data = {
                    let guard = OVERLAY_STATE.lock().unwrap();
                    guard.clone()
                };

                if let Some(data) = data {
                    let mut rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rect);

                    // Background: rounded dark box
                    let bg_brush = CreateSolidBrush(COLORREF(0x00222020)); // BGR dark gray
                    let border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00444040));
                    let old_brush = SelectObject(hdc, bg_brush.into());
                    let old_pen = SelectObject(hdc, border_pen.into());

                    let _ = RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 24, 24);

                    let _ = SetBkMode(hdc, TRANSPARENT);

                    // Title font
                    let hfont = CreateFontW(
                        22,
                        0,
                        0,
                        0,
                        FW_SEMIBOLD.0 as i32,
                        0,
                        0,
                        0,
                        DEFAULT_CHARSET,
                        OUT_DEFAULT_PRECIS,
                        CLIP_DEFAULT_PRECIS,
                        CLEARTYPE_QUALITY,
                        0,
                        w!("Segoe UI"),
                    );
                    let old_font = SelectObject(hdc, hfont.into());

                    SetTextColor(hdc, COLORREF(0x00FFFFFF)); // White

                    let mut title_rect = RECT {
                        left: rect.left,
                        top: if data.progress.is_some() { 18 } else { 34 },
                        right: rect.right,
                        bottom: rect.bottom,
                    };
                    let mut wide_title: Vec<u16> = data.title.encode_utf16().chain(std::iter::once(0)).collect();
                    let _ = DrawTextW(
                        hdc,
                        &mut wide_title[..],
                        &mut title_rect,
                        DT_CENTER | DT_SINGLELINE,
                    );

                    // If progress bar (Volume)
                    if let Some(prog) = data.progress {
                        let bar_w = 180;
                        let bar_h = 10;
                        let bar_left = (rect.right - bar_w) / 2;
                        let bar_top = 56;

                        let bar_bg = CreateSolidBrush(COLORREF(0x00555050));
                        let old_bar_brush = SelectObject(hdc, bar_bg.into());
                        let _ = RoundRect(
                            hdc,
                            bar_left,
                            bar_top,
                            bar_left + bar_w,
                            bar_top + bar_h,
                            bar_h,
                            bar_h,
                        );
                        let _ = DeleteObject(bar_bg.into());

                        if !data.is_muted && prog > 0.0 {
                            let fill_w = ((bar_w as f32) * prog.clamp(0.0, 1.0)) as i32;
                            let fill_brush = CreateSolidBrush(COLORREF(0x00D09020)); // Soft blue accent (BGR)
                            let _ = SelectObject(hdc, fill_brush.into());
                            let _ = RoundRect(
                                hdc,
                                bar_left,
                                bar_top,
                                bar_left + fill_w.max(bar_h),
                                bar_top + bar_h,
                                bar_h,
                                bar_h,
                            );
                            let _ = DeleteObject(fill_brush.into());
                        }
                        let _ = SelectObject(hdc, old_bar_brush);

                        // Subtitle / percentage
                        let sub_font = CreateFontW(
                            14,
                            0,
                            0,
                            0,
                            FW_NORMAL.0 as i32,
                            0,
                            0,
                            0,
                            DEFAULT_CHARSET,
                            OUT_DEFAULT_PRECIS,
                            CLIP_DEFAULT_PRECIS,
                            CLEARTYPE_QUALITY,
                            0,
                            w!("Segoe UI"),
                        );
                        let _ = SelectObject(hdc, sub_font.into());
                        SetTextColor(hdc, COLORREF(0x00CCCCCC));

                        let sub_text = if data.is_muted {
                            "Без звука".to_string()
                        } else {
                            format!("{:.0}%", prog * 100.0)
                        };
                        let mut wide_sub: Vec<u16> = sub_text.encode_utf16().chain(std::iter::once(0)).collect();
                        let mut sub_rect = RECT {
                            left: rect.left,
                            top: 74,
                            right: rect.right,
                            bottom: rect.bottom,
                        };
                        let _ = DrawTextW(
                            hdc,
                            &mut wide_sub[..],
                            &mut sub_rect,
                            DT_CENTER | DT_SINGLELINE,
                        );
                        let _ = DeleteObject(sub_font.into());
                    }

                    let _ = SelectObject(hdc, old_font);
                    let _ = DeleteObject(hfont.into());
                    let _ = SelectObject(hdc, old_brush);
                    let _ = SelectObject(hdc, old_pen);
                    let _ = DeleteObject(bg_brush.into());
                    let _ = DeleteObject(border_pen.into());
                }

                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

pub fn init_overlay() -> Result<()> {
    unsafe {
        let class_name = w!("MagicKeyWinOsdClass");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(osd_wndproc),
            lpszClassName: class_name,
            hbrBackground: HBRUSH(core::ptr::null_mut()),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let w = 240;
        let h = 100;
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - w) / 2;
        let y = screen_h - h - 110; // Bottom centered

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            w!("MagicKeyWinOsd"),
            WS_POPUP,
            x,
            y,
            w,
            h,
            None,
            None,
            None,
            None,
        )?;

        // Layered alpha opacity
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 225, LWA_ALPHA);

        if let Ok(mut guard) = OVERLAY_HWND.lock() {
            *guard = Some(hwnd.0 as isize);
        }
    }
    Ok(())
}

pub fn show_volume(vol: f32, is_muted: bool) {
    let title = if is_muted {
        "Звук отключен".to_string()
    } else {
        "Громкость".to_string()
    };
    show_osd(OsdData {
        title,
        progress: Some(vol),
        is_muted,
    });
}

pub fn show_action(name: &str) {
    show_osd(OsdData {
        title: name.to_string(),
        progress: None,
        is_muted: false,
    });
}

fn show_osd(data: OsdData) {
    if let Ok(mut guard) = OVERLAY_STATE.lock() {
        *guard = Some(data);
    }

    if let Ok(guard) = OVERLAY_HWND.lock() {
        if let Some(h) = *guard {
            let hwnd = HWND(h as *mut _);
            unsafe {
                let _ = InvalidateRect(Some(hwnd), None, true);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                // Reset fade timer to 1300 ms
                let _ = SetTimer(Some(hwnd), TIMER_HIDE, 1300, None);
            }
        }
    }
}
