//! `aether-remote` host utility.
//!
//! Subcommands are added per P7 task. `capture` is the first: it exercises the
//! descriptor and copy-audit path without a compositor.

use std::path::PathBuf;

use aether_remote::capture::{
    CaptureFrame, CaptureSource, CopyAuditor, DmaBufFrame, FramePayload, PixelFormat,
    SyntheticSource,
};
use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("capture") | None => {
            if let Err(error) = capture(&args) {
                eprintln!("aether-remote: {error}");
                std::process::exit(1);
            }
        }
        Some("--help" | "-h" | "help") => usage(),
        Some(other) => {
            eprintln!("aether-remote: unknown command {other}");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    println!(
        "aether-remote <command>\n\
         \n\
         commands:\n\
         \x20 capture [--width N] [--height N] [--frames N] [--json PATH]  exercise the capture model\n\
         \x20 help                                                           this message"
    );
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn number<T: std::str::FromStr>(args: &[String], name: &str, fallback: T) -> T {
    flag(args, name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn capture(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let width: u32 = number(args, "--width", 640);
    let height: u32 = number(args, "--height", 360);
    let frames: usize = number(args, "--frames", 4);
    let fps: u32 = number(args, "--fps", 60);
    let mut source = SyntheticSource::new(width, height, fps);
    let mut auditor = CopyAuditor::default();
    let mut captured = Vec::new();
    let mut now_us = 0u64;
    let interval = 1_000_000 / u64::from(fps.max(1));
    while captured.len() < frames {
        if let Some(frame) = source.next_frame(now_us) {
            auditor.record(&frame);
            captured.push(frame);
        }
        now_us += interval / 4;
    }
    let dma = DmaBufFrame::new(width, height, PixelFormat::Bgra, width * 4);
    let dma_frame = CaptureFrame {
        sequence: 1,
        captured_at_us: 0,
        output: 1,
        width,
        height,
        damage: vec![aether_remote::capture::DamageRect::new(0, 0, width, height)],
        payload: FramePayload::DmaBuf(dma.clone()),
    };
    let mut dma_auditor = CopyAuditor::default();
    dma_auditor.record(&dma_frame);
    let summary = json!({
        "source": "synthetic",
        "width": width,
        "height": height,
        "fps": fps,
        "software_auditor": auditor,
        "dma_buf": {
            "descriptor": dma,
            "valid": dma.validate().is_ok(),
            "auditor": dma_auditor,
        },
        "frames": captured.iter().map(|frame| json!({
            "sequence": frame.sequence,
            "captured_at_us": frame.captured_at_us,
            "format": frame.format().label(),
            "zero_copy": frame.is_zero_copy(),
            "copy_bytes": frame.copy_bytes(),
            "damage": frame.damage,
            "damage_area": frame.damage_area(),
        })).collect::<Vec<_>>(),
        "note": "synthetic source; the compositor supplies DMA-BUF planes for the same descriptor"
    });
    let text = serde_json::to_string_pretty(&summary)?;
    if let Some(path) = flag(args, "--json") {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("{text}\n"))?;
        println!("wrote {}", path.display());
    } else {
        println!("{text}");
    }
    println!("capture: {}", auditor.summary());
    Ok(())
}
