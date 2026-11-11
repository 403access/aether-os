import { describe, expect, test } from "bun:test";
import { KABYLAKE_7700K, type AgentSpec, type HardwareProfile } from "@aether/protocol";
import { describePlan, planIsolation } from "../src/index";

function spec(overrides: Partial<AgentSpec> = {}): AgentSpec {
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

const withPku: HardwareProfile = { ...KABYLAKE_7700K, id: "future", pku: true };

describe("planIsolation on the reference machine", () => {
  test("untrusted agents get a Landlock process, never the shell address space", () => {
    const plan = planIsolation(spec());
    expect(plan.tier).toBe("process-landlock");
    expect(plan.enforcement).toBe("landlock-seccomp");
    expect(plan.blocked).toBe(false);
    expect(plan.privileged).toBe(false);
    expect(plan.warnings.join(" ")).toContain("No PKU");
  });

  test("in-process requests from untrusted agents are escalated", () => {
    const plan = planIsolation(spec({ tier: "in-process" }));
    expect(plan.tier).toBe("process-landlock");
    expect(plan.warnings.join(" ")).toContain("escalated");
    expect(plan.rationale.join(" ")).toContain("never placed in the shell address space");
  });

  test("trusted agents may stay in-process with no enforcement on Kaby Lake", () => {
    const plan = planIsolation(spec({ trust: "trusted", tier: "in-process" }));
    expect(plan.tier).toBe("in-process");
    expect(plan.enforcement).toBe("none");
    expect(plan.rationale.join(" ")).toContain("Rust ownership");
  });

  test("trusted in-process agents use PKU when the CPU supports it", () => {
    const plan = planIsolation(spec({ trust: "trusted", tier: "in-process" }), {
      hardware: withPku,
    });
    expect(plan.enforcement).toBe("pku");
  });

  test("hostile workloads are forced into a microVM and require an image", () => {
    const plan = planIsolation(spec({ trust: "hostile", tier: "in-process" }));
    expect(plan.tier).toBe("microvm");
    expect(plan.enforcement).toBe("microvm-vfio");
    expect(plan.privileged).toBe(true);
    expect(plan.blocked).toBe(true);
    expect(plan.blockers.join(" ")).toContain("image");
  });

  test("microVM plans pass when an image and IOMMU are available", () => {
    const plan = planIsolation(
      spec({ trust: "hostile", tier: "microvm", image: "aether/worker:0.1" }),
    );
    expect(plan.blocked).toBe(false);
    expect(plan.blockers).toEqual([]);
  });

  test("microVM plans are blocked when VT-d is missing", () => {
    const plan = planIsolation(spec({ trust: "hostile", tier: "microvm", image: "x" }), {
      hardware: { ...KABYLAKE_7700K, vtD: false },
    });
    expect(plan.blocked).toBe(true);
    expect(plan.blockers.join(" ")).toContain("VT-d");
  });
});

describe("isolation guard rails", () => {
  test("untrusted agents may not spawn agents", () => {
    const plan = planIsolation(
      spec({ capabilities: [{ scope: "agent.spawn", resource: "*" }] }),
    );
    expect(plan.blocked).toBe(true);
    expect(plan.blockers.join(" ")).toContain("spawn");
  });

  test("trusted agents may spawn agents", () => {
    const plan = planIsolation(
      spec({
        trust: "trusted",
        tier: "in-process",
        capabilities: [{ scope: "agent.spawn", resource: "*" }],
      }),
    );
    expect(plan.blocked).toBe(false);
  });

  test("warns on memory, CPU weight, missing timeout and root write access", () => {
    const plan = planIsolation(
      spec({
        memoryMiB: 4_096,
        cpuWeight: 80,
        timeoutMs: undefined,
        capabilities: [{ scope: "fs.write", resource: "*" }],
      }),
      { memoryBudgetMiB: 2_048 },
    );
    const warnings = plan.warnings.join("\n");
    expect(warnings).toContain("4096 MiB");
    expect(warnings).toContain("80%");
    expect(warnings).toContain("No timeout");
    expect(warnings).toContain("entire filesystem");
    expect(plan.blocked).toBe(false);
  });

  test("process isolation is blocked without Landlock", () => {
    const plan = planIsolation(spec(), {
      hardware: { ...KABYLAKE_7700K, landlock: false },
    });
    expect(plan.blocked).toBe(true);
    expect(plan.blockers.join(" ")).toContain("Landlock");
  });
});

describe("describePlan", () => {
  test("renders tier, rationale, warnings and blockers", () => {
    const plan = planIsolation(spec({ trust: "hostile", tier: "microvm", memoryMiB: 4_096 }));
    const text = describePlan(plan);
    expect(text).toContain("worker-a: microvm (microvm-vfio) [privileged]");
    expect(text).toContain("warnings:");
    expect(text).toContain("blocked:");
  });
});
