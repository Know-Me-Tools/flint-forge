import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FlintProvider } from '../provider/FlintProvider';
import { registerBaseComponents, getRegistrySize } from '../registry/FlintRegistry';

registerBaseComponents();

describe('FlintProvider', () => {
  it('renders children without error given valid endpoint + applicationId', () => {
    render(
      <FlintProvider endpoint="https://api.test.com" applicationId="test-app-id" jwt="test-jwt">
        <div data-testid="child">hello</div>
      </FlintProvider>,
    );
    expect(screen.getByTestId('child')).toBeTruthy();
  });

  it('sets data-flint-app attribute on root element', () => {
    const { container } = render(
      <FlintProvider endpoint="https://api.test.com" applicationId="my-app" jwt="jwt">
        <span />
      </FlintProvider>,
    );
    expect(container.querySelector('[data-flint-app="my-app"]')).toBeTruthy();
  });

  it('applies override tokens as CSS custom properties', () => {
    const { container } = render(
      <FlintProvider
        endpoint="https://api.test.com"
        applicationId="app"
        jwt="jwt"
        tokens={{ '--flint-color-primary': '#ff0000' }}
      >
        <span />
      </FlintProvider>,
    );
    const root = container.querySelector('[data-flint-provider]') as HTMLElement;
    expect(root.style.getPropertyValue('--flint-color-primary')).toBe('#ff0000');
  });
});

describe('FlintRegistry', () => {
  it('registers at least 40 base components after registerBaseComponents()', () => {
    expect(getRegistrySize()).toBeGreaterThanOrEqual(40);
  });
});
