---
id: headless
title: Run headless (no UI)
sidebar_position: 5
description: "Run Aether as a display-less server: keep the index, MCP, sandboxes and audio, drop the compositor."
---

# Run headless (no UI)

Aether has no desktop environment and no display manager. The entire UI is one
systemd service:

- `aether-session` (`os/nixos/services.nix`) starts `aether-shell-drm` on tty7,
  with `cage` as the fallback adapter.

Everything else — kernel, networking, PipeWire, `aether-index`, `aether-mcp`,
`aether-sandbox`, `aether-chat` — runs without a display. That makes a
display-less install a configuration change, not a port.

## Enable the profile

`os/nixos/headless.nix` is imported by `configuration.nix` and off by default.
Turn it on in `os/nixos/configuration.nix` and rebuild:

```nix
aether.headless.enable = true;
```

```bash
sudo nixos-rebuild switch --flake .#kestrel
```

What the module changes:

| Setting | Effect |
| --- | --- |
| `systemd.services.aether-session` | no longer pulled at boot; the unit stays installed |
| `aether-clip-persist` | not started (Wayland-only helper) |
| `services.getty.autologinUser` | reset to `null`; the console asks for a password |
| `users.users.oliver.linger` | enabled, so `aether-index` and `aether-chat` start at boot without a login |
| `services.openssh` | enabled (`aether.headless.ssh = true`, default) |
| `networking.firewall` | stays enabled; only TCP 22 is opened |
| `boot.kernelParams` | adds `console=ttyS0,115200` (`aether.headless.serialConsole = true`, default) |

Neither option weakens the login: SSH password and keyboard-interactive
authentication stay disabled, and root cannot log in. Add a key before you rely
on remote access:

```nix
users.users.oliver.openssh.authorizedKeys.keys = [
  "ssh-ed25519 AAAA... you@workstation"
];
```

Disable the parts you do not need:

```nix
aether.headless = {
  enable = true;
  ssh = false;            # console only
  serialConsole = false;  # no serial getty
};
```

## Working on a headless box

| Task | How |
| --- | --- |
| Remote shell | `ssh oliver@kestrel` |
| Local console | tty1, or the serial line when `serialConsole` is on |
| Index daemon | `systemctl --user status aether-index` |
| Chat backend | set `AETHER_CHAT_ENDPOINT` in `~/.config/aether/chat.env`, then `systemctl --user status aether-chat` |
| Sandboxes / MCP | run `aether-sandbox` / `aether-mcp` directly; both are socket/CLI tools and need no display |

With lingering enabled, the user manager keeps `XDG_RUNTIME_DIR=/run/user/1000`
alive, so `%t/aether-index.sock` and `%t/aether-chat.sock` exist before you log
in.

## Limits

- **There is no headless compositor backend.** `aether-shell-drm` needs
  `/dev/dri` and a connected connector; without a monitor it logs
  `0 usable output(s)` and paints nothing. The `cage` fallback needs a seat and
  DRM as well. Do not expose the UI on a machine that has no display path.
- **Remote display is not headless.** `aether-remote` (`Super+R`) encodes frames
  produced by the compositor, so it requires an `aether-session` to run. It is
  opt-in, binds `127.0.0.1:7443`, and is not started by this profile.
- **Temporary local UI.** The unit is not masked, so
  `sudo systemctl start aether-session` still brings up tty7 on demand.
- **First boot still applies.** Log in as `oliver` and run `passwd` before
  exposing SSH, see [first boot](/docs/getting-started/first-boot).