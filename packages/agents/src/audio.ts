import type { Capability } from "@aether/protocol";
import {
  AUDIO_CAPTURE_RESOURCE,
  AUDIO_CAPTURE_SCOPE,
} from "@aether/protocol";
import type { CapabilityTable } from "@aether/ipc";
import { AuditLog } from "./audit";

/**
 * Microphone capability gate.
 *
 * A capture stream is not a default agent right. The supervisor asks this gate
 * before opening the source; the gate verifies a short-lived `audio.capture`
 * capability for `audio:input`, audits both grants and denials, and never
 * infers permission from the broader legacy `audio` scope.
 */

export type MicrophoneFailure =
  | "missing"
  | "unknown"
  | "revoked"
  | "expired"
  | "scope"
  | "resource"
  | "subject"
  | "generation";

export type MicrophoneDecision =
  | { ok: true; capabilityId: string }
  | { ok: false; reason: MicrophoneFailure };

export interface MicrophoneGateOptions {
  now?: () => number;
  audit?: AuditLog;
}

export class MicrophoneGate {
  readonly #capabilities: CapabilityTable;
  readonly #audit: AuditLog;
  readonly #now: () => number;

  constructor(capabilities: CapabilityTable, options: MicrophoneGateOptions = {}) {
    this.#capabilities = capabilities;
    this.#now = options.now ?? (() => Date.now());
    this.#audit = options.audit ?? new AuditLog({ now: this.#now });
  }

  get audit(): AuditLog {
    return this.#audit;
  }

  /** Issue a capture capability; callers should make this short-lived. */
  issue(agentId: string, ttlMs = 60_000): Capability {
    return this.#capabilities.issue({
      subject: agentId,
      scope: AUDIO_CAPTURE_SCOPE,
      resource: AUDIO_CAPTURE_RESOURCE,
      ttlMs,
      delegable: false,
    });
  }

  /** Verify a presented capability before opening the microphone. */
  authorize(agentId: string, capabilityId?: string): MicrophoneDecision {
    if (capabilityId === undefined || capabilityId.length === 0) {
      this.#deny(agentId, "missing", "missing audio.capture capability");
      return { ok: false, reason: "missing" };
    }
    const verification = this.#capabilities.verify(capabilityId, {
      subject: agentId,
      scope: AUDIO_CAPTURE_SCOPE,
      resource: AUDIO_CAPTURE_RESOURCE,
    });
    if (!verification.ok) {
      const reason = verification.reason ?? "unknown";
      this.#deny(agentId, reason, `audio.capture denied: ${reason}`);
      return { ok: false, reason };
    }
    this.#audit.append({
      actor: agentId,
      action: "audio.capture.granted",
      subject: AUDIO_CAPTURE_RESOURCE,
      detail: capabilityId,
    });
    return { ok: true, capabilityId };
  }

  #deny(agentId: string, reason: MicrophoneFailure, detail: string): void {
    this.#audit.append({
      actor: agentId,
      action: "audio.capture.denied",
      subject: AUDIO_CAPTURE_RESOURCE,
      detail: `${reason}: ${detail}`,
    });
  }
}
