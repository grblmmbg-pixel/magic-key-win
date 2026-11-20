use windows::core::*;
use windows::Win32::System::Registry::*;

const REG_KEY_RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_NAME: PCWSTR = w!("MagicKeyWin");

pub fn is_autostart_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            REG_KEY_RUN,
            Some(0),
            KEY_READ,
            &mut hkey,
        ).is_ok() {
            let mut buf_size: u32 = 0;
            let status = RegQueryValueExW(
                hkey,
                APP_NAME,
                None,
                None,
                None,
                Some(&mut buf_size),
            );
            let _ = RegCloseKey(hkey);
            status.is_ok() && buf_size > 0
        } else {
            false
        }
    }
}

pub fn set_autostart(enable: bool) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            REG_KEY_RUN,
            Some(0),
            KEY_WRITE,
            &mut hkey,
        );

        if status.is_ok() {
            if enable {
                if let Ok(exe_path) = std::env::current_exe() {
                    let cmd = format!("\"{}\"\0", exe_path.to_string_lossy());
                    let wide: Vec<u16> = cmd.encode_utf16().collect();
                    let bytes = std::slice::from_raw_parts(
                        wide.as_ptr() as *const u8,
                        wide.len() * 2,
                    );
                    let _ = RegSetValueExW(
                        hkey,
                        APP_NAME,
                        Some(0),
                        REG_SZ,
                        Some(bytes),
                    );
                }
            } else {
                let _ = RegDeleteValueW(hkey, APP_NAME);
            }

            let _ = RegCloseKey(hkey);
        }
        Ok(())
    }
}
