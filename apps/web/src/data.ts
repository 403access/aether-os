import type { IndexDocument } from "@aether/protocol";
import type { PaletteItem } from "@aether/palette";
import type { ExplorerEntry } from "@aether/ui";

export const withBase = (path: string): string =>
  `${import.meta.env.BASE_URL}${path.replace(/^\//, "")}`;

const DOCS_BASE = import.meta.env.DEV
  ? "http://localhost:4322/docs/"
  : withBase("docs/");

export const LINKS = {
  docs: DOCS_BASE,
  architecture: `${DOCS_BASE}architecture/overview/`,
  install: `${DOCS_BASE}getting-started/install/`,
  roadmap: `${DOCS_BASE}roadmap/`,
  designSystem: import.meta.env.DEV ? "http://localhost:8787/" : withBase("design-system/"),
  github: "https://github.com/olivermolnar/aether-os",
};

const DAY = 86_400_000;
const NOW = Date.UTC(2026, 10, 12);

/**
 * The demo corpus. Deliberately small: the palette in the hero should feel
 * instant, and every result is a real item from the documented architecture.
 */
export const DEMO_DOCUMENTS: IndexDocument[] = [
  { id: "app:terminal", kind: "app", title: "Terminal", subtitle: "Native PTY · foot", uses: 42, modifiedAt: NOW - DAY },
  { id: "app:browser", kind: "app", title: "Browser", subtitle: "WebKitGTK shell", uses: 31, modifiedAt: NOW - DAY },
  { id: "app:media", kind: "app", title: "Media Player", subtitle: "libmpv · hardware decode", uses: 12, modifiedAt: NOW - 4 * DAY },
  { id: "app:files", kind: "app", title: "Files", subtitle: "Two-pane explorer", uses: 18, modifiedAt: NOW - 2 * DAY },
  { id: "app:chat", kind: "app", title: "Aether Chat", subtitle: "DeepSeek · streaming", uses: 64, modifiedAt: NOW },
  { id: "app:settings", kind: "setting", title: "Settings", subtitle: "System preferences", uses: 9, modifiedAt: NOW - 6 * DAY },
  { id: "setting:audio", kind: "setting", title: "Audio Output", subtitle: "PipeWire · WirePlumber", uses: 22, modifiedAt: NOW - DAY },
  { id: "setting:scaling", kind: "setting", title: "Display Scaling", subtitle: "Compositor · fractional scale", uses: 4, modifiedAt: NOW - 9 * DAY },

  { id: "command:palette", kind: "command", title: "Open Command Palette", subtitle: "⌘K", uses: 120, modifiedAt: NOW },
  { id: "command:agent", kind: "command", title: "New Agent Session", subtitle: "Spawn an isolated worker", uses: 27, modifiedAt: NOW },
  { id: "command:audio-toggle", kind: "command", title: "Toggle Audio Output", subtitle: "Speakers ⇄ Headphones", uses: 16, modifiedAt: NOW - DAY },
  { id: "command:rebuild-index", kind: "command", title: "Rebuild Search Index", subtitle: "aether-index · FTS5", uses: 5, modifiedAt: NOW - 3 * DAY },
  { id: "command:capture", kind: "command", title: "Capture Screen Region", subtitle: "DMA-BUF · zero copy", uses: 8, modifiedAt: NOW - 2 * DAY },
  { id: "command:lock", kind: "command", title: "Lock Session", subtitle: "aether-shell", uses: 2, modifiedAt: NOW - 20 * DAY },

  { id: "agent:refactor", kind: "agent", title: "Refactor Worker", subtitle: "Untrusted · Landlock process", uses: 14, modifiedAt: NOW - DAY },
  { id: "agent:research", kind: "agent", title: "Research Agent", subtitle: "Untrusted · read-only index", uses: 7, modifiedAt: NOW - 2 * DAY },
  { id: "agent:test-runner", kind: "agent", title: "Test Runner", subtitle: "Hostile · Firecracker microVM", uses: 3, modifiedAt: NOW - 5 * DAY },

  { id: "file:indexer", kind: "file", title: "indexer.rs", subtitle: "/work/aether/indexer/src", path: "/work/aether/indexer/src/indexer.rs", tags: ["rs"], uses: 33, modifiedAt: NOW - DAY },
  { id: "file:compositor", kind: "file", title: "compositor.rs", subtitle: "/work/aether/shell/src", path: "/work/aether/shell/src/compositor.rs", tags: ["rs"], uses: 51, modifiedAt: NOW },
  { id: "file:ipc", kind: "file", title: "ring.rs", subtitle: "/work/aether/ipc/src", path: "/work/aether/ipc/src/ring.rs", tags: ["rs"], uses: 29, modifiedAt: NOW - DAY },
  { id: "file:kernel", kind: "file", title: "kestrel.config", subtitle: "/work/aether/os/kernel", path: "/work/aether/os/kernel/kestrel.config", tags: ["config"], modifiedAt: NOW - 8 * DAY },
  { id: "file:readme", kind: "file", title: "README.md", subtitle: "/work/aether", path: "/work/aether/README.md", tags: ["md"], uses: 12, modifiedAt: NOW - DAY },
  { id: "file:notes", kind: "file", title: "2026-11-11-agent-isolation.md", subtitle: "/home/oliver/notes", path: "/home/oliver/notes/2026-11-11-agent-isolation.md", tags: ["md"], modifiedAt: NOW },
  { id: "file:recording", kind: "file", title: "standup-2026-11-11.wav", subtitle: "/home/oliver/media", path: "/home/oliver/media/standup-2026-11-11.wav", tags: ["wav"], modifiedAt: NOW - 2 * DAY },
  { id: "file:bench", kind: "file", title: "baseline.json", subtitle: "/work/aether/bench", path: "/work/aether/bench/baseline.json", tags: ["json"], modifiedAt: NOW - DAY },
];

export const DEMO_APPS: PaletteItem[] = DEMO_DOCUMENTS.filter((doc) => doc.kind === "app").map(
  (doc) => ({
    id: doc.id,
    title: doc.title,
    subtitle: doc.subtitle,
    category: "Applications",
    icon: "▤",
    weight: 0.3,
    action: `launch:${doc.id}`,
  }),
);

export const DEMO_COMMANDS: PaletteItem[] = [
  {
    id: "command:new-workspace",
    title: "New Workspace",
    subtitle: "Tiling layout · 3 columns",
    category: "Commands",
    icon: "⌘",
    action: "workspace:new",
  },
  {
    id: "command:remote-session",
    title: "Start Remote Session",
    subtitle: "WebRTC · VA-API · AV1",
    category: "Commands",
    icon: "⌘",
    action: "stream:start",
  },
  {
    id: "command:spawn-agent",
    title: "Spawn Isolated Agent",
    subtitle: "Landlock process · 512 MiB cap",
    category: "Commands",
    icon: "⌘",
    action: "agent:spawn",
  },
];

const status = {
  local: { label: "Local only", tone: "local" as const },
  same: { label: "Same", tone: "same" as const },
  partial: { label: "Partial", tone: "partial" as const },
  missing: { label: "Missing", tone: "missing" as const },
  moved: { label: "Moved", tone: "moved" as const },
};

/**
 * Filesystem fixture for the explorer demo. Keys are absolute paths; folders
 * exist when a matching key is present.
 */
export const EXPLORER: Record<string, ExplorerEntry[]> = {
  "/home/oliver": [
    { id: "d:projects", name: "projects", type: "folder", status: status.local, modified: "Nov 12" },
    { id: "d:media", name: "media", type: "folder", status: status.local, modified: "Nov 11" },
    { id: "d:notes", name: "notes", type: "folder", status: status.local, modified: "Nov 12" },
    { id: "f:aether-toml", name: "aether.toml", type: "file", status: status.local, size: "2.1 KB", modified: "Nov 10" },
    { id: "f:recording", name: "standup-2026-11-11.wav", type: "file", status: status.local, size: "18.4 MB", modified: "Nov 11" },
    { id: "f:todos", name: "todos.md", type: "file", status: status.local, size: "4.2 KB", modified: "Nov 12" },
    { id: "f:screen", name: "screen-2026-11-12.png", type: "file", status: status.local, size: "1.8 MB", modified: "Nov 12" },
  ],
  "/home/oliver/projects": [
    { id: "d:aether", name: "aether-os", type: "folder", status: status.same, modified: "Nov 12" },
    { id: "d:indexer", name: "indexer", type: "folder", status: status.same, modified: "Nov 11" },
    { id: "d:screenshots", name: "screenshots", type: "folder", status: status.local, modified: "Nov 9" },
    { id: "f:bench", name: "baseline.json", type: "file", status: status.same, size: "1.2 KB", modified: "Nov 12" },
    { id: "f:notes", name: "design-notes.md", type: "file", status: status.same, size: "9.6 KB", modified: "Nov 8" },
  ],
  "/home/oliver/projects/aether-os": [
    { id: "d:shell", name: "shell", type: "folder", status: status.same, modified: "Nov 12" },
    { id: "d:ipc", name: "ipc", type: "folder", status: status.same, modified: "Nov 12" },
    { id: "d:docs", name: "docs", type: "folder", status: status.same, modified: "Nov 12" },
    { id: "f:readme", name: "README.md", type: "file", status: status.same, size: "12.4 KB", modified: "Nov 12" },
    { id: "f:roadmap", name: "roadmap.md", type: "file", status: status.same, size: "22.8 KB", modified: "Nov 12" },
  ],
  "/home/oliver/media": [
    { id: "f:standup", name: "standup-2026-11-11.wav", type: "file", status: status.local, size: "18.4 MB", modified: "Nov 11" },
    { id: "f:demo", name: "compositor-demo.mp4", type: "file", status: status.moved, size: "184 MB", modified: "Nov 10" },
    { id: "f:cover", name: "kestrel-cover.png", type: "file", status: status.local, size: "3.4 MB", modified: "Nov 6" },
  ],
  "/home/oliver/notes": [
    { id: "f:isolation", name: "2026-11-11-agent-isolation.md", type: "file", status: status.local, size: "14.2 KB", modified: "Nov 11" },
    { id: "f:water", name: "2026-11-09-water-design.md", type: "file", status: status.local, size: "6.8 KB", modified: "Nov 9" },
  ],
  "/mnt/archive": [
    { id: "d:2026", name: "2026-11-12", type: "folder", status: status.partial, modified: "Nov 12" },
    { id: "d:documents", name: "Documents", type: "folder", status: status.same, modified: "Nov 11" },
    { id: "d:photos", name: "Photos", type: "folder", status: status.missing, modified: "Nov 10" },
    { id: "d:backups", name: "Backups", type: "folder", status: status.partial, modified: "Nov 8" },
    { id: "f:archive-json", name: "archive.json", type: "file", status: status.same, size: "318 MB", modified: "Nov 9" },
    { id: "f:index-db", name: "file_indexer.db", type: "file", status: status.same, size: "1.5 GB", modified: "Nov 11" },
    { id: "f:sync-log", name: "sync.log", type: "file", status: status.partial, size: "84 KB", modified: "Nov 12" },
  ],
  "/mnt/archive/2026-11-12": [
    { id: "d:exports", name: "exports", type: "folder", status: status.partial, modified: "Nov 12" },
    { id: "f:manifest", name: "manifest.json", type: "file", status: status.same, size: "6.2 KB", modified: "Nov 12" },
    { id: "f:checksums", name: "checksums.txt", type: "file", status: status.partial, size: "142 KB", modified: "Nov 12" },
  ],
  "/mnt/archive/Documents": [
    { id: "f:contract", name: "contract-2026.pdf", type: "file", status: status.same, size: "2.4 MB", modified: "Nov 2" },
    { id: "f:invoice", name: "invoice-2026-11.pdf", type: "file", status: status.same, size: "184 KB", modified: "Nov 1" },
    { id: "f:notes", name: "meeting-notes.md", type: "file", status: status.same, size: "12 KB", modified: "Nov 10" },
  ],
};
