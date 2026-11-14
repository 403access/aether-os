//! `aether-audio` CLI: inspect routes, switch defaults, choose latency
//! presets and read xrun counters from the live PipeWire graph.

use std::process::ExitCode;

use aether_audio::{AudioSnapshot, LatencyPreset, RouteKind, SystemAudio};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aether-audio: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let audio = SystemAudio::new();
    match args.first().map(String::as_str) {
        None | Some("list") => {
            let snapshot = audio.snapshot()?;
            if args.iter().any(|arg| arg == "--json")
                || args.get(1).is_some_and(|arg| arg == "json")
            {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                print_routes(&snapshot);
            }
            Ok(())
        }
        Some("status") => {
            let snapshot = audio.snapshot()?;
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
            Ok(())
        }
        Some("switch") => {
            let target = args.get(1).ok_or("usage: aether-audio switch <id|name>")?;
            let snapshot = audio.snapshot()?;
            let route = resolve_route(&snapshot, target)?;
            audio.switch(route.kind, route.id)?;
            println!(
                "aether-audio: default {} -> {} ({})",
                route.kind.label().to_lowercase(),
                route.label(),
                route.id
            );
            Ok(())
        }
        Some("preset") => {
            let target = args
                .get(1)
                .ok_or("usage: aether-audio preset <media|voice|monitor|next>")?;
            let snapshot = audio.snapshot()?;
            let preset = if target == "next" {
                snapshot.preset.next()
            } else {
                LatencyPreset::parse(target)
                    .ok_or_else(|| format!("unknown latency preset: {target}"))?
            };
            audio.set_preset(preset)?;
            println!(
                "aether-audio: preset -> {} ({} frames)",
                preset.name(),
                preset.quantum()
            );
            Ok(())
        }
        Some("xruns") => {
            println!("{}", audio.xruns()?);
            Ok(())
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other} (try --help)").into()),
    }
}

fn resolve_route<'a>(
    snapshot: &'a AudioSnapshot,
    target: &str,
) -> Result<&'a aether_audio::AudioRoute, Box<dyn std::error::Error>> {
    if let Ok(id) = target.parse::<u32>() {
        return snapshot
            .routes
            .iter()
            .find(|route| route.id == id)
            .ok_or_else(|| format!("route id {id} not found").into());
    }
    let needle = target.to_ascii_lowercase();
    snapshot
        .routes
        .iter()
        .find(|route| {
            route.name.to_ascii_lowercase().contains(&needle)
                || route.label().to_ascii_lowercase().contains(&needle)
        })
        .ok_or_else(|| format!("route {target:?} not found").into())
}

fn print_routes(snapshot: &AudioSnapshot) {
    println!(
        "preset {} ({} frames) · {} xruns · {}",
        snapshot.preset.name(),
        snapshot.preset.quantum(),
        snapshot.xruns,
        match snapshot.source {
            aether_audio::SnapshotSource::Live => "live",
            aether_audio::SnapshotSource::Fixture => "fixture",
        }
    );
    for kind in [RouteKind::Output, RouteKind::Input] {
        println!("\n{}", kind.label());
        let mut any = false;
        for route in snapshot.routes.iter().filter(|route| route.kind == kind) {
            any = true;
            let marker = if route.default { "* " } else { "  " };
            let codec = route
                .codec
                .as_deref()
                .map(|codec| format!(" · {codec}"))
                .unwrap_or_default();
            let profile = if route.profile == "default" {
                String::new()
            } else {
                format!(" · {}", route.profile)
            };
            println!(
                "{marker}{:>3}  {}  [{}]{} {}{}",
                route.id,
                route.label(),
                route.transport.label(),
                profile,
                if route.available {
                    ""
                } else {
                    " · unavailable"
                },
                codec
            );
        }
        if !any {
            println!("  (none)");
        }
    }
}

fn print_help() {
    println!(
        "aether-audio — PipeWire route policy\n\
         \n\
         Usage:\n\
           aether-audio list [--json]       routes and active defaults\n\
           aether-audio status              JSON snapshot\n\
           aether-audio switch <id|name>    set default sink/source\n\
           aether-audio preset <name|next>  media | voice | monitor\n\
           aether-audio xruns               non-zero pw-top error rows\n\
         \n\
         Environment:\n\
           AETHER_AUDIO_FIXTURE=<path>      read a JSON snapshot instead of PipeWire\n\
           XDG_STATE_HOME                   preset state location\n"
    );
}
