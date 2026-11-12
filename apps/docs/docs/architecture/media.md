---
id: media
title: Media player
sidebar_position: 12
description: libmpv frontend, hardware decode, audio sync, subtitles and failure behaviour.
---

# Media player

A thin native frontend over libmpv. Playback, synchronisation and codec support are
mpv's job; Aether owns presentation and keyboard control.

## Why libmpv

- Hardware decode (VA-API, NVDEC, VideoToolbox) already implemented.
- Correct A/V sync and frame pacing for arbitrary containers.
- Subtitle rendering, seeking, network streams.
- A stable C API with a render callback for embedding.

Writing a decoder stack is not the project. The custom part is the surface, controls
and integration with the shell.

## Architecture

```text
file / URL → libmpv → render API → GL/Vulkan texture → compositor surface
                     ↘ audio → PipeWire
```

- The player is a sandboxed client with read-only access to the selected file.
- Frames are handed to the compositor as DMA-BUFs where possible; the fallback is a
  shared texture.
- Audio goes through PipeWire, so route switching works exactly like any other app.
- Media keys are handled by the shell and forwarded to the focused player.

## Controls

| Key | Action |
| --- | --- |
| `Space` | Play / pause |
| `←` / `→` | Seek −5 s / +5 s |
| `↑` / `↓` | Volume |
| `F` | Fullscreen |
| `S` | Subtitle track |
| `A` | Audio track |
| `Esc` | Exit fullscreen, then close |

Pointer controls appear on hover and auto-hide after two seconds of inactivity. They
never animate while paused.

## Performance

| Metric | Budget |
| --- | --- |
| 4K60 H.264 decode | < 6% CPU with VA-API |
| Seek latency | < 150 ms to first frame |
| A/V drift | < 20 ms over 2 hours |
| Memory | ~120–200 MB with a 4K buffer pool |
| Startup | < 300 ms to first frame |

The player is allowed to use memory proportional to the decode buffer; it is not a
background process and exits when closed.

## Subtitles

- External `.srt`/`.ass` picked up from the same directory.
- Embedded tracks enumerated.
- Font rendering by libass; no custom subtitle engine.
- Styling is intentionally plain: outline + shadow, no per-word animation.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Unsupported codec | Error toast with the codec name; no crash |
| GPU decode failure | Fallback to software decode with a badge |
| File disappears | Playback stops; toast names the path |
| Audio device changes | PipeWire moves the stream; playback continues |
| Malformed container | mpv exits; shell shows a recoverable error |
| HDR content on SDR output | Tone mapping on by default; toggle in overlay |

## Out of scope

- A media library, scraping, or metadata management.
- Streaming service integrations.
- Transcoding.
- Background playback after the player closes.

## Verification (P6)

1. 4K60 H.264 and HEVC playback with hardware decode on the reference GPU.
2. Ten seeks in a row with no audio desync.
3. Route switch mid-playback with no gap.
4. A corrupt file produces an error surface, not a shell crash.
