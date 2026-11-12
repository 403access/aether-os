---
id: audio
title: Audio
sidebar_position: 10
description: PipeWire graph, device switching, Bluetooth, latency targets and failure modes.
---

# Audio

Audio is PipeWire + WirePlumber, exposed through a shell overlay and a small policy
daemon. No PulseAudio, no per-application sound servers, no resampling in the
default path.

## Graph

```text
application → PipeWire stream → graph → device (analog / USB / Bluetooth)
                    ↑
             WirePlumber policy
                    ↑
             aether-audio (route policy + UI state)
```

| Component | Role |
| --- | --- |
| PipeWire | Graph, scheduling, format negotiation, device nodes |
| WirePlumber | Session policy: default devices, per-app routing, Bluetooth |
| `aether-audio` | Aether policy: naming, presets, switching rules, overlay state |
| Shell overlay | Device picker, per-app volume, input meter |

## Device model

Every route is described by:

```ts
type AudioRoute = {
  id: string;
  kind: "output" | "input";
  label: string;          // "Speakers · Analog", "AirPods Pro"
  transport: "analog" | "usb" | "bluetooth" | "hdmi";
  profile: string;        // e.g. a2dp-sink, analog-stereo
  default: boolean;
  available: boolean;
};
```

The overlay groups by kind, marks the default, and shows transport. Status is text
plus icon; colour alone never indicates the active route.

## Switching

`aether-audio` subscribes to PipeWire registry events and writes the default device
through WirePlumber:

1. User selects a route (overlay, `⌘⇧A` cycle, or a script).
2. Policy validates that the route is available.
3. WirePlumber moves existing streams to the new default.
4. The overlay updates from the registry event, not from an optimistic guess.

Existing streams keep playing. Applications that hard-code a device are reported as
“pinned” and left alone.

**Budget:** switch < 100 ms, no audible click. Measured in P4.

## Latency

| Path | Quantum | Target |
| --- | --- | --- |
| Media playback | 1024 frames | glitch-free, power efficient |
| Voice / calls | 256 frames | < 20 ms added latency |
| Monitoring | 128 frames | < 10 ms, opt-in |

The default is not the lowest possible quantum. A 64-frame buffer on a 4-core
machine trades CPU headroom for latency most users do not need. The overlay exposes
the trade-off; the policy ships conservative defaults.

## Bluetooth

- Codec preference: LDAC → aptX-HD → aptX → AAC → SBC, limited by the device.
- `aether-audio` reports the negotiated codec in the overlay; no guessing.
- Switching profiles (A2DP ↔ HFP) is explicit, because it changes quality and
  latency.
- Reconnect storms are debounced; the overlay shows “reconnecting” rather than
  flapping the default device.

## Inputs

- Input devices are enumerated separately; the overlay shows level meters.
- Noise suppression and echo cancellation are PipeWire filters, off by default.
- An agent cannot open the microphone without the `audio` capability and an explicit
  grant; the status bar shows a recording indicator that cannot be hidden.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Device unplugged | Streams move to the fallback default; toast names the device |
| Bluetooth disconnect | Reconnect with backoff; overlay keeps the route visible but unavailable |
| PipeWire crash | systemd restarts it; streams are recreated; the shell shows a brief audio-off badge |
| xrun | Counter increments; the overlay shows a warning only after a threshold |
| Agent requests mic | Denied unless granted; denial is audited |
| Sample-rate mismatch | PipeWire resamples; the overlay reports the active rate |

## Configuration

- WirePlumber rules live in `os/nixos/services.nix`.
- Per-app routing presets are TOML in `~/.config/aether/audio.toml`.
- The daemon validates presets at load; invalid rules are ignored with a warning.
- No UI for creating PipeWire graphs. The graph is an implementation detail; the
  user manages routes, not nodes.

## Why not a custom audio server?

PipeWire already provides the hard parts: a real-time graph, device hotplug,
Bluetooth codecs, per-client routing and JACK compatibility. Writing another server
would add risk and latency for no measurable gain. The custom layer is policy and
presentation, which is where Aether's design actually differs.

## Verification (P4)

1. Route switch < 100 ms across analog, USB and Bluetooth.
2. 64-frame stress test for 10 minutes with zero xruns on the reference machine.
3. Input/output meters match `pw-top` within 5%.
4. Agent microphone access is denied without a capability, proven by test.
