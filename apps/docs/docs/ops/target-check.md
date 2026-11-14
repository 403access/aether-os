---
id: target-check
title: Target preflight check
sidebar_position: 4
description: Check UEFI, KMS, input, memory and the 7700K before installing or debugging the target.
---

# Target preflight check

`scripts/target-check.sh` validates the firmware, DRM/KMS, input and memory
needed by the Aether compositor. It works on a live system or against a mounted
target root.

```bash
# live system
./scripts/target-check.sh

# before chroot/install, against a mounted target
./scripts/target-check.sh --root /mnt
```

The script exits `0` when the machine is ready and `1` on a blocking failure.
Warnings do not fail the run.

## Checks

| Check | Failure means |
| --- | --- |
| `/sys/firmware/efi` | Legacy boot; the installer expects UEFI |
| CPU model | Warns if it is not an i7-7700K (the reference target) |
| `MemTotal` | Warns below 8 GiB |
| `/proc/cmdline` | `nomodeset` blocks KMS |
| `i915` module | No Intel GPU driver |
| `/dev/dri/card*` | No KMS device for the compositor |
| `/dev/dri/renderD*` | Warns: no render node (software fallback) |
| DRM connector `status` | No connected monitor |
| Connector `modes` | Connected, but no KMS modes |
| `/dev/input/event*` | No keyboard/pointer devices |
| `seat0`/logind | Warns: no seat for libseat/DRM sessions |

Example failure:

```text
FAIL  no /dev/dri/card* (KMS device missing)
FAIL  no connected DRM connector (check cable/monitor)
RESULT: 2 failure(s), 1 warning(s)
```

Run it on the 7700K before opening a compositor bug: a missing DRM node, an
unconnected monitor or `nomodeset` will look like a compositor startup failure.

The script itself is covered by fixtures in `tests/target-check.test.ts`; a run
on the physical machine is still required for final confirmation.
