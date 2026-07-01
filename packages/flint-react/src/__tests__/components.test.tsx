import React from 'react';
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';

import { Button } from '../components/action';
import { DataGrid } from '../components/data-display';
import { TextField, Form } from '../components/input';
import { AgentChat, StreamingText } from '../components/agent';
import { Breadcrumb } from '../components/navigation';
import { Stack, Card } from '../components/layout';

describe('Button', () => {
  it('renders with primary variant by default', () => {
    render(<Button>Submit</Button>);
    expect(screen.getByText('Submit')).toBeTruthy();
  });

  it('shows loading spinner when loading=true', () => {
    render(<Button loading>Submit</Button>);
    expect(screen.getByText('Submit').closest('button')).toHaveAttribute('aria-busy', 'true');
  });

  it('is disabled when disabled=true', () => {
    const btn = render(<Button disabled>Click</Button>).container.querySelector('button');
    expect(btn?.disabled).toBe(true);
  });
});

describe('DataGrid', () => {
  const cols = [
    { field: 'name' as const, header: 'Name' },
    { field: 'age' as const, header: 'Age' },
  ];
  const data = [{ name: 'Alice', age: '30' }, { name: 'Bob', age: '25' }];

  it('renders column headers', () => {
    render(<DataGrid columns={cols} data={data} />);
    expect(screen.getByText('Name')).toBeTruthy();
    expect(screen.getByText('Age')).toBeTruthy();
  });

  it('renders all rows', () => {
    render(<DataGrid columns={cols} data={data} />);
    expect(screen.getByText('Alice')).toBeTruthy();
    expect(screen.getByText('Bob')).toBeTruthy();
  });

  it('shows loading state when loading=true', () => {
    render(<DataGrid columns={cols} data={[]} loading />);
    expect(screen.getByText('Loading…')).toBeTruthy();
  });
});

describe('TextField', () => {
  it('renders label and input', () => {
    render(<TextField label="Email" name="email" />);
    expect(screen.getByLabelText('Email')).toBeTruthy();
  });

  it('shows error message with aria-invalid', () => {
    render(<TextField label="Email" name="email" error="Required" />);
    const input = screen.getByLabelText('Email');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByText('Required')).toBeTruthy();
  });
});

describe('StreamingText', () => {
  it('renders text content', () => {
    render(<StreamingText text="Hello world" />);
    expect(screen.getByText(/Hello world/)).toBeTruthy();
  });

  it('shows cursor when streaming=true', () => {
    const { container } = render(<StreamingText text="Hello" streaming />);
    expect(container.querySelector('[data-flint-cursor]')).toBeTruthy();
  });
});

describe('Breadcrumb', () => {
  it('renders all items with separators', () => {
    render(<Breadcrumb items={[{ label: 'Home', href: '/' }, { label: 'Products', href: '/products' }, { label: 'Widget' }]} />);
    expect(screen.getByText('Home')).toBeTruthy();
    expect(screen.getByText('Products')).toBeTruthy();
    expect(screen.getByText('Widget')).toBeTruthy();
  });

  it('marks the last item as current page', () => {
    render(<Breadcrumb items={[{ label: 'Home' }, { label: 'Current' }]} />);
    const items = screen.getAllByRole('listitem');
    const lastNonSeparator = items.filter((li) => !li.getAttribute('aria-hidden')).pop();
    expect(lastNonSeparator).toHaveAttribute('aria-current', 'page');
  });
});

describe('Stack', () => {
  it('renders children in a flex container', () => {
    const { container } = render(<Stack><span>A</span><span>B</span></Stack>);
    const div = container.querySelector('[data-flint-component="stack"]') as HTMLElement;
    expect(div.style.display).toBe('flex');
  });
});

describe('AgentChat', () => {
  it('renders messages', () => {
    const msgs = [{ id: '1', role: 'user' as const, content: 'Hello' }, { id: '2', role: 'assistant' as const, content: 'Hi there' }];
    render(<AgentChat messages={msgs} />);
    expect(screen.getByText('Hello')).toBeTruthy();
    expect(screen.getByText('Hi there')).toBeTruthy();
  });
});
