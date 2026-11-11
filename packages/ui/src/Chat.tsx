import type { ReactNode } from "react";
import { parseMarkdownLite, type Segment } from "./markdown";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  streaming?: boolean;
  model?: string;
  at?: string;
}

export interface ChatTranscriptProps {
  messages: readonly ChatMessage[];
  empty?: ReactNode;
  className?: string;
}

/**
 * Streaming AI transcript. Uses the same markdown subset as the shell's text
 * layer, and keeps a stable DOM shape while tokens append so the compositor's
 * damage tracking only invalidates the tail.
 */
export function ChatTranscript({ messages, empty, className }: ChatTranscriptProps): ReactNode {
  if (messages.length === 0) {
    return (
      <div className={`aether-chat aether-chat--empty ds-scope${className ? ` ${className}` : ""}`}>
        {empty ?? <p>Ask the local DeepSeek runtime anything.</p>}
      </div>
    );
  }

  return (
    <div className={`aether-chat ds-scope${className ? ` ${className}` : ""}`} role="log" aria-live="polite">
      {messages.map((message) => (
        <article
          key={message.id}
          className={`aether-chat__message aether-chat__message--${message.role}`}
          aria-label={`${message.role} message`}
        >
          <header className="aether-chat__meta">
            <span className="aether-chat__role">{roleLabel(message.role)}</span>
            {message.model === undefined ? null : (
              <span className="aether-chat__model">{message.model}</span>
            )}
            {message.at === undefined ? null : <span className="aether-chat__time">{message.at}</span>}
          </header>
          <div className="aether-chat__body">
            {renderSegments(message.content)}
            {message.streaming === true ? (
              <span className="aether-chat__caret" aria-label="Streaming" />
            ) : null}
          </div>
        </article>
      ))}
    </div>
  );
}

function roleLabel(role: ChatMessage["role"]): string {
  switch (role) {
    case "user":
      return "You";
    case "assistant":
      return "DeepSeek";
    default:
      return "System";
  }
}

/** Render parsed segments. Exported for the docs demo and snapshot tests. */
export function renderSegments(content: string): ReactNode[] {
  return parseMarkdownLite(content).map((segment, index) => renderSegment(segment, index));
}

function renderSegment(segment: Segment, index: number): ReactNode {
  switch (segment.type) {
    case "code":
      return (
        <pre key={index} className="aether-code" data-language={segment.language ?? "text"}>
          <code>{segment.text}</code>
        </pre>
      );
    case "inlineCode":
      return (
        <code key={index} className="aether-inline-code">
          {segment.text}
        </code>
      );
    case "bold":
      return <strong key={index}>{segment.text}</strong>;
    case "italic":
      return <em key={index}>{segment.text}</em>;
    case "link":
      return (
        <a key={index} href={segment.href} target="_blank" rel="noreferrer noopener">
          {segment.text}
        </a>
      );
    default:
      return <span key={index}>{segment.text}</span>;
  }
}
