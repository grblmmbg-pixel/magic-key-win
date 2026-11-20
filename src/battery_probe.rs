use windows::core::*;
use windows::Win32::Devices::HumanInterfaceDevice::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;

pub fn probe_feature_reports_65() {
    let path = r"\\?\HID#{00001124-0000-1000-8000-00805f9b34fb}_VID&0001004c_PID&026c&Col01#c&22858f29&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}";
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let handle = CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        );

        let handle = match handle {
            Ok(h) if !h.is_invalid() => h,
            _ => {
                let h = CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAGS_AND_ATTRIBUTES(0),
                    None,
                );
                match h {
                    Ok(h) if !h.is_invalid() => h,
                    _ => {
                        println!("Failed to open Col01");
                        return;
                    }
                }
            }
        };

        println!("Testing 65-byte Feature Reports on Col01:");
        for report_id in 0x00..=0xFF {
            let mut buf = [0u8; 65];
            buf[0] = report_id;
            let ok = HidD_GetFeature(handle, buf.as_mut_ptr() as _, 65);
            if ok {
                let non_zero: Vec<_> = buf.iter().enumerate().filter(|(_, v)| **v != 0).collect();
                println!("  [0x{:02X}] OK! Non-zero entries: {:?}", report_id, non_zero);
            }
        }

        let _ = CloseHandle(handle);
    }
}
