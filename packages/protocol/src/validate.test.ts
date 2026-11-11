import { describe, expect, test } from "bun:test";
import {
  KABYLAKE_7700K,
  PROTOCOL_VERSION,
  clamp,
  formatIssues,
  mustParse,
  parseAgentSpec,
  parseCapability,
  parseIndexDocument,
} from "./index";

const validDocument = {
  id: "file:/work/aether/README.md",
  kind: "file",
  title: "README.md",
  subtitle: "/work/aether",
  body: "A single-target Linux workstation OS",
  path: "/work/aether/README.md",
  tags: ["docs", "readme"],
  modifiedAt: 1_760_000_000_000,
  uses: 12,
};

describe("protocol constants", () => {
  test("exposes a versioned protocol and the reference machine", () => {
    expect(PROTOCOL_VERSION).toBe(1);
    expect(KABYLAKE_7700K.cores).toBe(4);
    expect(KABYLAKE_7700K.threads).toBe(8);
    // Kaby Lake predates PKU; the isolation planner depends on this fact.
    expect(KABYLAKE_7700K.pku).toBe(false);
    expect(KABYLAKE_7700K.instructionSets).toContain("AVX2");
  });

  test("clamp is NaN-safe and inclusive", () => {
    expect(clamp(5, 0, 10)).toBe(5);
    expect(clamp(-1, 0, 10)).toBe(0);
    expect(clamp(11, 0, 10)).toBe(10);
    expect(clamp(Number.NaN, 3, 10)).toBe(3);
  });
});

describe("parseIndexDocument", () => {
  test("accepts a complete document and preserves every field", () => {
    const parsed = parseIndexDocument(validDocument);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.value).toEqual(validDocument);
  });

  test("drops unknown fields instead of leaking them downstream", () => {
    const parsed = parseIndexDocument({ ...validDocument, evil: "payload" });
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(Object.keys(parsed.value).sort()).toEqual(
      ["body", "id", "kind", "modifiedAt", "path", "subtitle", "tags", "title", "uses"].sort(),
    );
  });

  test("minimal document is valid", () => {
    const parsed = parseIndexDocument({ id: "app:terminal", kind: "app", title: "Terminal" });
    expect(parsed.ok).toBe(true);
  });

  test.each([
    [{ id: "", kind: "app", title: "x" }, "$.id"],
    [{ id: "a", kind: "spaceship", title: "x" }, "$.kind"],
    [{ id: "a", kind: "app", title: "" }, "$.title"],
    [{ id: "a", kind: "app", title: "x", modifiedAt: -5 }, "$.modifiedAt"],
    [{ id: "a", kind: "app", title: "x", uses: 1.5 }, "$.uses"],
    [{ id: "a", kind: "app", title: "x", tags: "nope" }, "$.tags"],
  ])("rejects invalid document %p", (input, expectedPath) => {
    const parsed = parseIndexDocument(input);
    expect(parsed.ok).toBe(false);
    if (parsed.ok) return;
    expect(parsed.issues.map((issue) => issue.path)).toContain(expectedPath);
  });

  test("rejects oversized tag lists", () => {
    const parsed = parseIndexDocument({
      id: "a",
      kind: "app",
      title: "x",
      tags: Array.from({ length: 65 }, (_, index) => `tag${index}`),
    });
    expect(parsed.ok).toBe(false);
    if (parsed.ok) return;
    expect(parsed.issues[0]?.message).toContain("at most 64");
  });

  test("rejects non-objects without throwing", () => {
    for (const value of [null, 42, "doc", [], undefined]) {
      expect(parseIndexDocument(value).ok).toBe(false);
    }
  });
});

describe("parseAgentSpec", () => {
  const spec = {
    id: "refactor-worker-1",
    name: "Refactor worker",
    trust: "untrusted",
    tier: "process-landlock",
    cpuWeight: 40,
    memoryMiB: 1024,
    capabilities: [{ scope: "fs.read", resource: "fs:/work/project", ttlMs: 60_000 }],
    timeoutMs: 120_000,
  };

  test("accepts a realistic worker spec", () => {
    const parsed = parseAgentSpec(spec);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.value.capabilities[0]?.scope).toBe("fs.read");
    expect(parsed.value.timeoutMs).toBe(120_000);
  });

  test("requires a safe agent id", () => {
    const parsed = parseAgentSpec({ ...spec, id: "../escape" });
    expect(parsed.ok).toBe(false);
    if (parsed.ok) return;
    expect(parsed.issues[0]?.path).toBe("$.id");
  });

  test("rejects out-of-range resources", () => {
    expect(parseAgentSpec({ ...spec, cpuWeight: 0 }).ok).toBe(false);
    expect(parseAgentSpec({ ...spec, cpuWeight: 101 }).ok).toBe(false);
    expect(parseAgentSpec({ ...spec, memoryMiB: 8 }).ok).toBe(false);
    expect(parseAgentSpec({ ...spec, capabilities: [{ scope: "root", resource: "*" }] }).ok).toBe(
      false,
    );
  });

  test("reports every invalid capability with its index", () => {
    const parsed = parseAgentSpec({
      ...spec,
      capabilities: [
        { scope: "fs.read", resource: "fs:/a" },
        { scope: "fs.write", resource: "" },
      ],
    });
    expect(parsed.ok).toBe(false);
    if (parsed.ok) return;
    expect(parsed.issues.map((issue) => issue.path)).toContain("$.capabilities[1].resource");
  });
});

describe("parseCapability", () => {
  const capability = {
    id: "cap_1",
    subject: "refactor-worker-1",
    scope: "agent.collab",
    resource: "channel:handoff",
    issuedAt: 1000,
    expiresAt: null,
    delegable: false,
    generation: 0,
    parentId: null,
  };

  test("accepts a non-expiring capability", () => {
    const parsed = parseCapability(capability);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;
    expect(parsed.value.expiresAt).toBeNull();
    expect(parsed.value.parentId).toBeNull();
  });

  test("rejects a non-boolean delegable flag", () => {
    const parsed = parseCapability({ ...capability, delegable: "yes" });
    expect(parsed.ok).toBe(false);
    if (parsed.ok) return;
    expect(parsed.issues[0]?.path).toBe("$.delegable");
  });

  test("rejects negative generations", () => {
    expect(parseCapability({ ...capability, generation: -1 }).ok).toBe(false);
  });
});

describe("mustParse / formatIssues", () => {
  test("throws a readable error for invalid input", () => {
    expect(() => mustParse(parseAgentSpec({}), "agent")).toThrow(/agent is invalid: \$\.id/);
  });

  test("formats multiple issues on one line", () => {
    expect(
      formatIssues([
        { path: "$.a", message: "one" },
        { path: "$.b", message: "two" },
      ]),
    ).toBe("$.a: one; $.b: two");
  });
});
