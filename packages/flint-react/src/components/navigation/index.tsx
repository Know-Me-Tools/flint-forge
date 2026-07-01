import React, { useRef, useEffect } from 'react';

interface NavItem { label: string; href?: string; onClick?: () => void; icon?: React.ReactNode; active?: boolean; children?: NavItem[] }
interface NavMenuProps { items: NavItem[]; orientation?: 'horizontal' | 'vertical'; ariaLabel?: string }
export function NavMenu({ items, orientation = 'horizontal', ariaLabel = 'Main navigation' }: NavMenuProps): React.ReactElement {
  const renderItem = (item: NavItem, depth = 0): React.ReactElement => (
    <li key={item.label} data-flint-part="item" data-depth={depth}>
      {item.href ? (
        <a href={item.href} aria-current={item.active ? 'page' : undefined} data-flint-part="link" style={{ display: 'flex', alignItems: 'center', gap: 'var(--flint-space-2)', padding: 'var(--flint-space-2) var(--flint-space-4)', textDecoration: 'none', color: 'inherit', fontFamily: 'var(--flint-font-sans)' }}>
          {item.icon}{item.label}
        </a>
      ) : (
        <button onClick={item.onClick} data-flint-part="button" style={{ display: 'flex', alignItems: 'center', gap: 'var(--flint-space-2)', padding: 'var(--flint-space-2) var(--flint-space-4)', background: 'none', border: 'none', cursor: 'pointer', color: 'inherit', fontFamily: 'var(--flint-font-sans)', width: '100%', textAlign: 'left' }}>
          {item.icon}{item.label}
        </button>
      )}
      {item.children && item.children.length > 0 && (
        <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>{item.children.map((child) => renderItem(child, depth + 1))}</ul>
      )}
    </li>
  );
  return (
    <nav data-flint-component="nav-menu" aria-label={ariaLabel}>
      <ul data-flint-part="list" style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: orientation === 'horizontal' ? 'row' : 'column', gap: 0 }}>
        {items.map((item) => renderItem(item))}
      </ul>
    </nav>
  );
}

interface CommandPaletteProps { open: boolean; onClose: () => void; commands: Array<{ id: string; label: string; group?: string; onSelect: () => void; shortcut?: string }>; placeholder?: string }
export function CommandPalette({ open, onClose, commands, placeholder = 'Search commands…' }: CommandPaletteProps): React.ReactElement | null {
  const [query, setQuery] = React.useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) { setQuery(''); setTimeout(() => inputRef.current?.focus(), 0); }
  }, [open]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onClose]);

  if (!open) return null;

  const filtered = commands.filter((c) => c.label.toLowerCase().includes(query.toLowerCase()));
  const groups = Array.from(new Set(filtered.map((c) => c.group ?? '')));

  return (
    <div data-flint-component="command-palette" role="dialog" aria-modal="true" aria-label="Command palette" style={{ position: 'fixed', inset: 0, zIndex: 200, display: 'flex', alignItems: 'flex-start', justifyContent: 'center', paddingTop: '10vh' }}>
      <div data-flint-part="backdrop" onClick={onClose} aria-hidden="true" style={{ position: 'absolute', inset: 0, background: 'color-mix(in oklch, black 40%, transparent)' }} />
      <div data-flint-part="panel" style={{ position: 'relative', width: '560px', maxWidth: '90vw', background: 'var(--flint-color-surface)', borderRadius: 'var(--flint-radius-lg)', overflow: 'hidden', boxShadow: '0 24px 64px color-mix(in oklch, black 24%, transparent)' }}>
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={placeholder}
          aria-label={placeholder}
          data-flint-part="input"
          style={{ width: '100%', padding: 'var(--flint-space-4)', border: 'none', borderBottom: '1px solid var(--flint-color-border)', fontFamily: 'var(--flint-font-sans)', fontSize: 'var(--flint-text-base)', background: 'transparent', boxSizing: 'border-box', outline: 'none' }}
        />
        <ul role="listbox" data-flint-part="results" style={{ listStyle: 'none', padding: 0, margin: 0, maxHeight: '400px', overflowY: 'auto' }}>
          {groups.map((group) => (
            <React.Fragment key={group}>
              {group && <li role="presentation" data-flint-part="group" style={{ padding: 'var(--flint-space-2) var(--flint-space-4)', fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)', fontWeight: 600 }}>{group}</li>}
              {filtered.filter((c) => (c.group ?? '') === group).map((cmd) => (
                <li key={cmd.id} role="option" aria-selected={false}>
                  <button
                    onClick={() => { cmd.onSelect(); onClose(); }}
                    data-flint-part="command"
                    style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%', padding: 'var(--flint-space-2) var(--flint-space-4)', background: 'none', border: 'none', cursor: 'pointer', fontFamily: 'var(--flint-font-sans)', textAlign: 'left' }}
                  >
                    {cmd.label}
                    {cmd.shortcut && <kbd data-flint-part="shortcut" style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)', padding: '2px 6px', border: '1px solid var(--flint-color-border)', borderRadius: 'var(--flint-radius-sm)' }}>{cmd.shortcut}</kbd>}
                  </button>
                </li>
              ))}
            </React.Fragment>
          ))}
          {filtered.length === 0 && <li data-flint-part="empty" style={{ padding: 'var(--flint-space-4)', textAlign: 'center', color: 'var(--flint-color-muted)' }}>No results</li>}
        </ul>
      </div>
    </div>
  );
}

interface FilterBarFilter { id: string; label: string; type: 'text' | 'select' | 'date'; options?: Array<{ label: string; value: string }> }
interface FilterBarProps { filters: FilterBarFilter[]; values: Record<string, string>; onChange: (id: string, val: string) => void; onReset?: () => void }
export function FilterBar({ filters, values, onChange, onReset }: FilterBarProps): React.ReactElement {
  return (
    <div data-flint-component="filter-bar" role="group" aria-label="Filters" style={{ display: 'flex', gap: 'var(--flint-space-4)', flexWrap: 'wrap', alignItems: 'flex-end' }}>
      {filters.map((filter) => (
        <div key={filter.id} data-flint-part="filter" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--flint-space-1)' }}>
          <label htmlFor={`flint-filter-${filter.id}`} style={{ fontSize: 'var(--flint-text-sm)', fontWeight: 500, fontFamily: 'var(--flint-font-sans)' }}>{filter.label}</label>
          {filter.type === 'select' ? (
            <select id={`flint-filter-${filter.id}`} value={values[filter.id] ?? ''} onChange={(e) => onChange(filter.id, e.target.value)} style={{ padding: 'var(--flint-space-1) var(--flint-space-2)', borderRadius: 'var(--flint-radius-md)', border: '1px solid var(--flint-color-border)', fontFamily: 'var(--flint-font-sans)' }}>
              <option value="">All</option>
              {filter.options?.map((opt) => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
            </select>
          ) : (
            <input id={`flint-filter-${filter.id}`} type={filter.type} value={values[filter.id] ?? ''} onChange={(e) => onChange(filter.id, e.target.value)} style={{ padding: 'var(--flint-space-1) var(--flint-space-2)', borderRadius: 'var(--flint-radius-md)', border: '1px solid var(--flint-color-border)', fontFamily: 'var(--flint-font-sans)' }} />
          )}
        </div>
      ))}
      {onReset && <button onClick={onReset} data-flint-part="reset" style={{ padding: 'var(--flint-space-1) var(--flint-space-4)', borderRadius: 'var(--flint-radius-md)', border: '1px solid var(--flint-color-border)', background: 'none', cursor: 'pointer', fontFamily: 'var(--flint-font-sans)', alignSelf: 'flex-end' }}>Reset</button>}
    </div>
  );
}

interface BreadcrumbItem { label: string; href?: string }
interface BreadcrumbProps { items: BreadcrumbItem[] }
export function Breadcrumb({ items }: BreadcrumbProps): React.ReactElement {
  return (
    <nav data-flint-component="breadcrumb" aria-label="Breadcrumb">
      <ol style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', gap: 'var(--flint-space-2)', alignItems: 'center', flexWrap: 'wrap', fontFamily: 'var(--flint-font-sans)', fontSize: 'var(--flint-text-sm)' }}>
        {items.map((item, i) => (
          <React.Fragment key={i}>
            {i > 0 && <li aria-hidden="true" data-flint-separator>/</li>}
            <li aria-current={i === items.length - 1 ? 'page' : undefined}>
              {item.href && i < items.length - 1 ? (
                <a href={item.href} style={{ color: 'var(--flint-color-primary)', textDecoration: 'none' }}>{item.label}</a>
              ) : (
                <span>{item.label}</span>
              )}
            </li>
          </React.Fragment>
        ))}
      </ol>
    </nav>
  );
}
