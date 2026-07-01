export const FLINT_TOKEN_PREFIX = '--flint-';

export const DEFAULT_TOKENS: Record<string, string> = {
  '--flint-color-primary': 'oklch(68% 0.21 250)',
  '--flint-color-surface': 'oklch(98% 0 0)',
  '--flint-color-text': 'oklch(18% 0 0)',
  '--flint-color-border': 'oklch(88% 0 0)',
  '--flint-color-accent': 'oklch(68% 0.21 250)',
  '--flint-color-muted': 'oklch(78% 0 0)',
  '--flint-color-error': 'oklch(55% 0.22 25)',
  '--flint-color-success': 'oklch(65% 0.17 145)',
  '--flint-text-base': 'clamp(1rem, 0.92rem + 0.4vw, 1.125rem)',
  '--flint-text-sm': '0.875rem',
  '--flint-text-lg': '1.25rem',
  '--flint-text-xl': '1.5rem',
  '--flint-text-hero': 'clamp(3rem, 1rem + 7vw, 8rem)',
  '--flint-font-sans': 'system-ui, sans-serif',
  '--flint-font-mono': 'ui-monospace, monospace',
  '--flint-space-1': '0.25rem',
  '--flint-space-2': '0.5rem',
  '--flint-space-4': '1rem',
  '--flint-space-6': '1.5rem',
  '--flint-space-8': '2rem',
  '--flint-space-section': 'clamp(4rem, 3rem + 5vw, 10rem)',
  '--flint-radius-sm': '0.25rem',
  '--flint-radius-md': '0.5rem',
  '--flint-radius-lg': '1rem',
  '--flint-radius-full': '9999px',
  '--flint-duration-fast': '150ms',
  '--flint-duration-normal': '300ms',
  '--flint-duration-slow': '500ms',
  '--flint-ease-out-expo': 'cubic-bezier(0.16, 1, 0.3, 1)',
};

export function injectTokens(
  element: HTMLElement,
  tokens: Record<string, string>,
): void {
  const merged = { ...DEFAULT_TOKENS, ...tokens };
  for (const [key, value] of Object.entries(merged)) {
    element.style.setProperty(key, value);
  }
}

export function removeTokens(element: HTMLElement): void {
  for (const key of Object.keys(DEFAULT_TOKENS)) {
    element.style.removeProperty(key);
  }
}
