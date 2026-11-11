/**
 * Aether protocol: the shared vocabulary between the shell, daemons, agents and
 * the web surfaces.
 *
 * Design rules:
 *  - Every value that crosses a process boundary has a documented range.
 *  - Times are epoch milliseconds (integers), never locale strings.
 *  - Highlight ranges are half-open `[start, end)` over UTF-16 code units, which
 *    is what the JS/React text layer consumes. Rust emits the same convention.
 */

export const PROTOCOL_VERSION = 1;

/** Ring-buffer magic: "AETH". Stored little-endian in the 64-byte ring header. */
export const AETHER_RING_MAGIC = 0x41455448;
export const RING_HEADER_BYTES = 64;

// ---------------------------------------------------------------------------
// Index documents
// ---------------------------------------------------------------------------

export type DocumentKind =
  | "app"
  | "command"
  | "file"
  | "content"
  | "setting"
  | "agent"
  | "contact";

export const DOCUMENT_KINDS: readonly DocumentKind[] = [
  "app",
  "command",
  "file",
  "content",
  "setting",
  "agent",
  "contact",
];

/**
 * Prior score added before textual relevance. Tuned so that a command typed
 * exactly beats a fuzzy file match, without letting priors dominate.
 */
export const KIND_PRIOR: Readonly<Record<DocumentKind, number>> = {
  command: 0.35,
  app: 0.3,
  agent: 0.2,
  setting: 0.12,
  file: 0.15,
  content: 0.1,
  contact: 0.05,
};

export interface IndexDocument {
  /** Stable identifier, usually a path or an app id. Must be unique. */
  id: string;
  kind: DocumentKind;
  title: string;
  subtitle?: string;
  /** Extracted text content, capped by the indexer before insertion. */
  body?: string;
  /** Absolute path for filesystem-backed documents. */
  path?: string;
  tags?: readonly string[];
  /** Last modification, epoch ms. Drives recency ranking. */
  modifiedAt?: number;
  /** Usage count. Drives frecency ranking. */
  uses?: number;
}

/** Half-open range over UTF-16 code units: `[start, end)`. */
export interface HighlightRange {
  start: number;
  end: number;
}

export type MatchSource = "exact" | "prefix" | "fuzzy" | "fragment";

export interface SearchHit {
  id: string;
  kind: DocumentKind;
  title: string;
  subtitle?: string;
  path?: string;
  score: number;
  source: MatchSource;
  /** Highlight ranges over `title`, sorted and non-overlapping. */
  highlights: HighlightRange[];
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

export type CapabilityScope =
  | "fs.read"
  | "fs.write"
  | "net"
  | "exec"
  | "display"
  | "audio"
  | "infer"
  | "agent.spawn"
  | "agent.collab";

export const CAPABILITY_SCOPES: readonly CapabilityScope[] = [
  "fs.read",
  "fs.write",
  "net",
  "exec",
  "display",
  "audio",
  "infer",
  "agent.spawn",
  "agent.collab",
];

export interface Capability {
  id: string;
  /** Agent id that holds the capability. */
  subject: string;
  scope: CapabilityScope;
  /** Resource pattern: exact path/name, `prefix/*`, or `*`. */
  resource: string;
  issuedAt: number;
  expiresAt: number | null;
  delegable: boolean;
  /** Incremented on revocation so stale holders are rejected cheaply. */
  generation: number;
  parentId: string | null;
}

export type VerifyFailure =
  | "unknown"
  | "revoked"
  | "expired"
  | "scope"
  | "resource"
  | "subject"
  | "generation";

export interface VerificationResult {
  ok: boolean;
  reason?: VerifyFailure;
}

// ---------------------------------------------------------------------------
// Agents and isolation
// ---------------------------------------------------------------------------

export type IsolationTier = "in-process" | "process-landlock" | "microvm";

export interface CapabilityRequest {
  scope: CapabilityScope;
  resource: string;
  ttlMs?: number;
}

export interface AgentSpec {
  id: string;
  name: string;
  trust: "trusted" | "untrusted" | "hostile";
  /** Requested tier. The planner may refuse or escalate it. */
  tier: IsolationTier;
  /** Relative CPU share, 1..100. */
  cpuWeight: number;
  memoryMiB: number;
  capabilities: CapabilityRequest[];
  /** OCI/microVM image id, required for the microvm tier. */
  image?: string;
  /** Hard ceiling for the agent's wall clock, ms. */
  timeoutMs?: number;
}

export type Enforcement = "pku" | "landlock-seccomp" | "microvm-vfio";

export interface IsolationPlan {
  agentId: string;
  tier: IsolationTier;
  enforcement: Enforcement;
  /** Human-readable reasons, in decision order. */
  rationale: string[];
  warnings: string[];
  /** True when the plan needs root/CAP_SYS_ADMIN on the host. */
  privileged: boolean;
}

export interface HardwareProfile {
  id: string;
  cpu: string;
  microarchitecture: string;
  cores: number;
  threads: number;
  baseClockGhz: number;
  turboClockGhz: number;
  l3CacheMiB: number;
  instructionSets: readonly string[];
  /** Intel Memory Protection Keys (WRPKRU). Absent before Skylake-SP. */
  pku: boolean;
  vtX: boolean;
  vtD: boolean;
  igpu: string | null;
}

/**
 * The reference machine. Every value here was used to make a decision in the
 * docs; treat it as the single source of truth for capacity planning.
 */
export const KABYLAKE_7700K: HardwareProfile = {
  id: "kabylake-7700k",
  cpu: "Intel Core i7-7700K",
  microarchitecture: "Kaby Lake",
  cores: 4,
  threads: 8,
  baseClockGhz: 4.2,
  turboClockGhz: 4.5,
  l3CacheMiB: 8,
  instructionSets: ["SSE4.2", "AVX2", "BMI2", "FMA3"],
  pku: false,
  vtX: true,
  vtD: true,
  igpu: "Intel HD Graphics 630",
};

// ---------------------------------------------------------------------------
// IPC message kinds
// ---------------------------------------------------------------------------

export const MESSAGE = {
  indexUpdate: 1,
  chatToken: 2,
  audioState: 3,
  frameReady: 4,
  agentHandoff: 5,
  control: 6,
  audit: 7,
} as const;

export type MessageKind = (typeof MESSAGE)[keyof typeof MESSAGE];

export const MESSAGE_NAMES: Readonly<Record<number, string>> = {
  1: "indexUpdate",
  2: "chatToken",
  3: "audioState",
  4: "frameReady",
  5: "agentHandoff",
  6: "control",
  7: "audit",
};

// ---------------------------------------------------------------------------
// Small utilities shared by every consumer
// ---------------------------------------------------------------------------

export function clamp(value: number, low: number, high: number): number {
  if (Number.isNaN(value)) return low;
  return Math.min(high, Math.max(low, value));
}

/** Type guard for values arriving from `postMessage`, JSON or the network. */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isDocumentKind(value: unknown): value is DocumentKind {
  return typeof value === "string" && (DOCUMENT_KINDS as readonly string[]).includes(value);
}

export function isCapabilityScope(value: unknown): value is CapabilityScope {
  return typeof value === "string" && (CAPABILITY_SCOPES as readonly string[]).includes(value);
}

export * from "./validate";
