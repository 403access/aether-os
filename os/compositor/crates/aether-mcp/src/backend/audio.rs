//! Audio route and volume probes.

use std::process::Command;

use serde_json::{json, Value};

pub fn status() -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("aether-audio").arg("status").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    return Ok(json!({"source": "aether-audio", "snapshot": value}));
                }
            }
        }
        let output = Command::new("wpctl")
            .args(["status"])
            .output()
            .map_err(|error| format!("wpctl unavailable: {error}"))?;
        if !output.status.success() {
            return Err("wpctl status failed".into());
        }
        return Ok(json!({
            "source": "wpctl",
            "status": String::from_utf8_lossy(&output.stdout),
        }));
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .args(["-e", "get volume settings"])
            .output()
            .map_err(|error| format!("osascript unavailable: {error}"))?;
        if !output.status.success() {
            return Err("cannot read macOS volume".into());
        }
        return Ok(json!({
            "source": "osascript",
            "settings": String::from_utf8_lossy(&output.stdout).trim(),
        }));
    }
    #[allow(unreachable_code)]
    Err("audio status is unsupported on this platform".into())
}

pub fn set_volume(percent: u64) -> Result<Value, String> {
    let percent = percent.min(150);
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{percent}%")])
            .output()
            .map_err(|error| format!("wpctl unavailable: {error}"))?;
        if !output.status.success() {
            return Err("wpctl set-volume failed".into());
        }
        return Ok(json!({"volume_percent": percent, "via": "wpctl"}));
    }
    #[cfg(target_os = "macos")]
    {
        let volume = percent.min(100);
        let output = Command::new("osascript")
            .args(["-e", &format!("set volume output volume {volume}")])
            .output()
            .map_err(|error| format!("osascript unavailable: {error}"))?;
        if !output.status.success() {
            return Err("osascript set volume failed".into());
        }
        return Ok(json!({"volume_percent": volume, "via": "osascript"}));
    }
    #[allow(unreachable_code)]
    Err("volume control is unsupported on this platform".into())
}
