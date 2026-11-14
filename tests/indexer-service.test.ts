import { describe, expect, test } from "bun:test";
import { buildIndex, handleRequest } from "../apps/indexer-service/src/service";

const documents = [
  {
    id: "file:/home/user/notes-alpha.txt",
    kind: "file" as const,
    title: "notes-alpha.txt",
    subtitle: "/home/user",
    path: "/home/user/notes-alpha.txt",
    tags: ["file", "txt"],
    body: "the indexer magic word",
  },
  {
    id: "file:/home/user/readme.md",
    kind: "file" as const,
    title: "readme.md",
    subtitle: "/home/user",
    path: "/home/user/readme.md",
    tags: ["file", "md"],
    body: "project documentation",
  },
];

function query(index: ReturnType<typeof buildIndex>, request: unknown): {
  results: Array<{ path: string; title: string }>;
  error?: string;
} {
  return JSON.parse(handleRequest(index, JSON.stringify(request)));
}

describe("indexer service", () => {
  test("matches file names and body content", () => {
    const index = buildIndex(documents);
    const byName = query(index, { query: "alpha" });
    expect(byName.results.map((result) => result.path)).toEqual([
      "/home/user/notes-alpha.txt",
    ]);

    const byContent = query(index, { query: "magic" });
    expect(byContent.results[0]?.path).toBe("/home/user/notes-alpha.txt");
  });

  test("returns an empty list for empty queries", () => {
    const index = buildIndex(documents);
    expect(query(index, { query: "   " }).results).toEqual([]);
  });

  test("reports malformed JSON without throwing", () => {
    const index = buildIndex(documents);
    const response = JSON.parse(handleRequest(index, "{not json"));
    expect(response.results).toEqual([]);
    expect(response.error).toBe("invalid json");
  });

  test("honours the result limit", () => {
    const index = buildIndex(documents);
    const response = query(index, { query: "user", limit: 1 });
    expect(response.results.length).toBe(1);
  });
});
