use std::error::Error;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub volume: f64, // 0.0 to 1.0
    pub is_muted: bool,
    pub is_output: bool,
}

#[derive(Clone, Debug)]
pub struct AudioManager {
    devices: Arc<RwLock<Vec<AudioDevice>>>,
}

impl AudioManager {
    pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self {
            devices: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub fn get_cached_devices(&self) -> Vec<AudioDevice> {
        self.devices.read().unwrap().clone()
    }

    /// Queries PipeWire/PulseAudio via D-Bus to build the active endpoint array
    pub async fn get_devices(&self) -> Result<Vec<AudioDevice>, Box<dyn Error + Send + Sync>> {
        let mut target_devices = Vec::new();

        let output = std::process::Command::new("pactl")
            .args(&["list", "sinks"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut current_id = String::new();
        let mut current_name = String::new();
        let mut current_volume = 0.5;
        let mut current_muted = false;

        for line in stdout.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("Sink #") {

                if !current_id.is_empty() {
                    target_devices.push(AudioDevice {
                        id: current_id.clone(),
                        name: current_name.clone(),
                        volume: current_volume,
                        is_muted: current_muted,
                        is_output: true,
                    });
                }

                if let Some(id_str) = trimmed.split('#').last() {
                    current_id = id_str.trim().to_string();
                }
                current_name.clear();
            }
            else if trimmed.starts_with("Name:") {
                if current_name.is_empty() {
                    if let Some(val) = trimmed.split_once(':') {
                        current_name = val.1.trim()
                            .replace("alsa_output.", "")
                            .replace(".analog-stereo", "")
                            .replace("-", " ");
                    }
                }
            }
            else if trimmed.starts_with("Description:") {
                if let Some(val) = trimmed.split_once(':') {
                    current_name = val.1.trim().to_string();
                }
            }
            else if trimmed.starts_with("Volume:") {
                if let Some(pct_idx) = trimmed.find('%') {
                    let start = &trimmed[..pct_idx];
                    let num_str = start.split_whitespace().last().unwrap_or("50");
                    let parsed_vol = num_str.parse::<f64>().unwrap_or(50.0) / 100.0;
                    current_volume = parsed_vol.clamp(0.0, 1.0);
                }
            }
            else if trimmed.starts_with("Mute:") {
                current_muted = trimmed.contains("yes");
            }
        }

        if !current_id.is_empty() {
            target_devices.push(AudioDevice {
                id: current_id,
                name: current_name,
                volume: current_volume,
                is_muted: current_muted,
                is_output: true,
            });
        }

        if let Ok(mut cache_guard) = self.devices.write() {
            *cache_guard = target_devices.clone();
        }

        Ok(target_devices)
    }

    pub async fn set_volume(&self, device_id: &str, volume: f64) -> Result<(), Box<dyn Error + Send + Sync>> {
        let clamped_volume = volume.clamp(0.0, 1.0);
        let pct = format!("{}%", (clamped_volume * 100.0) as u32);

        std::process::Command::new("pactl")
            .args(&["set-sink-volume", device_id, &pct])
            .output()?;

        Ok(())
    }

    pub async fn set_mute(&self, device_id: &str, is_muted: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mute_param = if is_muted { "1" } else { "0" };

        std::process::Command::new("pactl")
            .args(&["set-sink-mute", device_id, mute_param])
            .output()?;

        Ok(())
    }

    pub async fn set_default_sink(&self, device_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::process::Command::new("pactl")
            .args(&["set-default-sink", device_id])
            .output()?;

        Ok(())
    }
}
