import { afterEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const roots: string[] = [];
const UID = 1000;

/**
 * A /proc fixture with one process per memory-relevant category. Unit strings
 * are kB in status/smaps_rollup and KiB in fdinfo, exactly like Linux.
 */
async function fixture(options: { usedMb?: number } = {}): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "aether-memory-report-"));
  roots.push(root);
  const usedMb = options.usedMb ?? 512;
  const totalKb = 16 * 1024 * 1024;
  const availableKb = totalKb - usedMb * 1024;
  await mkdir(join(root, "etc"), { recursive: true });
  await mkdir(join(root, "proc"), { recursive: true });
  await writeFile(
    join(root, "proc/meminfo"),
    [
      `MemTotal:       ${totalKb} kB`,
      `MemAvailable:   ${availableKb} kB`,
      "Shmem:            42000 kB",
      "SReclaimable:    120000 kB",
      "SwapTotal:            0 kB",
      "SwapFree:             0 kB",
      "",
    ].join("\n"),
  );
  await writeFile(join(root, "etc/hostname"), "kestrel\n");

  await process(root, {
    pid: 4242,
    name: "aether-shell-drm",
    uid: UID,
    pssKb: 48_000,
    swapKb: 0,
    drmKb: 8192,
  });
  await process(root, {
    pid: 999,
    name: "aether-index",
    uid: UID,
    pssKb: 10_000,
    swapKb: 2048,
  });
  await process(root, {
    pid: 777,
    name: "pipewire",
    uid: UID,
    pssKb: 6_000,
    swapKb: 0,
  });
  await process(root, {
    pid: 1,
    name: "systemd",
    uid: 0,
    pssKb: 8_000,
    swapKb: 0,
  });
  return root;
}

async function process(
  root: string,
  spec: { pid: number; name: string; uid: number; pssKb: number; swapKb: number; drmKb?: number },
): Promise<void> {
  const dir = join(root, "proc", String(spec.pid));
  await mkdir(join(dir, "fdinfo"), { recursive: true });
  await writeFile(join(dir, "comm"), `${spec.name}\n`);
  await writeFile(
    join(dir, "status"),
    `Name:\t${spec.name}\nUid:\t${spec.uid}\t${spec.uid}\t${spec.uid}\t${spec.uid}\nVmRSS:\t${spec.pssKb} kB\n`,
  );
  await writeFile(
    join(dir, "smaps_rollup"),
    `Rss: ${spec.pssKb} kB\nPss: ${spec.pssKb} kB\nSwap: ${spec.swapKb} kB\n`,
  );
  if (spec.drmKb !== undefined) {
    await writeFile(
      join(dir, "fdinfo/3"),
      `drm-driver:\ti915\ndrm-resident-system:\t${spec.drmKb} KiB\ndrm-total-system:\t${spec.drmKb} KiB\n`,
    );
  }
}

function report(root: string, args: string[] = []) {
  return Bun.spawnSync(["bash", "scripts/memory-report.sh", "--root", root, ...args], {
    stdout: "pipe",
    stderr: "pipe",
  });
}

afterEach(async () => {
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

describe("memory report", () => {
  test("attributes PSS and DRM to categories as JSON", async () => {
    const root = await fixture();
    const json = join(root, "report.json");
    const result = report(root, [
      "--seconds",
      "0",
      "--uid",
      String(UID),
      "--gate",
      "attributed",
      "--json",
      json,
    ]);
    const output = result.stdout.toString() + result.stderr.toString();
    expect(result.exitCode).toBe(0);
    expect(output).toContain("aether-shell-drm");
    expect(output).toContain("RESULT: pass");

    const parsed = JSON.parse(await readFile(json, "utf8"));
    expect(parsed.machine.used_mb).toBe(512);
    expect(parsed.attributed.pss_mb).toBe(62.5); // 46.9 + 9.8 + 5.9 MiB
    expect(parsed.attributed.drm_mb).toBe(8);
    expect(parsed.pass).toBe(true);
    const shell = parsed.categories.find((entry: { name: string }) => entry.name === "shell");
    expect(shell.processes).toBe(1);
    expect(shell.pss_mb).toBe(46.9);
    const system = parsed.categories.find((entry: { name: string }) => entry.name === "system");
    expect(system.processes).toBe(1); // root services are not attributed to the session
  });

  test("fails the attributed gate when the session exceeds the budget", async () => {
    const root = await fixture();
    const result = report(root, [
      "--seconds",
      "0",
      "--uid",
      String(UID),
      "--gate",
      "attributed",
      "--budget-mb",
      "32",
      "--check",
    ]);
    expect(result.exitCode).toBe(1);
    expect(result.stdout.toString()).toContain("RESULT: fail");
    expect(result.stdout.toString()).toContain("over budget");
  });

  test("the machine gate uses MemTotal - MemAvailable", async () => {
    const root = await fixture({ usedMb: 300 });
    const result = report(root, ["--seconds", "0", "--budget-mb", "200", "--check"]);
    expect(result.exitCode).toBe(1);
    expect(result.stdout.toString()).toContain("300.0 MiB > 200 MiB");
  });

  test("needs a real /proc or a fixture root", async () => {
    const root = await mkdtemp(join(tmpdir(), "aether-memory-empty-"));
    roots.push(root);
    const result = report(root, ["--seconds", "0"]);
    expect(result.exitCode).toBe(2);
    expect(result.stderr.toString()).toContain("Linux-only");
  });
});
