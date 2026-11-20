use windows::core::*;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

pub struct AudioController {
    endpoint: IAudioEndpointVolume,
}

impl AudioController {
    pub fn new() -> Result<Self> {
        unsafe {
            // Initialize COM for current thread if not already initialized
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let default_device: IMMDevice =
                enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let endpoint: IAudioEndpointVolume = default_device.Activate(CLSCTX_ALL, None)?;

            Ok(Self { endpoint })
        }
    }

    pub fn get_volume(&self) -> Result<f32> {
        unsafe { self.endpoint.GetMasterVolumeLevelScalar() }
    }

    pub fn set_volume(&self, level: f32) -> Result<()> {
        let clamped = level.clamp(0.0, 1.0);
        unsafe {
            self.endpoint.SetMasterVolumeLevelScalar(clamped, std::ptr::null())?;
        }
        Ok(())
    }

    pub fn volume_up(&self, step: f32) -> Result<f32> {
        let current = self.get_volume()?;
        let target = (current + step).clamp(0.0, 1.0);
        self.set_volume(target)?;
        // Also unmute if muted when user turns volume up
        if self.is_muted()? {
            self.set_mute(false)?;
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
        unsafe {
            let muted = self.endpoint.GetMute()?;
            Ok(muted.as_bool())
        }
    }

    pub fn set_mute(&self, mute: bool) -> Result<()> {
        unsafe {
            self.endpoint.SetMute(mute, std::ptr::null())?;
        }
        Ok(())
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
