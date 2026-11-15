import { afterAll, describe, expect, test } from "bun:test";
import { mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  AETHER_RING_MAGIC,
  PROTOCOL_VERSION,
  RING_HEADER_BYTES,
  type AgentSpec,
  type IsolationTier,
} from "../packages/protocol/src/index";
import { CapabilityTable, RingBuffer, resourceMatches } from "../packages/ipc/src/index";
import {
  AdmissionController,
  AgentScheduler,
  planIsolation,
} from "../packages/agents/src/index";
import {
  DEFAULT_EVENTS_PER_SECOND,
  InputBatcher,
  MAX_MOTION_PER_EVENT,
} from "../packages/remote/src/index";
import { scanDirectory } from "../packages/indexer/src/fs";
import { buildIndex, handleRequest } from "../apps/indexer-service/src/service";

/**
 * Security regression tests for the deny path.
 *
 * The threat model requires every boundary to fail closed: a malformed or
 * adversarial value must not widen a grant, corrupt accounting, disable
 * isolation, or turn into an out-of-bounds read. These tests deliberately
 * construct inputs that bypass the protocol parsers.
 */

function agentSpec(overrides: Partial<AgentSpec> = {}): AgentSpec {
  return {
    id: "worker-a",
    name: "Worker A",
    trust: "untrusted",
    tier: "process-landlock",
    cpuWeight: 25,
    memoryMiB: 512,
    capabilities: [],
    timeoutMs: 60_000,
    ...overrides,
  };
}

describe("capability boundary security", () => {
  test("path traversal cannot escape a filesystem grant", () => {
    expect(resourceMatches("fs:/work/project", "fs:/work/project/../etc/shadow")).toBe(false);
    expect(resourceMatches("fs:/work/project", "fs:/work/project/./secrets")).toBe(false);
    expect(resourceMatches("fs:/work/project/*", "fs:/work/project/../.ssh/id_rsa")).toBe(false);
  });

  test("NUL bytes never match, including the global wildcard", () => {
    expect(resourceMatches("fs:/work", "fs:/work/\0/secret")).toBe(false);
    expect(resourceMatches("*", "fs:/work/\0/secret")).toBe(false);
  });

  test("ordinary dot-prefixed names are not mistaken for traversal", () => {
    expect(resourceMatches("fs:/work", "fs:/work/.config/aether")).toBe(true);
    expect(resourceMatches("fs:/work", "fs:/work/..data/file")).toBe(true);
  });

  test("verification rejects traversal and NUL requests", () => {
    const table = new CapabilityTable({ now: () => 1_000 });
    const capability = table.issue({
      subject: "worker-a",
      scope: "fs.read",
      resource: "fs:/work/project",
    });

    expect(
      table.verify(capability.id, {
        subject: "worker-a",
        scope: "fs.read",
        resource: "fs:/work/project/../etc/shadow",
      }),
    ).toEqual({ ok: false, reason: "resource" });
    expect(
      table.verify(capability.id, {
        subject: "worker-a",
        scope: "fs.read",
        resource: "fs:/work/project/\0/secret",
      }),
    ).toEqual({ ok: false, reason: "resource" });
  });

  test("delegation cannot launder a traversal path into a narrower grant", () => {
    const table = new CapabilityTable();
    const parent = table.issue({
      subject: "coordinator",
      scope: "fs.read",
      resource: "fs:/work/project",
      delegable: true,
    });

    expect(
      table.delegate(parent.id, "worker-a", { resource: "fs:/work/project/../etc" }),
    ).toEqual({ ok: false, reason: "resource" });
  });
});

describe("admission accounting security", () => {
  const budget = { cores: 4, memoryMiB: 2_048, reserveCores: 1, maxAgents: 6 };

  test.each([
    ["negative CPU", { cpuWeight: -100 }],
    ["zero CPU", { cpuWeight: 0 }],
    ["NaN CPU", { cpuWeight: Number.NaN }],
    ["infinite CPU", { cpuWeight: Number.POSITIVE_INFINITY }],
    ["negative memory", { memoryMiB: -1 }],
    ["NaN memory", { memoryMiB: Number.NaN }],
    ["infinite memory", { memoryMiB: Number.POSITIVE_INFINITY }],
  ])("rejects %s without changing usage", (_label, overrides) => {
    const controller = new AdmissionController(budget);
    expect(controller.tryAdmit(agentSpec(overrides))).toEqual({
      ok: false,
      reason: "invalid",
    });
    expect(controller.usage()).toMatchObject({ agents: 0, cpuPoints: 0, memoryMiB: 0 });
  });

  test("a malformed request cannot create headroom for a later agent", () => {
    const controller = new AdmissionController(budget);
    expect(controller.tryAdmit(agentSpec({ id: "valid", cpuWeight: 100 })).ok).toBe(true);
    expect(
      controller.tryAdmit(agentSpec({ id: "drain", cpuWeight: -100, memoryMiB: -1_000 })),
    ).toEqual({ ok: false, reason: "invalid" });
    expect(controller.usage()).toMatchObject({ agents: 1, cpuPoints: 100, memoryMiB: 512 });
  });

  test("the scheduler rejects invalid specs instead of queueing them forever", () => {
    const scheduler = new AgentScheduler(budget);
    expect(scheduler.submit(agentSpec({ cpuWeight: -1 }))).toEqual({
      status: "rejected",
      reason: "invalid",
    });
    expect(scheduler.queued()).toEqual([]);
  });
});

describe("isolation planner fail-closed", () => {
  test("unknown trust and tier values are blocked, not treated as in-process", () => {
    const plan = planIsolation(
      agentSpec({
        trust: "root" as AgentSpec["trust"],
        tier: "none" as IsolationTier,
      }),
    );

    expect(plan.blocked).toBe(true);
    expect(plan.enforcement).not.toBe("none");
    expect(plan.blockers.join(" ")).toContain("Unknown trust level");
    expect(plan.blockers.join(" ")).toContain("Unknown isolation tier");
  });

  test("an unknown tier alone is blocked even for a trusted agent", () => {
    const plan = planIsolation(
      agentSpec({
        trust: "trusted",
        tier: "in-process-and-then-some" as IsolationTier,
      }),
    );

    expect(plan.blocked).toBe(true);
    expect(plan.blockers.join(" ")).toContain("Unknown isolation tier");
  });
});

describe("remote input validation", () => {
  test("non-finite motion, wheel and button values are rejected", () => {
    const batcher = new InputBatcher();
    batcher.setFocused(true, 0);

    expect(batcher.pushMotion(Number.NaN, 1, 1)).toBe(false);
    expect(batcher.pushMotion(1, Number.POSITIVE_INFINITY, 2)).toBe(false);
    expect(batcher.pushWheel(Number.NaN, 1, 3)).toBe(false);
    expect(batcher.pushWheel(1, Number.NEGATIVE_INFINITY, 4)).toBe(false);
    expect(batcher.pushButton(1.5, true, 5)).toBe(false);
    expect(batcher.pushButton(8, true, 6)).toBe(false);

    const batch = batcher.drain();
    expect(batch.rejected).toBe(6);
    expect(batch.events).toEqual([{ kind: "focus", focused: true }]);
  });

  test("wheel deltas are bounded before they reach the remote seat", () => {
    const batcher = new InputBatcher();
    batcher.setFocused(true, 0);
    expect(batcher.pushWheel(1_000_000, -1_000_000, 1)).toBe(true);

    const batch = batcher.drain();
    expect(batch.events).toContainEqual({
      kind: "wheel",
      dx: MAX_MOTION_PER_EVENT,
      dy: -MAX_MOTION_PER_EVENT,
    });
  });

  test("an invalid rate limit falls back to the safe default", () => {
    expect(new InputBatcher(Number.NaN).maxPerSecond).toBe(DEFAULT_EVENTS_PER_SECOND);
    expect(new InputBatcher(Number.POSITIVE_INFINITY).maxPerSecond).toBe(
      DEFAULT_EVENTS_PER_SECOND,
    );
  });
});

describe("shared ring corruption", () => {
  function rawRing(capacity: number, head = 0n, tail = 0n): SharedArrayBuffer {
    const buffer = new SharedArrayBuffer(RING_HEADER_BYTES + 2_048);
    const view = new DataView(buffer);
    view.setUint32(0, AETHER_RING_MAGIC, true);
    view.setUint32(4, PROTOCOL_VERSION, true);
    view.setUint32(8, capacity, true);
    view.setBigInt64(16, head, true);
    view.setBigInt64(24, tail, true);
    return buffer;
  }

  test("attach refuses a capacity that is not 8-byte aligned", () => {
    expect(() => RingBuffer.attach(rawRing(513))).toThrow(/capacity/);
  });

  test("attach refuses impossible head/tail counters", () => {
    expect(() => RingBuffer.attach(rawRing(512, 10n, 20n))).toThrow(/counters/);
    expect(() => RingBuffer.attach(rawRing(512, 10_000n, 0n))).toThrow(/counters/);
  });

  test("the consumer rejects a forged record length instead of reading out of bounds", () => {
    const ring = RingBuffer.create(512, { shared: false });
    ring.tryPush(1, 0n, new Uint8Array([1, 2, 3]));

    const view = new DataView(ring.buffer);
    view.setUint32(RING_HEADER_BYTES, 512, true);
    expect(() => ring.tryPop()).toThrow(/corrupt ring/);
  });

  test("the consumer rejects a record larger than the published bytes", () => {
    const ring = RingBuffer.create(1_024, { shared: false });
    ring.tryPush(1, 0n, new Uint8Array([1]));

    const view = new DataView(ring.buffer);
    view.setUint32(RING_HEADER_BYTES, 64, true);
    expect(() => ring.tryPop()).toThrow(/published bytes/);
  });
});

describe("filesystem ingestion boundaries", () => {
  const roots: string[] = [];

  afterAll(async () => {
    await Promise.all(roots.map((root) => rm(root, { recursive: true, force: true })));
  });

  test("symlinks cannot pull files outside the scanned root into the index", async () => {
    const root = await mkdtemp(join(tmpdir(), "aether-security-root-"));
    const outside = await mkdtemp(join(tmpdir(), "aether-security-outside-"));
    roots.push(root, outside);

    await writeFile(join(root, "inside.txt"), "inside the workspace");
    await writeFile(join(outside, "secret.txt"), "outside the workspace");
    await symlink(outside, join(root, "escape-dir"));
    await symlink(join(outside, "secret.txt"), join(root, "secret-link.txt"));

    const documents = await scanDirectory(root);
    expect(documents.map((document) => document.title)).toEqual(["inside.txt"]);
    expect(JSON.stringify(documents)).not.toContain("secret");
    expect(documents.every((document) => !document.path?.includes(outside))).toBe(true);
  });
});

describe("indexer request bounds", () => {
  const index = buildIndex([
    {
      id: "file:/work/alpha-one.txt",
      kind: "file",
      title: "alpha-one.txt",
      path: "/work/alpha-one.txt",
      body: "alpha",
    },
    {
      id: "file:/work/alpha-two.txt",
      kind: "file",
      title: "alpha-two.txt",
      path: "/work/alpha-two.txt",
      body: "alpha",
    },
  ]);

  function query(line: string): { results: unknown[]; error?: string } {
    return JSON.parse(handleRequest(index, line));
  }

  test.each([
    ['{"query":"alpha","limit":1e999}', 2],
    ['{"query":"alpha","limit":-1e999}', 1],
    ['{"query":"alpha","limit":0}', 1],
    ['{"query":"alpha","limit":1e9}', 2],
  ])("bounds limit values that overflow JSON numbers (%s)", (line, expected) => {
    const response = query(line);
    expect(response.error).toBeUndefined();
    expect(response.results.length).toBe(expected);
  });

  test("oversized queries do not throw or bypass the result cap", () => {
    const response = query(
      JSON.stringify({ query: "a".repeat(200_000), limit: 8 }),
    );
    expect(response.error).toBeUndefined();
    expect(response.results.length).toBeLessThanOrEqual(8);
  });
});
