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

## Implementation

| Piece | Location |
| --- | --- |
| Route model, transports, presets, xrun parser | `os/compositor/crates/aether-audio` |
| PipeWire/WirePlumber adapter | `aether-audio/src/pipewire.rs` (`pw-dump`, `wpctl`, `pw-metadata`, `pw-top`) |
| CLI | `aether-audio list`, `switch`, `preset`, `xruns` |
| Shell overlay | `aether-shell-wayland` audio layer (`Super+A`) |
| Service policy | `os/nixos/services.nix` |

The crate contains no PipeWire headers or bindings. It consumes `pw-dump` JSON
and `wpctl` ids, which keeps the shell testable in CI and makes every policy
decision inspectable from a terminal:

```bash
aether-audio list
aether-audio switch 44
aether-audio preset voice
aether-audio xruns
```

`AETHER_AUDIO_FIXTURE` points the adapter at a JSON snapshot. The shell smokes use
it so overlay behaviour is deterministic without a live sound server. Preset
choice is stored under `$XDG_STATE_HOME/aether/audio/preset`.

![Audio overlay with outputs, inputs, presets and input meter](/img/smithay-audio.png)

*`Super+A` opens the compositor audio layer. The default route carries a labelled
pill; transport and profile are text, never colour alone. `P` cycles the latency
preset and `Enter` switches the selected route.*

## Overlay

The overlay is a shell layer, not a client. It lists up to four outputs and three
inputs, keeps selection on the default output when opened, and closes with `Esc`.
`↑`/`↓` move, `Enter` applies the route, `P` cycles the preset, and pointer clicks
hit the same rows and preset chips.

With `AETHER_AUDIO_FIXTURE` set, activation updates local state without touching
PipeWire; this is the deterministic path used by `smoke-audio-xvfb.sh`. Live mode
calls `wpctl set-default` and `pw-metadata` before the next snapshot.

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
- An agent cannot open the microphone with the legacy `audio` scope. Capture needs
  a short-lived `audio.capture` capability for `audio:input`, verified by
  `MicrophoneGate` before the stream opens. The status bar shows a recording
  indicator that cannot be hidden. The P5 supervisor wires the gate to the actual
  process launch; the denial/grant path is tested in `packages/agents`.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Device unplugged | Streams move to the fallback default; toast names the device |
| Bluetooth disconnect | Reconnect with backoff; overlay keeps the route visible but unavailable |
| PipeWire crash | systemd restarts it; streams are recreated; the shell shows a brief audio-off badge |
| xrun | Counter increments; the overlay shows a warning only after a threshold |
| Agent requests mic | Denied unless granted; denial is audited |
| Sample-rate mismatch | PipeWire resamples; the overlay reports the active rate |

## Microphone capability

```ts
const gate = new MicrophoneGate(capabilities);
const grant = gate.issue("agent-a", 60_000); // explicit, short-lived
gate.authorize("agent-a", grant.id);         // → { ok: true }
gate.authorize("agent-a");                   // → { ok: false, reason: "missing" }
```

`audio.capture` is separate from `audio`; a broad playback capability never
silently becomes a microphone grant. Both outcomes append to the hash-chained
audit log.

## Configuration

- PipeWire/WirePlumber rules live in `os/nixos/services.nix`.
- 48 kHz graph, 1024-frame default quantum, 64-frame floor (monitoring preset),
  2048-frame ceiling, `link.max-buffers=16`.
- Bluetooth is enabled at boot; WirePlumber prefers LDAC → aptX-HD → aptX → AAC →
  SBC and enables SBC-XQ, mSBC and hardware volume when advertised.
- `aether-audio` is installed by the target and VM profiles.
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
