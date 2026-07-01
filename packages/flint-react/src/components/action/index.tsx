import React from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'destructive';
type ButtonSize = 'sm' | 'md' | 'lg';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  children?: React.ReactNode;
}
export function Button({ variant = 'primary', size = 'md', loading, children, disabled, ...rest }: ButtonProps): React.ReactElement {
  return (
    <button
      data-flint-component="button"
      data-variant={variant}
      data-size={size}
      aria-busy={loading}
      disabled={disabled || loading}
      {...rest}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 'var(--flint-space-2)',
        padding: size === 'sm' ? 'var(--flint-space-1) var(--flint-space-2)' : size === 'lg' ? 'var(--flint-space-4) var(--flint-space-8)' : 'var(--flint-space-2) var(--flint-space-4)',
        borderRadius: 'var(--flint-radius-md)',
        border: 'none',
        cursor: disabled || loading ? 'not-allowed' : 'pointer',
        opacity: disabled || loading ? 0.6 : 1,
        background: variant === 'primary' ? 'var(--flint-color-primary)' : variant === 'destructive' ? 'var(--flint-color-error)' : 'transparent',
        color: variant === 'ghost' || variant === 'secondary' ? 'inherit' : 'white',
        fontFamily: 'var(--flint-font-sans)',
        fontSize: 'var(--flint-text-base)',
        transition: `opacity var(--flint-duration-fast)`,
        ...rest.style,
      }}
    >
      {loading && <span aria-hidden="true" data-flint-spinner>⟳</span>}
      {children}
    </button>
  );
}

interface ConfirmProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  cancelLabel?: string;
}
export function Confirm({ message, onConfirm, onCancel, confirmLabel = 'Confirm', cancelLabel = 'Cancel' }: ConfirmProps): React.ReactElement {
  return (
    <div data-flint-component="confirm" role="alertdialog" aria-label={message}>
      <p data-flint-part="message">{message}</p>
      <div data-flint-part="actions" style={{ display: 'flex', gap: 'var(--flint-space-2)', justifyContent: 'flex-end' }}>
        <Button variant="secondary" onClick={onCancel}>{cancelLabel}</Button>
        <Button variant="destructive" onClick={onConfirm}>{confirmLabel}</Button>
      </div>
    </div>
  );
}

interface WizardStep { title: string; content: React.ReactNode }
interface WizardProps { steps: WizardStep[]; onComplete?: () => void }
export function Wizard({ steps, onComplete }: WizardProps): React.ReactElement {
  const [current, setCurrent] = React.useState(0);
  const isLast = current === steps.length - 1;
  return (
    <div data-flint-component="wizard">
      <ol data-flint-part="steps" aria-label="Wizard steps" style={{ display: 'flex', gap: 'var(--flint-space-4)', listStyle: 'none', padding: 0 }}>
        {steps.map((step, i) => (
          <li key={i} aria-current={i === current ? 'step' : undefined} data-flint-step-state={i < current ? 'complete' : i === current ? 'current' : 'pending'}>
            {step.title}
          </li>
        ))}
      </ol>
      <div data-flint-part="content" style={{ padding: 'var(--flint-space-4) 0' }}>
        {steps[current]?.content}
      </div>
      <div data-flint-part="actions" style={{ display: 'flex', gap: 'var(--flint-space-2)', justifyContent: 'flex-end' }}>
        {current > 0 && <Button variant="secondary" onClick={() => setCurrent((n) => n - 1)}>Back</Button>}
        {!isLast && <Button onClick={() => setCurrent((n) => n + 1)}>Next</Button>}
        {isLast && <Button onClick={onComplete}>Complete</Button>}
      </div>
    </div>
  );
}

interface BulkActionProps { selectedCount: number; actions: Array<{ label: string; onClick: () => void; destructive?: boolean }> }
export function BulkAction({ selectedCount, actions }: BulkActionProps): React.ReactElement | null {
  if (selectedCount === 0) return null;
  return (
    <div data-flint-component="bulk-action" role="toolbar" aria-label={`${selectedCount} items selected`} style={{ display: 'flex', alignItems: 'center', gap: 'var(--flint-space-4)', padding: 'var(--flint-space-2) var(--flint-space-4)', background: 'var(--flint-color-primary)', borderRadius: 'var(--flint-radius-md)' }}>
      <span data-flint-part="count" style={{ color: 'white', fontSize: 'var(--flint-text-sm)' }}>{selectedCount} selected</span>
      {actions.map((action, i) => (
        <Button key={i} variant={action.destructive ? 'destructive' : 'ghost'} onClick={action.onClick} size="sm">{action.label}</Button>
      ))}
    </div>
  );
}

interface ActionBarProps { actions: Array<{ label: string; icon?: React.ReactNode; onClick: () => void; disabled?: boolean }> }
export function ActionBar({ actions }: ActionBarProps): React.ReactElement {
  return (
    <div data-flint-component="action-bar" role="toolbar" style={{ display: 'flex', gap: 'var(--flint-space-2)', alignItems: 'center', padding: 'var(--flint-space-2)' }}>
      {actions.map((action, i) => (
        <Button key={i} variant="ghost" onClick={action.onClick} disabled={action.disabled} size="sm">
          {action.icon}
          {action.label}
        </Button>
      ))}
    </div>
  );
}
