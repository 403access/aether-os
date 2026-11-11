import {
  KABYLAKE_7700K,
  type AgentSpec,
  type Enforcement,
  type HardwareProfile,
  type IsolationPlan,
  type IsolationTier,
} from "@aether/protocol";

/**
 * Isolation planning.
 *
 * The reference machine (Kaby Lake i7-7700K) has **no PKU**, so the original
 * "10 ns WRPKRU sandbox" design is unavailable. The planner therefore refuses
 * to place untrusted code in the shell's address space and falls back to
 * process isolation enforced by Landlock + seccomp-BPF + `no_new_privs`, with
 * microVMs reserved for hostile workloads.
 *
 * The planner never launches anything. It produces a plan that the supervisor
 * either executes or rejects; `blocked` is a hard stop.
 */

export const PRACTICAL_MEMORY_WARN_MIB = 2_048;
export const HIGH_CPU_WEIGHT = 50;

export interface IsolationOptions {
  hardware?: HardwareProfile;
  /** Overrides the practical memory guidance for this machine. */
  memoryBudgetMiB?: number;
}

export function planIsolation(spec: AgentSpec, options: IsolationOptions = {}): IsolationPlan {
  const hardware = options.hardware ?? KABYLAKE_7700K;
  const budget = options.memoryBudgetMiB ?? PRACTICAL_MEMORY_WARN_MIB;

  const rationale: string[] = [];
  const warnings: string[] = [];
  const blockers: string[] = [];

  let tier: IsolationTier = spec.tier;

  switch (spec.trust) {
    case "hostile":
      tier = "microvm";
      rationale.push(
        "Hostile workloads always run in a microVM; they never share the host kernel.",
      );
      break;
    case "untrusted":
      if (tier === "in-process") {
        tier = "process-landlock";
        warnings.push(
          `Agent "${spec.id}" requested in-process isolation but is untrusted; escalated to a Landlock process.`,
        );
        rationale.push(
          "Untrusted code is never placed in the shell address space; isolation escalated.",
        );
      } else {
        rationale.push("Untrusted agent runs outside the shell address space.");
      }
      break;
    case "trusted":
      rationale.push(
        "Trusted agent: the Rust ownership model and code review are the boundary.",
      );
      break;
  }

  let enforcement: Enforcement;

  if (tier === "microvm") {
    enforcement = "microvm-vfio";
    rationale.push("Devices are assigned directly via VFIO; no emulated hardware.");
    if (!hardware.vtX || !hardware.vtD) {
      blockers.push("microVM tier requires VT-x and VT-d (IOMMU)");
    }
    if (spec.image === undefined || spec.image.length === 0) {
      blockers.push("microVM tier requires an image id");
    }
  } else if (tier === "process-landlock") {
    enforcement = "landlock-seccomp";
    rationale.push(
      "Filesystem allowlist (Landlock) + syscall allowlist (seccomp-BPF) + no_new_privs.",
    );
    if (!hardware.landlock) {
      blockers.push("process isolation requires Landlock (Linux 5.13+)");
    }
    if (!hardware.pku) {
      warnings.push(
        "No PKU on this CPU: cross-agent memory separation relies on page tables (processes), not WRPKRU.",
      );
    }
  } else {
    // Trusted, in-process. PKU is defense in depth when the silicon has it.
    enforcement = hardware.pku ? "pku" : "none";
    if (hardware.pku) {
      rationale.push("PKU domains isolate agent heaps with WRPKRU (no syscalls).");
    } else {
      rationale.push(
        "No PKU on this CPU: in-process isolation is Rust ownership only, so trusted code only.",
      );
    }
  }

  if (spec.memoryMiB > budget) {
    warnings.push(
      `Agent requests ${spec.memoryMiB} MiB, above the ${budget} MiB practical budget for a 4c/8t machine.`,
    );
  }
  if (spec.cpuWeight > HIGH_CPU_WEIGHT) {
    warnings.push(
      `Agent requests ${spec.cpuWeight}% of a core; concurrent CPU-bound agents will contend on four physical cores.`,
    );
  }
  if (spec.timeoutMs === undefined) {
    warnings.push("No timeout set: the agent may run indefinitely if it stalls.");
  }
  if (spec.trust !== "trusted" && spec.capabilities.some((request) => request.scope === "agent.spawn")) {
    blockers.push("Untrusted agents may not spawn further agents (no self-replication).");
  }
  if (
    spec.capabilities.some(
      (request) =>
        request.scope === "fs.write" && (request.resource === "*" || request.resource === "fs:/"),
    )
  ) {
    warnings.push("Write access to the entire filesystem defeats the isolation plan.");
  }

  return {
    agentId: spec.id,
    tier,
    enforcement,
    rationale,
    warnings,
    privileged: tier === "microvm",
    blocked: blockers.length > 0,
    blockers,
  };
}

/** Human-readable plan, used by the docs demo and the admin overlay. */
export function describePlan(plan: IsolationPlan): string {
  const lines = [
    `${plan.agentId}: ${plan.tier} (${plan.enforcement})${plan.privileged ? " [privileged]" : ""}`,
    ...plan.rationale.map((line) => `  · ${line}`),
  ];
  if (plan.warnings.length > 0) {
    lines.push("  warnings:");
    lines.push(...plan.warnings.map((line) => `    ! ${line}`));
  }
  if (plan.blocked) {
    lines.push("  blocked:");
    lines.push(...plan.blockers.map((line) => `    × ${line}`));
  }
  return lines.join("\n");
}
