import React from 'react';

type Justify = 'start' | 'end' | 'center' | 'between' | 'around';
type Align = 'start' | 'end' | 'center' | 'stretch';

interface StackProps {
  direction?: 'horizontal' | 'vertical';
  gap?: number;
  justify?: Justify;
  align?: Align;
  wrap?: boolean;
  children?: React.ReactNode;
}
export function Stack({ direction = 'vertical', gap = 4, justify = 'start', align = 'stretch', wrap, children }: StackProps): React.ReactElement {
  return (
    <div
      data-flint-component="stack"
      data-direction={direction}
      style={{
        display: 'flex',
        flexDirection: direction === 'horizontal' ? 'row' : 'column',
        gap: `calc(var(--flint-space-1, 0.25rem) * ${gap})`,
        justifyContent: justify === 'between' ? 'space-between' : justify === 'around' ? 'space-around' : justify,
        alignItems: align,
        flexWrap: wrap ? 'wrap' : 'nowrap',
      }}
    >
      {children}
    </div>
  );
}

interface CardProps {
  title?: string;
  elevated?: boolean;
  children?: React.ReactNode;
}
export function Card({ title, elevated, children }: CardProps): React.ReactElement {
  return (
    <div
      data-flint-component="card"
      data-elevated={elevated}
      role="region"
      aria-label={title}
      style={{
        background: 'var(--flint-color-surface)',
        borderRadius: 'var(--flint-radius-md)',
        border: '1px solid var(--flint-color-border)',
        padding: 'var(--flint-space-4)',
        boxShadow: elevated ? '0 4px 24px color-mix(in oklch, currentColor 8%, transparent)' : undefined,
      }}
    >
      {title && <h2 data-flint-part="title" style={{ margin: 0, fontSize: 'var(--flint-text-lg)' }}>{title}</h2>}
      {children}
    </div>
  );
}

interface GridProps {
  columns?: number | string;
  gap?: number;
  children?: React.ReactNode;
}
export function Grid({ columns = 3, gap = 4, children }: GridProps): React.ReactElement {
  return (
    <div
      data-flint-component="grid"
      style={{
        display: 'grid',
        gridTemplateColumns: typeof columns === 'number' ? `repeat(${columns}, 1fr)` : columns,
        gap: `calc(var(--flint-space-1, 0.25rem) * ${gap})`,
      }}
    >
      {children}
    </div>
  );
}

interface SplitProps {
  ratio?: string;
  children?: React.ReactNode;
}
export function Split({ ratio = '1fr 1fr', children }: SplitProps): React.ReactElement {
  return (
    <div
      data-flint-component="split"
      style={{ display: 'grid', gridTemplateColumns: ratio, gap: 'var(--flint-space-4)' }}
    >
      {children}
    </div>
  );
}

interface TabItem { label: string; value: string; content: React.ReactNode }
interface TabsProps { items: TabItem[]; defaultValue?: string }
export function Tabs({ items, defaultValue }: TabsProps): React.ReactElement {
  const [active, setActive] = React.useState(defaultValue ?? items[0]?.value ?? '');
  return (
    <div data-flint-component="tabs">
      <div role="tablist" data-flint-part="tablist" style={{ display: 'flex', gap: 'var(--flint-space-2)', borderBottom: '1px solid var(--flint-color-border)' }}>
        {items.map((item) => (
          <button
            key={item.value}
            role="tab"
            aria-selected={active === item.value}
            aria-controls={`flint-tab-panel-${item.value}`}
            id={`flint-tab-${item.value}`}
            onClick={() => setActive(item.value)}
            data-flint-part="tab"
            style={{ background: 'none', border: 'none', padding: 'var(--flint-space-2) var(--flint-space-4)', cursor: 'pointer' }}
          >
            {item.label}
          </button>
        ))}
      </div>
      {items.map((item) => (
        <div
          key={item.value}
          role="tabpanel"
          id={`flint-tab-panel-${item.value}`}
          aria-labelledby={`flint-tab-${item.value}`}
          hidden={active !== item.value}
          data-flint-part="panel"
        >
          {item.content}
        </div>
      ))}
    </div>
  );
}

interface AccordionItem { label: string; value: string; content: React.ReactNode }
interface AccordionProps { items: AccordionItem[]; allowMultiple?: boolean }
export function Accordion({ items, allowMultiple }: AccordionProps): React.ReactElement {
  const [open, setOpen] = React.useState<Set<string>>(new Set());
  const toggle = (value: string) => {
    setOpen((prev) => {
      const next = new Set(allowMultiple ? prev : []);
      if (prev.has(value)) { next.delete(value); } else { next.add(value); }
      return next;
    });
  };
  return (
    <div data-flint-component="accordion">
      {items.map((item) => (
        <div key={item.value} data-flint-part="item">
          <button
            aria-expanded={open.has(item.value)}
            aria-controls={`flint-acc-${item.value}`}
            onClick={() => toggle(item.value)}
            data-flint-part="trigger"
            style={{ width: '100%', background: 'none', border: 'none', textAlign: 'left', padding: 'var(--flint-space-4)', cursor: 'pointer' }}
          >
            {item.label}
          </button>
          <div
            id={`flint-acc-${item.value}`}
            hidden={!open.has(item.value)}
            data-flint-part="content"
            style={{ padding: open.has(item.value) ? 'var(--flint-space-4)' : '0' }}
          >
            {item.content}
          </div>
        </div>
      ))}
    </div>
  );
}

interface ScrollProps { maxHeight?: string; children?: React.ReactNode }
export function Scroll({ maxHeight = '400px', children }: ScrollProps): React.ReactElement {
  return (
    <div
      data-flint-component="scroll"
      style={{ maxHeight, overflowY: 'auto', scrollbarWidth: 'thin' }}
      tabIndex={0}
      role="region"
      aria-label="Scrollable content"
    >
      {children}
    </div>
  );
}

interface ModalProps { open: boolean; onClose: () => void; title?: string; children?: React.ReactNode }
export function Modal({ open, onClose, title, children }: ModalProps): React.ReactElement | null {
  if (!open) return null;
  return (
    <div
      data-flint-component="modal"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      style={{ position: 'fixed', inset: 0, zIndex: 100, display: 'flex', alignItems: 'center', justifyContent: 'center' }}
    >
      <div
        data-flint-part="backdrop"
        onClick={onClose}
        style={{ position: 'absolute', inset: 0, background: 'color-mix(in oklch, black 40%, transparent)' }}
        aria-hidden="true"
      />
      <div
        data-flint-part="panel"
        style={{ position: 'relative', background: 'var(--flint-color-surface)', borderRadius: 'var(--flint-radius-lg)', padding: 'var(--flint-space-8)', minWidth: '320px', maxWidth: '90vw' }}
      >
        {title && <h2 data-flint-part="title" style={{ margin: '0 0 var(--flint-space-4)' }}>{title}</h2>}
        <button
          onClick={onClose}
          aria-label="Close dialog"
          data-flint-part="close"
          style={{ position: 'absolute', top: 'var(--flint-space-4)', right: 'var(--flint-space-4)', background: 'none', border: 'none', cursor: 'pointer' }}
        >
          ✕
        </button>
        {children}
      </div>
    </div>
  );
}

interface DrawerProps { open: boolean; onClose: () => void; side?: 'left' | 'right'; children?: React.ReactNode }
export function Drawer({ open, onClose, side = 'right', children }: DrawerProps): React.ReactElement | null {
  if (!open) return null;
  return (
    <div data-flint-component="drawer" role="dialog" aria-modal="true" style={{ position: 'fixed', inset: 0, zIndex: 100 }}>
      <div data-flint-part="backdrop" onClick={onClose} aria-hidden="true" style={{ position: 'absolute', inset: 0, background: 'color-mix(in oklch, black 30%, transparent)' }} />
      <div
        data-flint-part="panel"
        style={{
          position: 'absolute',
          top: 0,
          bottom: 0,
          [side]: 0,
          width: '360px',
          maxWidth: '90vw',
          background: 'var(--flint-color-surface)',
          padding: 'var(--flint-space-8)',
          overflowY: 'auto',
        }}
      >
        {children}
      </div>
    </div>
  );
}
