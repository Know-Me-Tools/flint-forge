import React, { useEffect, useRef } from 'react';

interface AgentChatMessage { id: string; role: 'user' | 'assistant' | 'tool'; content: string; timestamp?: string }
interface AgentChatProps { messages: AgentChatMessage[]; onSend?: (text: string) => void; loading?: boolean }
export function AgentChat({ messages, onSend, loading }: AgentChatProps): React.ReactElement {
  const [draft, setDraft] = React.useState('');
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!draft.trim() || !onSend) return;
    onSend(draft.trim());
    setDraft('');
  };

  return (
    <div data-flint-component="agent-chat" style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <ol
        data-flint-part="messages"
        aria-live="polite"
        aria-label="Chat messages"
        style={{ flex: 1, overflowY: 'auto', listStyle: 'none', padding: 'var(--flint-space-4)', margin: 0, display: 'flex', flexDirection: 'column', gap: 'var(--flint-space-4)' }}
      >
        {messages.map((msg) => (
          <li
            key={msg.id}
            data-flint-part="message"
            data-role={msg.role}
            style={{ display: 'flex', flexDirection: msg.role === 'user' ? 'row-reverse' : 'row', gap: 'var(--flint-space-2)', alignItems: 'flex-end' }}
          >
            <div
              data-flint-part="bubble"
              style={{
                padding: 'var(--flint-space-2) var(--flint-space-4)',
                borderRadius: 'var(--flint-radius-lg)',
                background: msg.role === 'user' ? 'var(--flint-color-primary)' : 'var(--flint-color-surface)',
                color: msg.role === 'user' ? 'white' : 'inherit',
                maxWidth: '80%',
                fontFamily: 'var(--flint-font-sans)',
                fontSize: 'var(--flint-text-base)',
                border: msg.role !== 'user' ? '1px solid var(--flint-color-border)' : 'none',
              }}
            >
              {msg.content}
            </div>
          </li>
        ))}
        {loading && (
          <li data-flint-part="typing" aria-label="Agent is typing" style={{ display: 'flex', gap: 'var(--flint-space-1)', padding: 'var(--flint-space-2)' }}>
            <span style={{ animation: 'flint-blink 1s infinite' }}>●</span>
            <span style={{ animation: 'flint-blink 1s 0.2s infinite' }}>●</span>
            <span style={{ animation: 'flint-blink 1s 0.4s infinite' }}>●</span>
          </li>
        )}
        <div ref={bottomRef} aria-hidden="true" />
      </ol>
      {onSend && (
        <form onSubmit={handleSubmit} data-flint-part="composer" style={{ display: 'flex', gap: 'var(--flint-space-2)', padding: 'var(--flint-space-4)', borderTop: '1px solid var(--flint-color-border)' }}>
          <input
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="Type a message…"
            aria-label="Type a message"
            data-flint-part="input"
            style={{ flex: 1, padding: 'var(--flint-space-2) var(--flint-space-4)', borderRadius: 'var(--flint-radius-full)', border: '1px solid var(--flint-color-border)', fontFamily: 'var(--flint-font-sans)' }}
          />
          <button type="submit" aria-label="Send" disabled={!draft.trim()} data-flint-part="send" style={{ padding: 'var(--flint-space-2) var(--flint-space-4)', borderRadius: 'var(--flint-radius-full)', background: 'var(--flint-color-primary)', color: 'white', border: 'none', cursor: 'pointer' }}>
            Send
          </button>
        </form>
      )}
    </div>
  );
}

interface ToolCallProps { name: string; status: 'pending' | 'running' | 'complete' | 'error'; args?: string; result?: string }
export function ToolCall({ name, status, args, result }: ToolCallProps): React.ReactElement {
  return (
    <div data-flint-component="tool-call" data-status={status} role="status" aria-label={`Tool call: ${name} (${status})`} style={{ padding: 'var(--flint-space-2) var(--flint-space-4)', borderLeft: '3px solid var(--flint-color-primary)', background: 'var(--flint-color-surface)', borderRadius: '0 var(--flint-radius-md) var(--flint-radius-md) 0', fontFamily: 'var(--flint-font-mono)', fontSize: 'var(--flint-text-sm)' }}>
      <div data-flint-part="name" style={{ fontWeight: 700 }}>{name} <span data-flint-part="status">[{status}]</span></div>
      {args && <pre data-flint-part="args" style={{ margin: '4px 0', overflow: 'auto' }}>{args}</pre>}
      {result && <pre data-flint-part="result" style={{ margin: '4px 0', overflow: 'auto', color: status === 'error' ? 'var(--flint-color-error)' : 'var(--flint-color-success)' }}>{result}</pre>}
    </div>
  );
}

interface StreamingTextProps { text: string; streaming?: boolean }
export function StreamingText({ text, streaming }: StreamingTextProps): React.ReactElement {
  return (
    <span data-flint-component="streaming-text" aria-live={streaming ? 'polite' : 'off'}>
      {text}
      {streaming && <span aria-hidden="true" data-flint-cursor style={{ borderRight: '2px solid currentColor', marginLeft: '1px', animation: 'flint-blink 1s infinite' }} />}
    </span>
  );
}

interface DecisionOption { id: string; label: string; description?: string }
interface DecisionProps { question: string; options: DecisionOption[]; onSelect?: (id: string) => void }
export function Decision({ question, options, onSelect }: DecisionProps): React.ReactElement {
  return (
    <div data-flint-component="decision" role="group" aria-label={question}>
      <p data-flint-part="question" style={{ fontWeight: 600, marginBottom: 'var(--flint-space-4)' }}>{question}</p>
      <div data-flint-part="options" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--flint-space-2)' }}>
        {options.map((opt) => (
          <button
            key={opt.id}
            onClick={onSelect ? () => onSelect(opt.id) : undefined}
            data-flint-part="option"
            style={{ textAlign: 'left', padding: 'var(--flint-space-4)', borderRadius: 'var(--flint-radius-md)', border: '1px solid var(--flint-color-border)', background: 'var(--flint-color-surface)', cursor: 'pointer', fontFamily: 'var(--flint-font-sans)' }}
          >
            <div style={{ fontWeight: 600 }}>{opt.label}</div>
            {opt.description && <div style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)' }}>{opt.description}</div>}
          </button>
        ))}
      </div>
    </div>
  );
}

interface ProgressLogEntry { id: string; message: string; level?: 'info' | 'warn' | 'error'; timestamp?: string }
interface ProgressLogProps { entries: ProgressLogEntry[]; title?: string }
export function ProgressLog({ entries, title }: ProgressLogProps): React.ReactElement {
  return (
    <div data-flint-component="progress-log" role="log" aria-label={title ?? 'Progress log'} aria-live="polite">
      {title && <h3 data-flint-part="title" style={{ margin: '0 0 var(--flint-space-2)', fontSize: 'var(--flint-text-base)' }}>{title}</h3>}
      <ol data-flint-part="entries" style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: '2px', fontFamily: 'var(--flint-font-mono)', fontSize: 'var(--flint-text-sm)' }}>
        {entries.map((entry) => (
          <li key={entry.id} data-flint-part="entry" data-level={entry.level ?? 'info'}>
            {entry.timestamp && <time>{entry.timestamp}</time>}{' '}{entry.message}
          </li>
        ))}
      </ol>
    </div>
  );
}

interface ArtifactProps { type: 'code' | 'text' | 'image' | 'file'; content: string; language?: string; filename?: string }
export function Artifact({ type, content, language, filename }: ArtifactProps): React.ReactElement {
  return (
    <figure data-flint-component="artifact" data-artifact-type={type} style={{ margin: 0 }}>
      {filename && <figcaption data-flint-part="filename" style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)', marginBottom: 'var(--flint-space-1)' }}>{filename}</figcaption>}
      {type === 'code' ? (
        <pre
          data-flint-part="code"
          data-language={language}
          style={{ padding: 'var(--flint-space-4)', background: 'oklch(14% 0 0)', color: 'oklch(90% 0 0)', borderRadius: 'var(--flint-radius-md)', overflowX: 'auto', fontFamily: 'var(--flint-font-mono)', fontSize: 'var(--flint-text-sm)', margin: 0 }}
        >
          <code>{content}</code>
        </pre>
      ) : (
        <div data-flint-part="content" style={{ padding: 'var(--flint-space-4)', background: 'var(--flint-color-surface)', border: '1px solid var(--flint-color-border)', borderRadius: 'var(--flint-radius-md)' }}>
          {content}
        </div>
      )}
    </figure>
  );
}
