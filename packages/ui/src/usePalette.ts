import { useCallback, useEffect, useRef, useState } from "react";
import {
  moveSelection,
  type PaletteEngine,
  type PaletteItemResult,
  type PaletteResult,
} from "@aether/palette";
import type { KeyboardEvent } from "react";

/**
 * React binding for the headless palette engine.
 *
 * Owns only view state: the query string, the latest non-stale result and a
 * loading flag. Ranking, grouping and selection maths stay in `@aether/palette`
 * so the compositor and the browser behave identically.
 */
export interface UsePaletteOptions {
  engine: PaletteEngine;
  limit?: number;
  onAction?: (item: PaletteItemResult) => void;
  onDismiss?: () => void;
  initialQuery?: string;
}

export interface UsePaletteApi {
  input: string;
  setInput: (value: string) => void;
  result: PaletteResult | null;
  loading: boolean;
  error: string | null;
  move: (delta: number) => void;
  select: (index?: number) => void;
  onKeyDown: (event: KeyboardEvent<HTMLInputElement>) => void;
}

export function usePalette(options: UsePaletteOptions): UsePaletteApi {
  const { engine, limit, onAction, onDismiss, initialQuery = "" } = options;
  const [input, setInput] = useState(initialQuery);
  const [result, setResult] = useState<PaletteResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const resultRef = useRef<PaletteResult | null>(null);

  useEffect(() => {
    resultRef.current = result;
  }, [result]);

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);

    engine
      .query(input, { signal: controller.signal, ...(limit === undefined ? {} : { limit }) })
      .then((next) => {
        if (next.stale || controller.signal.aborted) return;
        setResult(next);
        setError(null);
      })
      .catch((reason: unknown) => {
        if (reason instanceof Error && reason.name === "AbortError") return;
        setError(reason instanceof Error ? reason.message : String(reason));
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });

    return () => controller.abort();
  }, [engine, input, limit]);

  const setQuery = useCallback((value: string) => {
    setInput(value);
  }, []);

  const move = useCallback((delta: number) => {
    setResult((current) => (current === null ? current : moveSelection(current, delta)));
  }, []);

  const select = useCallback(
    (index?: number) => {
      const current = resultRef.current;
      if (current === null) return;
      const target = index === undefined ? current.items[current.selectedIndex] : current.items[index];
      if (target !== undefined) onAction?.(target);
    },
    [onAction],
  );

  const onKeyDown = useCallback(
    (event: KeyboardEvent<HTMLInputElement>) => {
      switch (event.key) {
        case "ArrowDown":
          event.preventDefault();
          move(1);
          return;
        case "ArrowUp":
          event.preventDefault();
          move(-1);
          return;
        case "PageDown":
          event.preventDefault();
          move(5);
          return;
        case "PageUp":
          event.preventDefault();
          move(-5);
          return;
        case "Home": {
          event.preventDefault();
          const current = resultRef.current;
          if (current !== null) setResult({ ...current, selectedIndex: 0 });
          return;
        }
        case "End": {
          event.preventDefault();
          const current = resultRef.current;
          if (current !== null && current.items.length > 0) {
            setResult({ ...current, selectedIndex: current.items.length - 1 });
          }
          return;
        }
        case "Enter":
          event.preventDefault();
          select();
          return;
        case "Escape":
          event.preventDefault();
          onDismiss?.();
          return;
        default:
          break;
      }

      if (event.ctrlKey && event.key === "n") {
        event.preventDefault();
        move(1);
      } else if (event.ctrlKey && event.key === "p") {
        event.preventDefault();
        move(-1);
      }
    },
    [move, onDismiss, select],
  );

  return { input, setInput: setQuery, result, loading, error, move, select, onKeyDown };
}
