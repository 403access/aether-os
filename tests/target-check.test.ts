import { afterEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const roots: string[] = [];

async function fixture(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "aether-target-check-"));
  roots.push(root);
  await mkdir(join(root, "proc"), { recursive: true });
  await mkdir(join(root, "sys/firmware/efi"), { recursive: true });
  await mkdir(join(root, "sys/firmware/acpi/tables"), { recursive: true });
  await mkdir(join(root, "sys/module/i915"), { recursive: true });
  await mkdir(join(root, "dev/dri"), { recursive: true });
  await mkdir(join(root, "dev/input"), { recursive: true });
  await mkdir(join(root, "sys/class/drm/card0-HDMI-A-1"), { recursive: true });
  await writeFile(
    join(root, "proc/cpuinfo"),
    "model name\t: Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz\n",
  );
  await writeFile(join(root, "proc/meminfo"), "MemTotal:       16384000 kB\n");
  await writeFile(join(root, "proc/cmdline"), "root=LABEL=aether quiet\n");
  await writeFile(join(root, "dev/dri/card0"), "");
  await writeFile(join(root, "dev/dri/renderD128"), "");
  await writeFile(join(root, "dev/input/event0"), "");
  await writeFile(join(root, "sys/class/drm/card0-HDMI-A-1/status"), "connected\n");
  await writeFile(join(root, "sys/class/drm/card0-HDMI-A-1/modes"), "1920x1080\n");
  return root;
}

function check(root: string) {
  return Bun.spawnSync(["bash", "scripts/target-check.sh", "--root", root], {
    stdout: "pipe",
    stderr: "pipe",
  });
}

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

describe("target preflight", () => {
  test("passes on a complete 7700K fixture", async () => {
    const result = check(await fixture());
    const output = result.stdout.toString() + result.stderr.toString();
    expect(result.exitCode).toBe(0);
    expect(output).toContain("CPU: Intel(R) Core(TM) i7-7700K");
    expect(output).toContain("connected connectors: card0-HDMI-A-1");
    expect(output).toContain("RESULT: ready");
  });

  test("fails when the DRM device is missing", async () => {
    const root = await fixture();
    await rm(join(root, "dev/dri"), { recursive: true, force: true });
    const result = check(root);
    const output = result.stdout.toString() + result.stderr.toString();
    expect(result.exitCode).toBe(1);
    expect(output).toContain("no /dev/dri/card*");
    expect(output).toContain("RESULT: 1 failure(s)");
  });

  test("warns but stays ready for a different CPU", async () => {
    const root = await fixture();
    await writeFile(join(root, "proc/cpuinfo"), "model name\t: Intel(R) Core(TM) i5-6500\n");
    const result = check(root);
    const output = result.stdout.toString();
    expect(result.exitCode).toBe(0);
    expect(output).toContain("WARN  CPU is Intel(R) Core(TM) i5-6500");
  });
});
