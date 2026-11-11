import { fnv1a64 } from "@aether/ipc";

/**
 * Tamper-evident audit log.
 *
 * Each entry commits to the previous entry's hash, so removing, reordering or
 * editing a record invalidates every subsequent entry. The hash here is a
 * 64-bit FNV-1a chain so the same code runs in the browser demo; the shipping
 * daemon uses SHA-256 in the same layout.
 */

export interface AuditEventInput {
  actor: string;
  action: string;
  subject: string;
  detail?: string;
}

export interface AuditEntry {
  seq: number;
  at: number;
  actor: string;
  action: string;
  subject: string;
  detail: string;
  prevHash: string;
  hash: string;
}

export type VerifyResult = { ok: true } | { ok: false; index: number; reason: string };

export interface AuditLogOptions {
  now?: () => number;
  hash?: (input: string) => string;
}

const GENESIS = "0".repeat(16);

function defaultHash(input: string): string {
  return fnv1a64(input).toString(16).padStart(16, "0");
}

export class AuditLog {
  readonly #entries: AuditEntry[] = [];
  readonly #now: () => number;
  readonly #hash: (input: string) => string;

  constructor(options: AuditLogOptions = {}) {
    this.#now = options.now ?? (() => Date.now());
    this.#hash = options.hash ?? defaultHash;
  }

  append(input: AuditEventInput): AuditEntry {
    const previous = this.#entries[this.#entries.length - 1];
    const prevHash = previous?.hash ?? GENESIS;
    const seq = this.#entries.length;
    const at = this.#now();
    const detail = input.detail ?? "";
    const hash = this.#hash(
      canonical(seq, at, input.actor, input.action, input.subject, detail, prevHash),
    );
    const entry: AuditEntry = {
      seq,
      at,
      actor: input.actor,
      action: input.action,
      subject: input.subject,
      detail,
      prevHash,
      hash,
    };
    this.#entries.push(entry);
    return { ...entry };
  }

  get entries(): readonly AuditEntry[] {
    return this.#entries;
  }

  get length(): number {
    return this.#entries.length;
  }

  verify(): VerifyResult {
    let prevHash = GENESIS;
    for (let index = 0; index < this.#entries.length; index += 1) {
      const entry = this.#entries[index];
      if (entry === undefined) continue;
      if (entry.seq !== index) {
        return { ok: false, index, reason: `sequence mismatch: expected ${index}, got ${entry.seq}` };
      }
      if (entry.prevHash !== prevHash) {
        return { ok: false, index, reason: "previous hash mismatch" };
      }
      const expected = this.#hash(
        canonical(entry.seq, entry.at, entry.actor, entry.action, entry.subject, entry.detail, prevHash),
      );
      if (entry.hash !== expected) {
        return { ok: false, index, reason: "entry hash mismatch" };
      }
      prevHash = entry.hash;
    }
    return { ok: true };
  }

  toJSON(): AuditEntry[] {
    return this.#entries.map((entry) => ({ ...entry }));
  }

  static fromJSON(entries: readonly AuditEntry[], options: AuditLogOptions = {}): AuditLog {
    const log = new AuditLog(options);
    for (const entry of entries) {
      log.#entries.push({ ...entry });
    }
    return log;
  }
}

function canonical(
  seq: number,
  at: number,
  actor: string,
  action: string,
  subject: string,
  detail: string,
  prevHash: string,
): string {
  return JSON.stringify([seq, at, actor, action, subject, detail, prevHash]);
}
