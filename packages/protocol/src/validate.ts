import {
  isCapabilityScope,
  isDocumentKind,
  isRecord,
  type AgentSpec,
  type Capability,
  type CapabilityRequest,
  type IndexDocument,
  type IsolationTier,
} from "./index";

/**
 * Boundary validation. Every value that enters the system from JSON, a worker
 * boundary, a file, or the network goes through one of these parsers. They are
 * deliberately dependency-free and return structured issues instead of throwing,
 * so callers can decide between fail-closed and best-effort ingestion.
 */

export interface Issue {
  path: string;
  message: string;
}

export type Parsed<T> = { ok: true; value: T } | { ok: false; issues: Issue[] };

function ok<T>(value: T): Parsed<T> {
  return { ok: true, value };
}

function fail(path: string, message: string): Parsed<never> {
  return { ok: false, issues: [{ path, message }] };
}

export function formatIssues(issues: readonly Issue[]): string {
  return issues.map((issue) => `${issue.path}: ${issue.message}`).join("; ");
}

/** Parse or throw. Use only where an invalid value is a programmer error. */
export function mustParse<T>(parsed: Parsed<T>, label: string): T {
  if (!parsed.ok) {
    throw new Error(`${label} is invalid: ${formatIssues(parsed.issues)}`);
  }
  return parsed.value;
}

function readString(value: unknown, path: string, max: number, issues: Issue[]): string | undefined {
  if (typeof value !== "string") {
    issues.push({ path, message: "expected a string" });
    return undefined;
  }
  if (value.length === 0) {
    issues.push({ path, message: "must not be empty" });
    return undefined;
  }
  if (value.length > max) {
    issues.push({ path, message: `must be at most ${max} characters` });
    return undefined;
  }
  return value;
}

function readOptionalString(
  value: unknown,
  path: string,
  max: number,
  issues: Issue[],
): string | undefined {
  if (value === undefined || value === null) return undefined;
  return readString(value, path, max, issues);
}

function readFiniteNumber(
  value: unknown,
  path: string,
  issues: Issue[],
  options: { min?: number; max?: number; integer?: boolean } = {},
): number | undefined {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    issues.push({ path, message: "expected a finite number" });
    return undefined;
  }
  if (options.integer && !Number.isInteger(value)) {
    issues.push({ path, message: "expected an integer" });
    return undefined;
  }
  if (options.min !== undefined && value < options.min) {
    issues.push({ path, message: `must be >= ${options.min}` });
    return undefined;
  }
  if (options.max !== undefined && value > options.max) {
    issues.push({ path, message: `must be <= ${options.max}` });
    return undefined;
  }
  return value;
}

const MAX_TAGS = 64;
const MAX_TAG_LENGTH = 64;
const MAX_BODY_LENGTH = 1_000_000;
const MAX_TITLE_LENGTH = 512;
const MAX_SUBTITLE_LENGTH = 2_048;
const MAX_PATH_LENGTH = 4_096;
const MAX_ID_LENGTH = 1_024;

export function parseIndexDocument(input: unknown): Parsed<IndexDocument> {
  if (!isRecord(input)) return fail("$", "expected an object");
  const issues: Issue[] = [];

  const id = readString(input["id"], "$.id", MAX_ID_LENGTH, issues);
  const kind = input["kind"];
  if (!isDocumentKind(kind)) {
    issues.push({ path: "$.kind", message: "unknown document kind" });
  }
  const title = readString(input["title"], "$.title", MAX_TITLE_LENGTH, issues);
  const subtitle = readOptionalString(input["subtitle"], "$.subtitle", MAX_SUBTITLE_LENGTH, issues);
  const body = readOptionalString(input["body"], "$.body", MAX_BODY_LENGTH, issues);
  const path = readOptionalString(input["path"], "$.path", MAX_PATH_LENGTH, issues);
  const modifiedAt =
    input["modifiedAt"] === undefined
      ? undefined
      : readFiniteNumber(input["modifiedAt"], "$.modifiedAt", issues, { min: 0 });
  const uses =
    input["uses"] === undefined
      ? undefined
      : readFiniteNumber(input["uses"], "$.uses", issues, { min: 0, integer: true });

  let tags: string[] | undefined;
  if (input["tags"] !== undefined) {
    if (!Array.isArray(input["tags"])) {
      issues.push({ path: "$.tags", message: "expected an array of strings" });
    } else if (input["tags"].length > MAX_TAGS) {
      issues.push({ path: "$.tags", message: `must have at most ${MAX_TAGS} entries` });
    } else {
      tags = [];
      for (let index = 0; index < input["tags"].length; index += 1) {
        const tag = readString(input["tags"][index], `$.tags[${index}]`, MAX_TAG_LENGTH, issues);
        if (tag !== undefined) tags.push(tag);
      }
    }
  }

  if (issues.length > 0) return { ok: false, issues };
  if (id === undefined || title === undefined || !isDocumentKind(kind)) {
    return fail("$", "unreachable: validated fields missing");
  }

  const document: IndexDocument = { id, kind, title };
  if (subtitle !== undefined) document.subtitle = subtitle;
  if (body !== undefined) document.body = body;
  if (path !== undefined) document.path = path;
  if (tags !== undefined) document.tags = tags;
  if (modifiedAt !== undefined) document.modifiedAt = modifiedAt;
  if (uses !== undefined) document.uses = uses;
  return ok(document);
}

const AGENT_ID = /^[a-z0-9][a-z0-9._-]{0,63}$/i;

export function parseCapabilityRequest(input: unknown, path: string): Parsed<CapabilityRequest> {
  if (!isRecord(input)) return fail(path, "expected an object");
  const issues: Issue[] = [];
  const scope = input["scope"];
  if (!isCapabilityScope(scope)) {
    issues.push({ path: `${path}.scope`, message: "unknown capability scope" });
  }
  const resource = readString(input["resource"], `${path}.resource`, MAX_PATH_LENGTH, issues);
  const ttlMs =
    input["ttlMs"] === undefined
      ? undefined
      : readFiniteNumber(input["ttlMs"], `${path}.ttlMs`, issues, { min: 1, max: 86_400_000 });

  if (issues.length > 0) return { ok: false, issues };
  if (!isCapabilityScope(scope) || resource === undefined) {
    return fail(path, "unreachable: validated fields missing");
  }
  const request: CapabilityRequest = { scope, resource };
  if (ttlMs !== undefined) request.ttlMs = ttlMs;
  return ok(request);
}

const TIERS: readonly IsolationTier[] = ["in-process", "process-landlock", "microvm"];
const TRUST_LEVELS: readonly AgentSpec["trust"][] = ["trusted", "untrusted", "hostile"];

export function parseAgentSpec(input: unknown): Parsed<AgentSpec> {
  if (!isRecord(input)) return fail("$", "expected an object");
  const issues: Issue[] = [];

  const id = readString(input["id"], "$.id", 64, issues);
  if (id !== undefined && !AGENT_ID.test(id)) {
    issues.push({ path: "$.id", message: "must match [a-z0-9][a-z0-9._-]{0,63}" });
  }
  const name = readString(input["name"], "$.name", 128, issues);

  const trust = input["trust"];
  if (!TRUST_LEVELS.includes(trust as AgentSpec["trust"])) {
    issues.push({ path: "$.trust", message: "expected trusted | untrusted | hostile" });
  }
  const tier = input["tier"];
  if (!TIERS.includes(tier as IsolationTier)) {
    issues.push({ path: "$.tier", message: "expected in-process | process-landlock | microvm" });
  }

  const cpuWeight = readFiniteNumber(input["cpuWeight"], "$.cpuWeight", issues, {
    min: 1,
    max: 100,
    integer: true,
  });
  const memoryMiB = readFiniteNumber(input["memoryMiB"], "$.memoryMiB", issues, {
    min: 16,
    max: 65_536,
    integer: true,
  });
  const timeoutMs =
    input["timeoutMs"] === undefined
      ? undefined
      : readFiniteNumber(input["timeoutMs"], "$.timeoutMs", issues, { min: 1, max: 86_400_000 });
  const image = readOptionalString(input["image"], "$.image", 256, issues);

  const capabilities: CapabilityRequest[] = [];
  if (!Array.isArray(input["capabilities"])) {
    issues.push({ path: "$.capabilities", message: "expected an array" });
  } else if (input["capabilities"].length > 32) {
    issues.push({ path: "$.capabilities", message: "at most 32 capability requests" });
  } else {
    for (let index = 0; index < input["capabilities"].length; index += 1) {
      const parsed = parseCapabilityRequest(
        input["capabilities"][index],
        `$.capabilities[${index}]`,
      );
      if (parsed.ok) capabilities.push(parsed.value);
      else issues.push(...parsed.issues);
    }
  }

  if (issues.length > 0) return { ok: false, issues };
  if (
    id === undefined ||
    name === undefined ||
    !TRUST_LEVELS.includes(trust as AgentSpec["trust"]) ||
    !TIERS.includes(tier as IsolationTier) ||
    cpuWeight === undefined ||
    memoryMiB === undefined
  ) {
    return fail("$", "unreachable: validated fields missing");
  }

  const spec: AgentSpec = {
    id,
    name,
    trust: trust as AgentSpec["trust"],
    tier: tier as IsolationTier,
    cpuWeight,
    memoryMiB,
    capabilities,
  };
  if (image !== undefined) spec.image = image;
  if (timeoutMs !== undefined) spec.timeoutMs = timeoutMs;
  return ok(spec);
}

export function parseCapability(input: unknown): Parsed<Capability> {
  if (!isRecord(input)) return fail("$", "expected an object");
  const issues: Issue[] = [];

  const id = readString(input["id"], "$.id", 128, issues);
  const subject = readString(input["subject"], "$.subject", 128, issues);
  const scope = input["scope"];
  if (!isCapabilityScope(scope)) {
    issues.push({ path: "$.scope", message: "unknown capability scope" });
  }
  const resource = readString(input["resource"], "$.resource", MAX_PATH_LENGTH, issues);
  const issuedAt = readFiniteNumber(input["issuedAt"], "$.issuedAt", issues, { min: 0 });
  const expiresAt =
    input["expiresAt"] === null || input["expiresAt"] === undefined
      ? null
      : readFiniteNumber(input["expiresAt"], "$.expiresAt", issues, { min: 0 });
  if (typeof input["delegable"] !== "boolean") {
    issues.push({ path: "$.delegable", message: "expected a boolean" });
  }
  const generation = readFiniteNumber(input["generation"], "$.generation", issues, {
    min: 0,
    integer: true,
  });
  const parentId =
    input["parentId"] === null || input["parentId"] === undefined
      ? null
      : readString(input["parentId"], "$.parentId", 128, issues);

  if (issues.length > 0) return { ok: false, issues };
  if (
    id === undefined ||
    subject === undefined ||
    !isCapabilityScope(scope) ||
    resource === undefined ||
    issuedAt === undefined ||
    generation === undefined ||
    typeof input["delegable"] !== "boolean"
  ) {
    return fail("$", "unreachable: validated fields missing");
  }

  const capability: Capability = {
    id,
    subject,
    scope,
    resource,
    issuedAt,
    expiresAt,
    delegable: input["delegable"],
    generation,
    parentId,
  };
  return ok(capability);
}
