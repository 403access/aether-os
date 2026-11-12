import { describe, expect, test } from "bun:test";
import { readFile, readdir } from "node:fs/promises";
import { join, resolve } from "node:path";

/**
 * Architectural boundary between the physical target (os/nixos) and the
 * development VM (os/vm). This test is intentionally blunt: it reads the tree
 * and fails on cross-imports or VM-only posture leaking into the reference.
 *
 * Shared composition is allowed only through os/common/base.nix (Nix) or
 * packages/ (application code).
 */

const ROOT = resolve(import.meta.dir, "..");

async function collect(relativeDir: string): Promise<string[]> {
  const dir = join(ROOT, relativeDir);
  const entries = await readdir(dir, { withFileTypes: true });
  const files: string[] = [];

  for (const entry of entries) {
    const path = `${relativeDir}/${entry.name}`;
    if (entry.isDirectory()) files.push(...(await collect(path)));
    else if (entry.isFile()) files.push(path);
  }
  return files;
}

async function source(relativePath: string): Promise<string> {
  return readFile(join(ROOT, relativePath), "utf8");
}

async function nixSources(relativeDir: string): Promise<Array<{ path: string; text: string }>> {
  const files = (await collect(relativeDir)).filter((path) => path.endsWith(".nix"));
  return Promise.all(files.map(async (path) => ({ path, text: await source(path) })));
}

describe("physical/VM separation", () => {
  test("the reference target never references the VM tree", async () => {
    for (const file of await nixSources("os/nixos")) {
      expect(file.text, `${file.path} must not reference os/vm`).not.toMatch(
        /os\/vm|\.\.\/vm|aether-vm|vm-image/,
      );
      expect(file.text, `${file.path} must not configure VM disks`).not.toContain(
        "virtualisation.diskSize",
      );
    }
  });

  test("the VM never imports the reference target", async () => {
    for (const file of await nixSources("os/vm")) {
      expect(file.text, `${file.path} must not import ../nixos`).not.toMatch(
        /\.\.\/nixos|imports\s*=\s*\[[^\]]*os\/nixos/,
      );
    }
  });

  test("VM-only security posture never appears in the reference", async () => {
    const reference = (await nixSources("os/nixos")).map((file) => file.text).join("\n");
    expect(reference).not.toContain("wheelNeedsPassword = false");
    expect(reference).not.toContain("firewall.enable = false");
    expect(reference).not.toContain("PasswordAuthentication = true");
    expect(reference).not.toContain("virtualisation.diskSize");
  });

  test("both targets compose the shared base", async () => {
    expect(await source("os/nixos/configuration.nix")).toContain("../common/base.nix");
    expect(await source("os/vm/configuration.nix")).toContain("../common/base.nix");
  });

  test("the shared module contains no hardware, users, storage or security policy", async () => {
    const common = await source("os/common/base.nix");
    expect(common).not.toMatch(/fileSystems|boot\.loader|users\.|virtualisation|firewall|sudo/);
  });

  test("kestrel's flake block excludes the VM tree", async () => {
    const flake = await source("flake.nix");
    const start = flake.indexOf("nixosConfigurations.kestrel");
    const end = flake.indexOf("nixosConfigurations.installer");
    expect(start).toBeGreaterThan(-1);
    expect(end).toBeGreaterThan(start);
    const kestrelBlock = flake.slice(start, end);
    expect(kestrelBlock).not.toContain("os/vm");
  });

  test("the VM package uses only the VM configuration", async () => {
    const flake = await source("flake.nix");
    const start = flake.indexOf("packages.aarch64-linux");
    const end = flake.indexOf("devShells.");
    expect(start).toBeGreaterThan(-1);
    expect(end).toBeGreaterThan(start);
    const vmBlock = flake.slice(start, end);
    expect(vmBlock).toContain("./os/vm/configuration.nix");
    expect(vmBlock).not.toContain("os/nixos");
  });
});
