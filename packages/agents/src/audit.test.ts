import { describe, expect, test } from "bun:test";
import { AuditLog } from "../src/audit";

describe("AuditLog", () => {
  test("chains entries from genesis", () => {
    let now = 1_000;
    const log = new AuditLog({ now: () => (now += 1) });
    const first = log.append({ actor: "aetherd", action: "agent.spawn", subject: "worker-a" });
    const second = log.append({ actor: "aetherd", action: "agent.stop", subject: "worker-a" });

    expect(first.seq).toBe(0);
    expect(first.prevHash).toBe("0000000000000000");
    expect(second.prevHash).toBe(first.hash);
    expect(first.at).toBe(1_001);
    expect(second.at).toBe(1_002);
    expect(log.verify()).toEqual({ ok: true });
  });

  test("detects an edited entry", () => {
    const log = new AuditLog();
    log.append({ actor: "a", action: "channel.open", subject: "a->b" });
    log.append({ actor: "a", action: "channel.close", subject: "a->b" });
    log.entries[0]!.detail = "forged";
    const result = log.verify();
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.index).toBe(0);
    expect(result.reason).toContain("hash");
  });

  test("detects a broken chain after removing an entry", () => {
    const log = new AuditLog();
    log.append({ actor: "a", action: "one", subject: "x" });
    log.append({ actor: "a", action: "two", subject: "x" });
    log.append({ actor: "a", action: "three", subject: "x" });
    const tampered = AuditLog.fromJSON(log.toJSON().filter((_, index) => index !== 1));
    const result = tampered.verify();
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.index).toBe(1);
  });

  test("round-trips through JSON", () => {
    const log = new AuditLog({ hash: (input) => `h${input.length}` });
    log.append({ actor: "a", action: "x", subject: "y", detail: "z" });
    const restored = AuditLog.fromJSON(JSON.parse(JSON.stringify(log.toJSON())), {
      hash: (input) => `h${input.length}`,
    });
    expect(restored.verify()).toEqual({ ok: true });
    expect(restored.entries).toEqual(log.entries);
  });

  test("empty log verifies", () => {
    expect(new AuditLog().verify()).toEqual({ ok: true });
  });
});
