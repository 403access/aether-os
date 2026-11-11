/**
 * A deliberately small markdown subset for streaming chat tokens.
 *
 * We do not need a full CommonMark parser in the shell: fenced code, inline
 * code, bold, italic and links cover assistant output. Crucially, an unclosed
 * fence (the normal state while tokens stream in) must render as code rather
 * than leaking markdown syntax.
 */

export type SegmentType = "text" | "code" | "inlineCode" | "bold" | "italic" | "link";

export interface Segment {
  type: SegmentType;
  text: string;
  href?: string;
  language?: string;
}

const FENCE = /```([\w+-]*)\n?([\s\S]*?)(?:```|$)/g;
const INLINE =
  /`([^`\n]+)`|\*\*([^*\n]+)\*\*|\*([^*\n]+)\*|\[([^\]\n]+)\]\(([^)\s]+)\)/g;

export function parseMarkdownLite(input: string): Segment[] {
  if (input.length === 0) return [];

  const segments: Segment[] = [];
  let cursor = 0;

  for (const match of input.matchAll(FENCE)) {
    const index = match.index ?? 0;
    if (index > cursor) pushInline(segments, input.slice(cursor, index));

    const language = match[1] ?? "";
    const body = match[2] ?? "";
    segments.push({
      type: "code",
      text: body.replace(/\n$/, ""),
      ...(language.length > 0 ? { language } : {}),
    });
    cursor = index + match[0].length;
  }

  if (cursor < input.length) pushInline(segments, input.slice(cursor));
  return segments;
}

function pushInline(segments: Segment[], text: string): void {
  if (text.length === 0) return;
  let cursor = 0;

  for (const match of text.matchAll(INLINE)) {
    const index = match.index ?? 0;
    if (index > cursor) segments.push({ type: "text", text: text.slice(cursor, index) });

    if (match[1] !== undefined) {
      segments.push({ type: "inlineCode", text: match[1] });
    } else if (match[2] !== undefined) {
      segments.push({ type: "bold", text: match[2] });
    } else if (match[3] !== undefined) {
      segments.push({ type: "italic", text: match[3] });
    } else if (match[4] !== undefined && match[5] !== undefined) {
      segments.push({ type: "link", text: match[4], href: match[5] });
    }

    cursor = index + match[0].length;
  }

  if (cursor < text.length) segments.push({ type: "text", text: text.slice(cursor) });
}
