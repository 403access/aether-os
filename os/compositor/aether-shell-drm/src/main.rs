//! Native DRM bring-up probe.
//!
//! This binary currently validates KMS device access and connector/mode
//! enumeration without libseat. The full compositor backend (libinput,
//! DrmCompositor page-flip and shared Wayland handlers) is the next layer.

use std::error::Error;
use std::fs::OpenOptions;
use std::os::fd::OwnedFd;

use smithay::backend::drm::{DrmDevice, DrmDeviceFd};
use smithay::reexports::drm::control::{connector, Device as ControlDevice};
use smithay::utils::DeviceFd;

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::var("AETHER_DRM_DEVICE").unwrap_or_else(|_| "/dev/dri/card0".to_string());
    let file = OpenOptions::new().read(true).write(true).open(&path)?;
    let fd = DrmDeviceFd::new(DeviceFd::from(OwnedFd::from(file)));
    let (drm, _notifier) = DrmDevice::new(fd, false)?;
    let resources = drm.resource_handles()?;

    println!("device: {path}");
    println!("connectors: {}", resources.connectors().len());
    println!("crtcs: {}", resources.crtcs().len());

    let mut connected = 0;
    let mut modes = 0;
    for handle in resources.connectors() {
        let info = drm.get_connector(*handle, false)?;
        let state = match info.state() {
            connector::State::Connected => "connected",
            connector::State::Disconnected => "disconnected",
            connector::State::Unknown => "unknown",
        };
        println!(
            "connector {info}: {state}, encoders={}, modes={}",
            info.encoders().len(),
            info.modes().len()
        );
        if info.state() == connector::State::Connected {
            connected += 1;
            for mode in info.modes().iter().take(4) {
                let (w, h) = mode.size();
                println!(
                    "  mode {} {}x{}@{}Hz flags=0x{:x}",
                    mode.name().to_string_lossy(),
                    w,
                    h,
                    mode.vrefresh(),
                    mode.flags().bits()
                );
                modes += 1;
            }
        }
    }

    if connected == 0 {
        return Err("no connected DRM connector".into());
    }
    if modes == 0 {
        return Err("connected connectors expose no modes".into());
    }
    println!("RESULT: ready");
    Ok(())
}
