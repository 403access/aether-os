import {
  type Capability,
  type CapabilityScope,
  type VerificationResult,
  type VerifyFailure,
} from "@aether/protocol";

/**
 * Object-capability table.
 *
 * Design notes:
 *  - Capabilities are POSIX-style handles, not ambient permissions. An agent
 *    can only use a capability it was explicitly handed.
 *  - Delegation can narrow scope and resource, never widen them.
 *  - Revocation cascades to descendants, so revoking a coordinator instantly
 *    disables every capability it minted.
 *  - Verification is cheap and allocation-free: the hot path is a map lookup
 *    plus two integer comparisons.
 */

export interface IssueInput {
  subject: string;
  scope: CapabilityScope;
  resource: string;
  /** Lifetime in ms. Omit for no expiry. */
  ttlMs?: number;
  delegable?: boolean;
  parentId?: string | null;
}

export interface VerifyRequest {
  scope: CapabilityScope;
  resource: string;
  subject?: string;
}

export type DelegateResult =
  | { ok: true; capability: Capability }
  | { ok: false; reason: "unknown" | "revoked" | "not-delegable" | "scope" | "resource" };

export interface CapabilityTableOptions {
  /** Injectable clock (epoch ms) for deterministic expiry tests. */
  now?: () => number;
  idFactory?: (sequence: number) => string;
}

/**
 * Resource patterns:
 *  - `*` matches everything (only issuable by the host broker)
 *  - `fs:/work/project/*` matches the directory and anything below it
 *  - `fs:/work/project` matches itself and, hierarchically, descendants
 *    (`fs:/work/project/src/main.rs`) but never siblings (`fs:/work/project2`)
 */
export function resourceMatches(pattern: string, resource: string): boolean {
  if (pattern === "*") return true;
  if (pattern === resource) return true;
  if (pattern.endsWith("/*")) {
    const base = pattern.slice(0, -2);
    return resource === base || resource.startsWith(`${base}/`);
  }
  return resource.startsWith(`${pattern}/`);
}

export class CapabilityTable {
  readonly #capabilities = new Map<string, Capability>();
  readonly #revoked = new Set<string>();
  readonly #children = new Map<string, Set<string>>();
  readonly #now: () => number;
  readonly #idFactory: (sequence: number) => string;
  #sequence = 0;

  constructor(options: CapabilityTableOptions = {}) {
    this.#now = options.now ?? (() => Date.now());
    this.#idFactory = options.idFactory ?? ((sequence) => `cap_${sequence.toString(36)}`);
  }

  issue(input: IssueInput): Capability {
    this.#sequence += 1;
    const issuedAt = this.#now();
    const capability: Capability = {
      id: this.#idFactory(this.#sequence),
      subject: input.subject,
      scope: input.scope,
      resource: input.resource,
      issuedAt,
      expiresAt: input.ttlMs === undefined ? null : issuedAt + input.ttlMs,
      delegable: input.delegable ?? false,
      generation: 0,
      parentId: input.parentId ?? null,
    };

    this.#capabilities.set(capability.id, capability);
    if (capability.parentId !== null) {
      let children = this.#children.get(capability.parentId);
      if (children === undefined) {
        children = new Set<string>();
        this.#children.set(capability.parentId, children);
      }
      children.add(capability.id);
    }
    return capability;
  }

  verify(capabilityId: string, request: VerifyRequest): VerificationResult {
    const capability = this.#capabilities.get(capabilityId);
    if (capability === undefined) return { ok: false, reason: "unknown" };
    return this.#check(capability, request, this.#now());
  }

  /** Verify against an explicit clock; used by the broker on capture. */
  verifyAt(capabilityId: string, request: VerifyRequest, now: number): VerificationResult {
    const capability = this.#capabilities.get(capabilityId);
    if (capability === undefined) return { ok: false, reason: "unknown" };
    return this.#check(capability, request, now);
  }

  delegate(
    parentId: string,
    toSubject: string,
    narrow?: { scope?: CapabilityScope; resource?: string; ttlMs?: number },
  ): DelegateResult {
    const parent = this.#capabilities.get(parentId);
    if (parent === undefined) return { ok: false, reason: "unknown" };
    if (this.#revoked.has(parent.id) || parent.generation > 0) {
      return { ok: false, reason: "revoked" };
    }
    if (!parent.delegable) return { ok: false, reason: "not-delegable" };

    const scope = narrow?.scope ?? parent.scope;
    if (scope !== parent.scope) return { ok: false, reason: "scope" };

    const resource = narrow?.resource ?? parent.resource;
    if (!resourceMatches(parent.resource, resource)) {
      return { ok: false, reason: "resource" };
    }

    const now = this.#now();
    const requestedExpiry =
      narrow?.ttlMs === undefined ? parent.expiresAt : now + Math.max(1, narrow.ttlMs);
    const expiresAt =
      parent.expiresAt === null
        ? requestedExpiry
        : requestedExpiry === null
          ? parent.expiresAt
          : Math.min(parent.expiresAt, requestedExpiry);

    this.#sequence += 1;
    const capability: Capability = {
      id: this.#idFactory(this.#sequence),
      subject: toSubject,
      scope,
      resource,
      issuedAt: now,
      expiresAt,
      delegable: false,
      generation: 0,
      parentId: parent.id,
    };
    this.#capabilities.set(capability.id, capability);
    let children = this.#children.get(parent.id);
    if (children === undefined) {
      children = new Set<string>();
      this.#children.set(parent.id, children);
    }
    children.add(capability.id);
    return { ok: true, capability };
  }

  /** Revoke one capability and everything derived from it. Returns the count. */
  revoke(capabilityId: string): number {
    if (!this.#capabilities.has(capabilityId)) return 0;
    return this.#revokeRecursive(capabilityId);
  }

  revokeSubject(subject: string): number {
    let revoked = 0;
    for (const capability of this.#capabilities.values()) {
      if (capability.subject === subject && !this.#revoked.has(capability.id)) {
        revoked += this.#revokeRecursive(capability.id);
      }
    }
    return revoked;
  }

  isRevoked(capabilityId: string): boolean {
    return this.#revoked.has(capabilityId);
  }

  get(capabilityId: string): Capability | undefined {
    return this.#capabilities.get(capabilityId);
  }

  list(subject?: string): Capability[] {
    const result: Capability[] = [];
    for (const capability of this.#capabilities.values()) {
      if (this.#revoked.has(capability.id)) continue;
      if (subject !== undefined && capability.subject !== subject) continue;
      result.push({ ...capability });
    }
    return result;
  }

  get size(): number {
    return this.#capabilities.size;
  }

  #check(capability: Capability, request: VerifyRequest, now: number): VerificationResult {
    if (this.#revoked.has(capability.id) || capability.generation > 0) {
      return { ok: false, reason: "revoked" satisfies VerifyFailure };
    }
    if (capability.expiresAt !== null && now >= capability.expiresAt) {
      return { ok: false, reason: "expired" };
    }
    if (request.subject !== undefined && request.subject !== capability.subject) {
      return { ok: false, reason: "subject" };
    }
    if (request.scope !== capability.scope) {
      return { ok: false, reason: "scope" };
    }
    if (!resourceMatches(capability.resource, request.resource)) {
      return { ok: false, reason: "resource" };
    }
    return { ok: true };
  }

  #revokeRecursive(capabilityId: string): number {
    if (this.#revoked.has(capabilityId)) return 0;
    const capability = this.#capabilities.get(capabilityId);
    if (capability === undefined) return 0;

    capability.generation += 1;
    this.#revoked.add(capabilityId);
    let count = 1;

    const children = this.#children.get(capabilityId);
    if (children !== undefined) {
      for (const child of [...children]) {
        count += this.#revokeRecursive(child);
      }
    }
    return count;
  }
}
