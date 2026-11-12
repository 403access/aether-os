---
id: browser
title: Browser
sidebar_position: 11
description: A WebKitGTK shell with per-tab isolation, a minimal permission model and no extension surface.
---

# Browser

The browser is a thin WebKitGTK shell. Aether does not implement a rendering
engine; it removes everything around one and hardens the boundary.

## Why WebKitGTK

| Option | Verdict |
| --- | --- |
| Chromium from scratch | Too large to audit, too much integration work |
| Electron | Ships a second compositor and runtime; disqualified |
| Servo | Promising, but not web-complete enough for daily use |
| WebKitGTK | Complete engine, native Wayland, process separation, C API |

The cost is that WebKit is a large dependency with its own release cadence. The
benefit is that the hard part — rendering the web — is not the project.

## Process model

```text
aether-browser (UI process)
├── WebKit web process   (one per tab)
├── Network process      (shared, sandboxed)
└── GPU process          (shared, sandboxed)
```

Each tab is a separate WebKit web process. A tab crash cannot take down the shell or
other tabs. The browser maps its rendered surface into the compositor as a buffer;
the shell never sees DOM or JavaScript.

Additional Aether sandboxing:

- Each tab runs under bwrap with an empty home, a read-only root and a private
  `/tmp`.
- The browser profile is per-session and disposable; persistent cookies are opt-in.
- Network access is denied unless the tab holds the `net` capability (all normal
  browsing does; local file URLs do not).
- Downloads land in a quarantine directory; there is no auto-open.

## UI

- One window, tabs in the title bar, no bookmarks bar by default.
- Address bar is a palette provider: `⌘L` focuses it, typing searches history and
  the index.
- No extension surface. No Web Store, no native messaging, no plugins.
- Devtools available behind a setting; remote debugging is off.
- Reader mode and per-site permissions are first-class.

## Permissions

| Permission | Default | UX |
| --- | --- | --- |
| Geolocation | deny | prompt (never persisted without explicit “always”) |
| Camera / microphone | deny | prompt with recording indicator |
| Notifications | deny | prompt; routed to shell toasts |
| Clipboard read | deny | prompt |
| Downloads | allow to quarantine | status chip |
| Local file access | deny | explicit file picker |
| WebGL | allow | can be disabled per site |

Permission state lives in the session, not in a hidden profile. A site cannot
re-prompt on every navigation (that trains users to click allow); denials are sticky
for the session.

## Performance

| Metric | Budget |
| --- | --- |
| Cold tab open | < 250 ms |
| RSS per idle tab | ~40–80 MB (engine-dependent) |
| Scrolling | 120 Hz on the compositor |
| Video | hardware decode via VA-API when available |
| Tab crash recovery | < 1 s, no shell impact |

The browser is the largest memory consumer on the machine by design. It is also the
component with the highest attack surface, which is why it is sandboxed and why it is
not the shell.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Web process crash | Tab shows crash surface with reload |
| Network process crash | Tabs show offline; restart in place |
| GPU process crash | Falls back to software rendering; warning badge |
| OOM | Tab is killed; shell and other tabs survive |
| Malicious page | Sandbox + permission model; no shell access |
| Engine update regression | Pin WebKit version; test corpus before upgrade |

## What is deliberately missing

- Extensions and userscripts.
- Sync, accounts, passwords in the cloud.
- A built-in PDF editor (PDF viewing uses the engine).
- Tab groups, workspaces inside the browser (the shell owns workspaces).
- Picture-in-picture outside the shell (media surfaces are shell-managed).

## Verification (P6)

1. Tab crash leaves the shell at 120 Hz.
2. A page cannot read `~/.ssh` even with a `file://` URL (bwrap empty home).
3. Microphone permission denial is enforced and audited.
4. 4K60 video uses hardware decode at < 10% CPU.
5. Engine upgrade passes a pinned page corpus and a memory regression check.
