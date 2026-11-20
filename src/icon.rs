use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn create_tray_keyboard_icon() -> Result<HICON> {
    unsafe {
        let size = 32;

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut pixel_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

        let hbm_color = CreateDIBSection(
            Some(hdc_mem),
            &bmi,
            DIB_RGB_COLORS,
            &mut pixel_ptr,
            None,
            0,
        )?;

        let old_bmp = SelectObject(hdc_mem, hbm_color.into());

        // Fill background with black (transparent mask)
        let bg_brush = CreateSolidBrush(COLORREF(0x00000000));
        let rect = RECT { left: 0, top: 0, right: size, bottom: size };
        let _ = FillRect(hdc_mem, &rect, bg_brush);
        let _ = DeleteObject(bg_brush.into());

        // Keyboard frame
        let kb_brush = CreateSolidBrush(COLORREF(0x00EEEEEE)); // Silver / White
        let kb_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00888888));
        let old_brush = SelectObject(hdc_mem, kb_brush.into());
        let old_pen = SelectObject(hdc_mem, kb_pen.into());

        let _ = RoundRect(hdc_mem, 2, 7, 30, 25, 6, 6);

        // Top row function keys
        let key_brush = CreateSolidBrush(COLORREF(0x00444444));
        let _ = SelectObject(hdc_mem, key_brush.into());

        for i in 0..5 {
            let kx = 5 + i * 5;
            let _ = RoundRect(hdc_mem, kx, 10, kx + 4, 13, 2, 2);
        }

        // Main keys row
        for i in 0..4 {
            let kx = 5 + i * 6;
            let _ = RoundRect(hdc_mem, kx, 15, kx + 5, 18, 2, 2);
        }

        // Spacebar
        let _ = RoundRect(hdc_mem, 10, 20, 22, 22, 2, 2);

        let _ = SelectObject(hdc_mem, old_brush);
        let _ = SelectObject(hdc_mem, old_pen);
        let _ = DeleteObject(kb_brush.into());
        let _ = DeleteObject(kb_pen.into());
        let _ = DeleteObject(key_brush.into());

        let _ = SelectObject(hdc_mem, old_bmp);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);

        let hbm_mask = CreateBitmap(size, size, 1, 1, None);

        let icon_info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };

        let hicon = CreateIconIndirect(&icon_info);
        let _ = DeleteObject(hbm_color.into());
        let _ = DeleteObject(hbm_mask.into());

        hicon
    }
}
