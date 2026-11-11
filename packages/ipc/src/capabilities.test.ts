import { describe, expect, test } from "bun:test";
import { CapabilityTable, resourceMatches } from "../src/index";

describe("resourceMatches", () => {
  test("exact and wildcard patterns", () => {
    expect(resourceMatches("*", "fs:/anything")).toBe(true);
    expect(resourceMatches("fs:/work", "fs:/work")).toBe(true);
    expect(resourceMatches("fs:/work", "fs:/workshop")).toBe(false);
  });

  test("hierarchical directories match descendants, not siblings", () => {
    expect(resourceMatches("fs:/work/project", "fs:/work/project/src/main.rs")).toBe(true);
    expect(resourceMatches("fs:/work/project", "fs:/work/project2/src/main.rs")).toBe(false);
  });

  test("explicit trailing wildcard", () => {
    expect(resourceMatches("fs:/work/project/*", "fs:/work/project/src")).toBe(true);
    expect(resourceMatches("fs:/work/project/*", "fs:/work/project")).toBe(true);
    expect(resourceMatches("fs:/work/project/*", "fs:/work/projector")).toBe(false);
  });
});

describe("CapabilityTable issuance and verification", () => {
  test("issues and verifies a capability", () => {
    const table = new CapabilityTable({ now: () => 1_000 });
    const capability = table.issue({
      subject: "worker-a",
      scope: "fs.read",
      resource: "fs:/work/project",
    });
    expect(capability.generation).toBe(0);
    expect(table.verify(capability.id, {
      subject: "worker-a",
      scope: "fs.read",
      resource: "fs:/work/project/src/main.ts",
    })).toEqual({ ok: true });
  });

  test("rejects wrong subject, scope and resource", () => {
    const table = new CapabilityTable();
    const capability = table.issue({
      subject: "worker-a",
      scope: "fs.read",
      resource: "fs:/work/project",
    });
    expect(
      table.verify(capability.id, {
        subject: "worker-b",
        scope: "fs.read",
        resource: "fs:/work/project",
      }),
    ).toEqual({ ok: false, reason: "subject" });
    expect(
      table.verify(capability.id, {
        subject: "worker-a",
        scope: "fs.write",
        resource: "fs:/work/project",
      }),
    ).toEqual({ ok: false, reason: "scope" });
    expect(
      table.verify(capability.id, {
        subject: "worker-a",
        scope: "fs.read",
        resource: "fs:/etc/shadow",
      }),
    ).toEqual({ ok: false, reason: "resource" });
    expect(table.verify("missing", { scope: "fs.read", resource: "*" })).toEqual({
      ok: false,
      reason: "unknown",
    });
  });

  test("expiry is enforced against the injectable clock", () => {
    let now = 1_000;
    const table = new CapabilityTable({ now: () => now });
    const capability = table.issue({
      subject: "worker-a",
      scope: "infer",
      resource: "*",
      ttlMs: 5_000,
    });
    expect(table.verify(capability.id, { scope: "infer", resource: "*" }).ok).toBe(true);
    now = 6_000;
    expect(table.verify(capability.id, { scope: "infer", resource: "*" })).toEqual({
      ok: false,
      reason: "expired",
    });
  });

  test("revocation is immediate and idempotent", () => {
    const table = new CapabilityTable();
    const capability = table.issue({ subject: "a", scope: "net", resource: "*" });
    const request = { scope: "net" as const, resource: "*" };
    expect(table.verify(capability.id, request).ok).toBe(true);
    expect(table.revoke(capability.id)).toBe(1);
    expect(table.verify(capability.id, request)).toEqual({ ok: false, reason: "revoked" });
    expect(table.revoke(capability.id)).toBe(0);
  });
});

describe("delegation", () => {
  test("delegates a narrower resource and caps the lifetime", () => {
    let now = 10_000;
    const table = new CapabilityTable({ now: () => now });
    const parent = table.issue({
      subject: "coordinator",
      scope: "fs.read",
      resource: "fs:/work/project",
      ttlMs: 60_000,
      delegable: true,
    });

    const delegated = table.delegate(parent.id, "worker-a", {
      resource: "fs:/work/project/src",
      ttlMs: 999_999,
    });
    expect(delegated.ok).toBe(true);
    if (!delegated.ok) return;
    expect(delegated.capability.expiresAt).toBe(70_000);
    expect(delegated.capability.delegable).toBe(false);
    expect(delegated.capability.parentId).toBe(parent.id);
    expect(
      table.verify(delegated.capability.id, {
        subject: "worker-a",
        scope: "fs.read",
        resource: "fs:/work/project/src/lib.ts",
      }).ok,
    ).toBe(true);
  });

  test("refuses to widen scope or resource", () => {
    const table = new CapabilityTable();
    const parent = table.issue({
      subject: "coordinator",
      scope: "fs.read",
      resource: "fs:/work/project",
      delegable: true,
    });
    expect(table.delegate(parent.id, "worker", { scope: "fs.write" })).toEqual({
      ok: false,
      reason: "scope",
    });
    expect(table.delegate(parent.id, "worker", { resource: "fs:/etc" })).toEqual({
      ok: false,
      reason: "resource",
    });
  });

  test("non-delegable capabilities cannot be delegated", () => {
    const table = new CapabilityTable();
    const parent = table.issue({
      subject: "coordinator",
      scope: "audio",
      resource: "*",
      delegable: false,
    });
    expect(table.delegate(parent.id, "worker")).toEqual({
      ok: false,
      reason: "not-delegable",
    });
  });

  test("revoking a parent cascades to the whole tree", () => {
    const table = new CapabilityTable();
    const root = table.issue({
      subject: "coordinator",
      scope: "fs.read",
      resource: "fs:/work",
      delegable: true,
    });
    const child = table.delegate(root.id, "worker-a", { resource: "fs:/work/a" });
    expect(child.ok).toBe(true);
    if (!child.ok) return;
    // Delegated capabilities are not delegable by default, so build a chain
    // explicitly through the table API.
    const grandchild = table.issue({
      subject: "worker-b",
      scope: "fs.read",
      resource: "fs:/work/a/b",
      parentId: child.capability.id,
    });

    expect(table.revoke(root.id)).toBe(3);
    for (const id of [root.id, child.capability.id, grandchild.id]) {
      expect(table.verify(id, { scope: "fs.read", resource: "fs:/work" })).toEqual({
        ok: false,
        reason: "revoked",
      });
    }
  });

  test("revokeSubject removes every capability for one agent", () => {
    const table = new CapabilityTable();
    table.issue({ subject: "a", scope: "net", resource: "*" });
    table.issue({ subject: "a", scope: "infer", resource: "*" });
    table.issue({ subject: "b", scope: "net", resource: "*" });
    expect(table.revokeSubject("a")).toBe(2);
    expect(table.list("a")).toEqual([]);
    expect(table.list("b").length).toBe(1);
  });
});
