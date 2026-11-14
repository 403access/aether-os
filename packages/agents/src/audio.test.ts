import { describe, expect, test } from "bun:test";
import { CapabilityTable } from "@aether/ipc";
import { MicrophoneGate } from "./audio";
import { AuditLog } from "./audit";

function setup(now = 1_000) {
  let clock = now;
  const capabilities = new CapabilityTable({ now: () => clock });
  const audit = new AuditLog({ now: () => clock });
  const gate = new MicrophoneGate(capabilities, { now: () => clock, audit });
  return {
    capabilities,
    audit,
    gate,
    advance(ms: number) {
      clock += ms;
    },
  };
}

describe("microphone capability gate", () => {
  test("denies capture without a capability and audits the denial", () => {
    const { gate, audit } = setup();
    const decision = gate.authorize("agent-a");
    expect(decision).toEqual({ ok: false, reason: "missing" });
    expect(gate.authorize("agent-a", "cap_nope")).toEqual({
      ok: false,
      reason: "unknown",
    });
    expect(audit.entries.map((entry) => entry.action)).toEqual([
      "audio.capture.denied",
      "audio.capture.denied",
    ]);
    expect(audit.verify().ok).toBe(true);
  });

  test("allows an explicit short-lived capability for the subject", () => {
    const { gate, audit, advance } = setup();
    const capability = gate.issue("agent-a", 1_000);
    expect(gate.authorize("agent-a", capability.id)).toEqual({
      ok: true,
      capabilityId: capability.id,
    });
    expect(gate.authorize("agent-b", capability.id)).toEqual({
      ok: false,
      reason: "subject",
    });
    advance(1_001);
    expect(gate.authorize("agent-a", capability.id)).toEqual({
      ok: false,
      reason: "expired",
    });
    expect(audit.entries.map((entry) => entry.action)).toEqual([
      "audio.capture.granted",
      "audio.capture.denied",
      "audio.capture.denied",
    ]);
    expect(audit.verify().ok).toBe(true);
  });

  test("revocation denies the next open", () => {
    const { gate, capabilities } = setup();
    const capability = gate.issue("agent-a");
    expect(gate.authorize("agent-a", capability.id).ok).toBe(true);
    capabilities.revoke(capability.id);
    expect(gate.authorize("agent-a", capability.id)).toEqual({
      ok: false,
      reason: "revoked",
    });
  });

  test("the legacy audio scope does not grant capture", () => {
    const { gate, capabilities } = setup();
    const legacy = capabilities.issue({
      subject: "agent-a",
      scope: "audio",
      resource: "*",
    });
    expect(gate.authorize("agent-a", legacy.id)).toEqual({
      ok: false,
      reason: "scope",
    });
  });
});
