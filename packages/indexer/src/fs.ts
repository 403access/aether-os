import { readFile, readdir, stat } from "node:fs/promises";
import { basename, dirname, extname, join, resolve } from "node:path";
import type { IndexDocument } from "@aether/protocol";

/**
 * Filesystem ingestion for `aether-index`.
 *
 * This is the reference implementation of the ingestion rules; the shipping
 * daemon uses inotify for incremental updates but reuses these same policies:
 *  - only text-ish files contribute body content
 *  - binaries and media are still searchable by name and path
 *  - noisy build/VCS directories are skipped by name
 */

export const TEXT_EXTENSIONS: ReadonlySet<string> = new Set([
  "txt",
  "md",
  "mdx",
  "markdown",
  "rs",
  "ts",
  "tsx",
  "js",
  "jsx",
  "mjs",
  "cjs",
  "json",
  "jsonc",
  "toml",
  "yaml",
  "yml",
  "nix",
  "sh",
  "bash",
  "zsh",
  "fish",
  "py",
  "go",
  "c",
  "h",
  "cc",
  "cpp",
  "hpp",
  "css",
  "scss",
  "html",
  "svg",
  "sql",
  "lua",
  "zig",
  "java",
  "kt",
  "swift",
  "rb",
  "php",
  "ini",
  "conf",
  "env",
  "lock",
  "log",
  "csv",
  "tsv",
]);

export const DEFAULT_IGNORES: ReadonlySet<string> = new Set([
  "node_modules",
  ".git",
  ".hg",
  ".svn",
  "dist",
  "build",
  ".docusaurus",
  ".next",
  ".turbo",
  "target",
  ".cache",
  "coverage",
  "vendor",
  "__pycache__",
]);

export interface FileDocumentInput {
  path: string;
  content?: string;
  modifiedAt?: number;
}

export function extensionOf(path: string): string {
  const extension = extname(path).toLowerCase();
  return extension.startsWith(".") ? extension.slice(1) : extension;
}

/** Cheap binary sniff: NUL bytes in the first 4 KiB mean "do not index body". */
export function isProbablyText(buffer: Uint8Array): boolean {
  const sample = buffer.subarray(0, Math.min(buffer.length, 4096));
  for (const byte of sample) {
    if (byte === 0) return false;
  }
  return true;
}

export function deriveFileDocument(input: FileDocumentInput): IndexDocument {
  const title = basename(input.path) || input.path;
  const directory = dirname(input.path);
  const extension = extensionOf(input.path);
  const tags = ["file"];
  if (extension.length > 0) tags.push(extension);

  const document: IndexDocument = {
    id: `file:${input.path}`,
    kind: "file",
    title,
    subtitle: directory,
    path: input.path,
    tags,
  };

  if (input.modifiedAt !== undefined) document.modifiedAt = input.modifiedAt;
  if (input.content !== undefined && input.content.length > 0) {
    const textish = extension.length === 0 || TEXT_EXTENSIONS.has(extension);
    if (textish) document.body = input.content;
  }
  return document;
}

export interface ScanOptions {
  maxFiles?: number;
  maxDepth?: number;
  /** Bytes read per file for body extraction. Default 256 KiB. */
  maxBytesPerFile?: number;
  ignore?: ReadonlySet<string>;
  includeHidden?: boolean;
  signal?: AbortSignal;
}

/**
 * Recursively scan a directory into index documents. Deterministic ordering
 * (sorted entries) makes snapshot tests and e2e assertions stable.
 */
export async function scanDirectory(
  root: string,
  options: ScanOptions = {},
): Promise<IndexDocument[]> {
  const maxFiles = options.maxFiles ?? 50_000;
  const maxDepth = options.maxDepth ?? 12;
  const maxBytesPerFile = options.maxBytesPerFile ?? 256 * 1024;
  const ignore = options.ignore ?? DEFAULT_IGNORES;
  const includeHidden = options.includeHidden ?? false;
  const absoluteRoot = resolve(root);
  const documents: IndexDocument[] = [];
  const aborted = (): boolean => options.signal?.aborted === true;

  async function walk(directory: string, depth: number): Promise<void> {
    if (documents.length >= maxFiles) return;
    if (depth > maxDepth) return;
    if (aborted()) return;

    let entries;
    try {
      entries = await readdir(directory, { withFileTypes: true });
    } catch {
      return;
    }
    entries.sort((a, b) => a.name.localeCompare(b.name));

    for (const entry of entries) {
      if (documents.length >= maxFiles) return;
      if (aborted()) return;
      if (entry.isSymbolicLink()) continue;

      const fullPath = join(directory, entry.name);

      if (entry.isDirectory()) {
        if (ignore.has(entry.name)) continue;
        if (!includeHidden && entry.name.startsWith(".")) continue;
        await walk(fullPath, depth + 1);
        continue;
      }

      if (!entry.isFile()) continue;
      if (ignore.has(entry.name)) continue;
      if (!includeHidden && entry.name.startsWith(".")) continue;

      try {
        const info = await stat(fullPath);
        const document = deriveFileDocument({
          path: fullPath,
          modifiedAt: Math.floor(info.mtimeMs),
        });

        const extension = extensionOf(fullPath);
        if (info.size <= maxBytesPerFile && TEXT_EXTENSIONS.has(extension)) {
          const buffer = await readFile(fullPath);
          if (isProbablyText(buffer)) {
            document.body = buffer.subarray(0, maxBytesPerFile).toString("utf8");
          }
        }

        documents.push(document);
      } catch {
        // Unreadable entries are skipped; one bad file must not fail a scan.
      }
    }
  }

  await walk(absoluteRoot, 0);
  return documents;
}
