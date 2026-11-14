import { watch } from "node:fs";
import { join } from "node:path";

export interface WatchTreeOptions {
  /** Debounce window per changed path. Default 120ms. */
  debounceMs?: number;
  onPath: (path: string) => void;
  onError?: (error: unknown) => void;
}

/**
 * Watch a directory tree using the platform watcher. On Linux `fs.watch` is
 * backed by inotify; on macOS by FSEvents. Paths are debounced so a write that
 * emits several events only triggers one update.
 *
 * Returns a stop function that clears pending timers and closes the watcher.
 */
export function watchTree(root: string, options: WatchTreeOptions): () => void {
  const debounce = options.debounceMs ?? 120;
  const pending = new Map<string, ReturnType<typeof setTimeout>>();
  let watcher: ReturnType<typeof watch>;
  try {
    watcher = watch(root, { recursive: true }, (_event, filename) => {
      if (filename === null) return;
      const path = join(root, filename.toString());
      const previous = pending.get(path);
      if (previous !== undefined) clearTimeout(previous);
      pending.set(
        path,
        setTimeout(() => {
          pending.delete(path);
          options.onPath(path);
        }, debounce),
      );
    });
  } catch (error) {
    options.onError?.(error);
    return () => {};
  }

  return () => {
    for (const timer of pending.values()) clearTimeout(timer);
    pending.clear();
    watcher.close();
  };
}
