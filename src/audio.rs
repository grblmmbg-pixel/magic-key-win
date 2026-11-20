use std::sync::Mutex;
use windows::core::*;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

struct CachedEndpoint {
    device_id: String,
    endpoint: IAudioEndpointVolume,
}

pub struct AudioController {
    enumerator: IMMDeviceEnumerator,
    cached: Mutex<Option<CachedEndpoint>>,
}

impl AudioController {
    pub fn new() -> Result<Self> {
        unsafe {
            // Initialize COM for current thread
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let controller = Self {
                enumerator,
                cached: Mutex::new(None),
            };

            // Pre-warm the cache with the current default endpoint
            let _ = controller.get_endpoint();

            Ok(controller)
        }
    }

    /// Dynamically obtains the current active default audio render endpoint.
    /// If the user switches audio device (e.g. HDMI -> USB, USB -> Headphones),
    /// this automatically detects the new device and activates its volume endpoint.
    fn get_endpoint(&self) -> Result<IAudioEndpointVolume> {
        unsafe {
            let default_device: IMMDevice =
                self.enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let id_pwstr = default_device.GetId()?;
            let current_id = id_pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(id_pwstr.as_ptr() as _));

            if let Ok(guard) = self.cached.lock() {
                if let Some(ref c) = *guard {
                    if c.device_id == current_id {
                        return Ok(c.endpoint.clone());
                    }
                }
            }

            // Default audio device changed or cache empty: activate new endpoint
            let endpoint: IAudioEndpointVolume = default_device.Activate(CLSCTX_ALL, None)?;

            if let Ok(mut guard) = self.cached.lock() {
                *guard = Some(CachedEndpoint {
                    device_id: current_id,
                    endpoint: endpoint.clone(),
                });
            }

            Ok(endpoint)
        }
    }

    pub fn invalidate_cache(&self) {
        if let Ok(mut guard) = self.cached.lock() {
            *guard = None;
        }
    }

    pub fn get_volume(&self) -> Result<f32> {
        let ep = self.get_endpoint()?;
        unsafe {
            match ep.GetMasterVolumeLevelScalar() {
                Ok(vol) => Ok(vol),
                Err(_) => {
                    self.invalidate_cache();
                    let fresh_ep = self.get_endpoint()?;
                    fresh_ep.GetMasterVolumeLevelScalar()
                }
            }
        }
    }

    pub fn set_volume(&self, level: f32) -> Result<()> {
        let clamped = level.clamp(0.0, 1.0);
        let ep = self.get_endpoint()?;
        unsafe {
            match ep.SetMasterVolumeLevelScalar(clamped, std::ptr::null()) {
                Ok(_) => Ok(()),
                Err(_) => {
                    self.invalidate_cache();
                    let fresh_ep = self.get_endpoint()?;
                    fresh_ep.SetMasterVolumeLevelScalar(clamped, std::ptr::null())
                }
            }
        }
    }

    pub fn volume_up(&self, step: f32) -> Result<f32> {
        let current = self.get_volume()?;
        let target = (current + step).clamp(0.0, 1.0);
        self.set_volume(target)?;
        if self.is_muted().unwrap_or(false) {
            let _ = self.set_mute(false);
        }
        Ok(target)
    }

    pub fn volume_down(&self, step: f32) -> Result<f32> {
        let current = self.get_volume()?;
        let target = (current - step).clamp(0.0, 1.0);
        self.set_volume(target)?;
        Ok(target)
    }

    pub fn is_muted(&self) -> Result<bool> {
        let ep = self.get_endpoint()?;
        unsafe {
            match ep.GetMute() {
                Ok(muted) => Ok(muted.as_bool()),
                Err(_) => {
                    self.invalidate_cache();
                    let fresh_ep = self.get_endpoint()?;
                    let muted = fresh_ep.GetMute()?;
                    Ok(muted.as_bool())
                }
            }
        }
    }

    pub fn set_mute(&self, mute: bool) -> Result<()> {
        let ep = self.get_endpoint()?;
        unsafe {
            match ep.SetMute(mute, std::ptr::null()) {
                Ok(_) => Ok(()),
                Err(_) => {
                    self.invalidate_cache();
                    let fresh_ep = self.get_endpoint()?;
                    fresh_ep.SetMute(mute, std::ptr::null())
                }
            }
        }
    }

    pub fn toggle_mute(&self) -> Result<bool> {
        let current = self.is_muted()?;
        let target = !current;
        self.set_mute(target)?;
        Ok(target)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_controller_init() {
        let audio = AudioController::new().expect("Failed to initialize AudioController");
        let vol = audio.get_volume().expect("Failed to get master volume");
        let muted = audio.is_muted().expect("Failed to get mute status");
        println!("Current master volume: {:.1}%, Muted: {}", vol * 100.0, muted);
        assert!((0.0..=1.0).contains(&vol));
    }
}


unsafe impl Send for AudioController {}
unsafe impl Sync for AudioController {}
