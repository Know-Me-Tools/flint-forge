import React from 'react';

interface Column<T> { field: keyof T; header: string; sortable?: boolean; render?: (val: T[keyof T], row: T) => React.ReactNode }
interface DataGridProps<T extends Record<string, unknown>> {
  columns: Column<T>[];
  data: T[];
  loading?: boolean;
  pageSize?: number;
  onRowClick?: (row: T) => void;
  rowKey?: keyof T;
}
export function DataGrid<T extends Record<string, unknown>>({ columns, data, loading, onRowClick, rowKey }: DataGridProps<T>): React.ReactElement {
  return (
    <div data-flint-component="data-grid" role="region" aria-busy={loading}>
      <table data-flint-part="table" style={{ width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={String(col.field)} data-flint-sortable={col.sortable} scope="col" style={{ textAlign: 'left', padding: 'var(--flint-space-2) var(--flint-space-4)', borderBottom: '2px solid var(--flint-color-border)', fontFamily: 'var(--flint-font-sans)' }}>
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.map((row, i) => (
            <tr
              key={rowKey ? String(row[rowKey]) : i}
              data-flint-part="row"
              onClick={onRowClick ? () => onRowClick(row) : undefined}
              tabIndex={onRowClick ? 0 : undefined}
              style={{ cursor: onRowClick ? 'pointer' : 'default' }}
            >
              {columns.map((col) => (
                <td key={String(col.field)} style={{ padding: 'var(--flint-space-2) var(--flint-space-4)', borderBottom: '1px solid var(--flint-color-border)' }}>
                  {col.render ? col.render(row[col.field], row) : String(row[col.field] ?? '')}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {loading && <div aria-live="polite" aria-label="Loading" data-flint-part="loading" style={{ padding: 'var(--flint-space-4)', textAlign: 'center' }}>Loading…</div>}
    </div>
  );
}

interface TableProps { headers: string[]; rows: React.ReactNode[][] }
export function Table({ headers, rows }: TableProps): React.ReactElement {
  return (
    <div data-flint-component="table" style={{ overflowX: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontFamily: 'var(--flint-font-sans)' }}>
        <thead>
          <tr>{headers.map((h, i) => <th key={i} scope="col" style={{ textAlign: 'left', padding: 'var(--flint-space-2) var(--flint-space-4)', borderBottom: '2px solid var(--flint-color-border)' }}>{h}</th>)}</tr>
        </thead>
        <tbody>
          {rows.map((row, ri) => (
            <tr key={ri}>{row.map((cell, ci) => <td key={ci} style={{ padding: 'var(--flint-space-2) var(--flint-space-4)', borderBottom: '1px solid var(--flint-color-border)' }}>{cell}</td>)}</tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

interface MetricProps { label: string; value: string | number; unit?: string; trend?: 'up' | 'down' | 'flat' }
export function Metric({ label, value, unit, trend }: MetricProps): React.ReactElement {
  return (
    <div data-flint-component="metric" role="figure" aria-label={label}>
      <dt data-flint-part="label" style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)' }}>{label}</dt>
      <dd data-flint-part="value" style={{ fontSize: 'var(--flint-text-xl)', fontWeight: 700, margin: 0 }}>
        {value}{unit && <span data-flint-part="unit" style={{ fontSize: 'var(--flint-text-base)', fontWeight: 400 }}> {unit}</span>}
        {trend && <span aria-label={`Trend: ${trend}`} data-flint-trend={trend}> {trend === 'up' ? '↑' : trend === 'down' ? '↓' : '→'}</span>}
      </dd>
    </div>
  );
}

interface BadgeProps { label: string; variant?: 'default' | 'success' | 'warning' | 'error' | 'info' }
export function Badge({ label, variant = 'default' }: BadgeProps): React.ReactElement {
  const colorMap: Record<string, string> = {
    default: 'var(--flint-color-border)',
    success: 'var(--flint-color-success)',
    error: 'var(--flint-color-error)',
    warning: 'oklch(75% 0.18 70)',
    info: 'var(--flint-color-primary)',
  };
  return (
    <span
      data-flint-component="badge"
      data-variant={variant}
      style={{ display: 'inline-block', padding: '2px var(--flint-space-2)', borderRadius: 'var(--flint-radius-full)', background: colorMap[variant], fontSize: 'var(--flint-text-sm)', fontFamily: 'var(--flint-font-sans)' }}
    >
      {label}
    </span>
  );
}

interface TimelineEvent { id: string; label: string; timestamp: string; description?: string }
interface TimelineProps { events: TimelineEvent[] }
export function Timeline({ events }: TimelineProps): React.ReactElement {
  return (
    <ol data-flint-component="timeline" style={{ listStyle: 'none', padding: 0, margin: 0 }}>
      {events.map((event) => (
        <li key={event.id} data-flint-part="event" style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: 'var(--flint-space-4)', padding: 'var(--flint-space-2) 0', alignItems: 'start' }}>
          <time dateTime={event.timestamp} data-flint-part="time" style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)', whiteSpace: 'nowrap' }}>{event.timestamp}</time>
          <div>
            <div data-flint-part="label" style={{ fontWeight: 600 }}>{event.label}</div>
            {event.description && <div data-flint-part="description" style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)' }}>{event.description}</div>}
          </div>
        </li>
      ))}
    </ol>
  );
}

interface KanbanColumn<T> { id: string; title: string; cards: T[] }
interface KanbanProps<T extends { id: string; title: string }> { columns: KanbanColumn<T>[]; renderCard?: (card: T) => React.ReactNode }
export function Kanban<T extends { id: string; title: string }>({ columns, renderCard }: KanbanProps<T>): React.ReactElement {
  return (
    <div data-flint-component="kanban" style={{ display: 'flex', gap: 'var(--flint-space-4)', overflowX: 'auto', alignItems: 'flex-start' }}>
      {columns.map((col) => (
        <div key={col.id} data-flint-part="column" data-column-id={col.id} style={{ minWidth: '240px', background: 'var(--flint-color-surface)', borderRadius: 'var(--flint-radius-md)', padding: 'var(--flint-space-4)' }} role="region" aria-label={col.title}>
          <h3 data-flint-part="column-title" style={{ margin: '0 0 var(--flint-space-4)', fontSize: 'var(--flint-text-base)' }}>{col.title}</h3>
          <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 'var(--flint-space-2)' }}>
            {col.cards.map((card) => (
              <li key={card.id} data-flint-part="card">
                {renderCard ? renderCard(card) : <div style={{ padding: 'var(--flint-space-2)', background: 'white', borderRadius: 'var(--flint-radius-sm)', boxShadow: '0 1px 4px color-mix(in oklch, black 8%, transparent)' }}>{card.title}</div>}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}

interface CalendarProps { year: number; month: number; events?: Array<{ date: string; label: string }>; onDateSelect?: (date: string) => void }
export function Calendar({ year, month, events = [], onDateSelect }: CalendarProps): React.ReactElement {
  const firstDay = new Date(year, month - 1, 1).getDay();
  const daysInMonth = new Date(year, month, 0).getDate();
  const cells = Array.from({ length: firstDay + daysInMonth }, (_, i) => i < firstDay ? null : i - firstDay + 1);
  const eventMap = new Map(events.map((e) => [e.date, e.label]));
  return (
    <div data-flint-component="calendar" role="grid" aria-label={`Calendar ${year}-${String(month).padStart(2, '0')}`}>
      <div data-flint-part="header" style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', fontWeight: 700, textAlign: 'center', fontSize: 'var(--flint-text-sm)' }}>
        {['Su','Mo','Tu','We','Th','Fr','Sa'].map((d) => <div key={d} role="columnheader" aria-label={d}>{d}</div>)}
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: '2px' }}>
        {cells.map((day, i) => (
          <div
            key={i}
            role={day ? 'gridcell' : 'presentation'}
            tabIndex={day ? 0 : undefined}
            aria-label={day ? `${year}-${String(month).padStart(2,'0')}-${String(day).padStart(2,'0')}` : undefined}
            onClick={day && onDateSelect ? () => onDateSelect(`${year}-${String(month).padStart(2,'0')}-${String(day).padStart(2,'0')}`) : undefined}
            style={{ minHeight: '40px', padding: 'var(--flint-space-1)', position: 'relative', cursor: day && onDateSelect ? 'pointer' : 'default' }}
            data-flint-part={day ? 'day' : 'empty'}
            data-has-event={day ? eventMap.has(`${year}-${String(month).padStart(2,'0')}-${String(day).padStart(2,'0')}`) : undefined}
          >
            {day ?? ''}
          </div>
        ))}
      </div>
    </div>
  );
}

interface ChartProps { type?: 'bar' | 'line'; data: Array<{ label: string; value: number }>; title?: string }
export function Chart({ type = 'bar', data, title }: ChartProps): React.ReactElement {
  const max = Math.max(...data.map((d) => d.value), 1);
  return (
    <figure data-flint-component="chart" data-chart-type={type} aria-label={title}>
      {title && <figcaption style={{ marginBottom: 'var(--flint-space-2)', fontWeight: 600 }}>{title}</figcaption>}
      <div data-flint-part="bars" style={{ display: 'flex', gap: 'var(--flint-space-2)', alignItems: 'flex-end', height: '160px' }}>
        {data.map((item, i) => (
          <div key={i} data-flint-part="bar-group" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', flex: 1, gap: 'var(--flint-space-1)' }}>
            <div
              data-flint-part="bar"
              role="img"
              aria-label={`${item.label}: ${item.value}`}
              style={{ width: '100%', background: 'var(--flint-color-primary)', borderRadius: 'var(--flint-radius-sm) var(--flint-radius-sm) 0 0', height: `${(item.value / max) * 140}px`, transition: `height var(--flint-duration-normal) var(--flint-ease-out-expo)` }}
            />
            <span style={{ fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-muted)', textAlign: 'center' }}>{item.label}</span>
          </div>
        ))}
      </div>
    </figure>
  );
}
