import React from 'react';

const inputStyle: React.CSSProperties = {
  display: 'block',
  width: '100%',
  padding: 'var(--flint-space-2) var(--flint-space-4)',
  borderRadius: 'var(--flint-radius-md)',
  border: '1px solid var(--flint-color-border)',
  fontFamily: 'var(--flint-font-sans)',
  fontSize: 'var(--flint-text-base)',
  background: 'var(--flint-color-surface)',
  boxSizing: 'border-box',
};

interface TextFieldProps {
  label: string;
  name: string;
  value?: string;
  onChange?: (val: string) => void;
  placeholder?: string;
  type?: string;
  required?: boolean;
  error?: string;
}
export function TextField({ label, name, value, onChange, placeholder, type = 'text', required, error }: TextFieldProps): React.ReactElement {
  const id = `flint-field-${name}`;
  return (
    <div data-flint-component="text-field">
      <label htmlFor={id} data-flint-part="label" style={{ display: 'block', marginBottom: 'var(--flint-space-1)', fontFamily: 'var(--flint-font-sans)', fontWeight: 500 }}>
        {label}{required && <span aria-hidden="true" style={{ color: 'var(--flint-color-error)' }}> *</span>}
      </label>
      <input
        id={id}
        name={name}
        type={type}
        value={value}
        placeholder={placeholder}
        required={required}
        aria-invalid={!!error}
        aria-describedby={error ? `${id}-error` : undefined}
        onChange={onChange ? (e) => onChange(e.target.value) : undefined}
        data-flint-part="input"
        style={{ ...inputStyle, borderColor: error ? 'var(--flint-color-error)' : 'var(--flint-color-border)' }}
      />
      {error && <p id={`${id}-error`} role="alert" data-flint-part="error" style={{ margin: 'var(--flint-space-1) 0 0', fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-error)' }}>{error}</p>}
    </div>
  );
}

interface SelectOption { label: string; value: string }
interface SelectProps {
  label: string;
  name: string;
  options: SelectOption[];
  value?: string;
  onChange?: (val: string) => void;
  required?: boolean;
  error?: string;
}
export function Select({ label, name, options, value, onChange, required, error }: SelectProps): React.ReactElement {
  const id = `flint-select-${name}`;
  return (
    <div data-flint-component="select">
      <label htmlFor={id} data-flint-part="label" style={{ display: 'block', marginBottom: 'var(--flint-space-1)', fontFamily: 'var(--flint-font-sans)', fontWeight: 500 }}>
        {label}{required && <span aria-hidden="true" style={{ color: 'var(--flint-color-error)' }}> *</span>}
      </label>
      <select
        id={id}
        name={name}
        value={value}
        required={required}
        aria-invalid={!!error}
        onChange={onChange ? (e) => onChange(e.target.value) : undefined}
        data-flint-part="select"
        style={{ ...inputStyle, appearance: 'auto' }}
      >
        <option value="">Select…</option>
        {options.map((opt) => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
      </select>
      {error && <p role="alert" data-flint-part="error" style={{ margin: 'var(--flint-space-1) 0 0', fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-error)' }}>{error}</p>}
    </div>
  );
}

interface DatePickerProps { label: string; name: string; value?: string; onChange?: (val: string) => void; required?: boolean }
export function DatePicker({ label, name, value, onChange, required }: DatePickerProps): React.ReactElement {
  return <TextField label={label} name={name} value={value} onChange={onChange} type="date" required={required} />;
}

interface SearchProps { value?: string; onChange?: (val: string) => void; placeholder?: string; onSearch?: (val: string) => void }
export function Search({ value, onChange, placeholder = 'Search…', onSearch }: SearchProps): React.ReactElement {
  return (
    <div data-flint-component="search" role="search" style={{ position: 'relative' }}>
      <span aria-hidden="true" data-flint-part="icon" style={{ position: 'absolute', left: 'var(--flint-space-2)', top: '50%', transform: 'translateY(-50%)' }}>⌕</span>
      <input
        type="search"
        value={value}
        placeholder={placeholder}
        onChange={onChange ? (e) => onChange(e.target.value) : undefined}
        onKeyDown={onSearch ? (e) => { if (e.key === 'Enter') onSearch(e.currentTarget.value); } : undefined}
        data-flint-part="input"
        aria-label={placeholder}
        style={{ ...inputStyle, paddingLeft: 'calc(var(--flint-space-4) + 1em)' }}
      />
    </div>
  );
}

interface FileUploadProps { label: string; name: string; accept?: string; multiple?: boolean; onChange?: (files: FileList | null) => void }
export function FileUpload({ label, name, accept, multiple, onChange }: FileUploadProps): React.ReactElement {
  const id = `flint-file-${name}`;
  return (
    <div data-flint-component="file-upload">
      <label htmlFor={id} data-flint-part="label" style={{ display: 'block', marginBottom: 'var(--flint-space-1)', fontFamily: 'var(--flint-font-sans)', fontWeight: 500 }}>{label}</label>
      <label
        htmlFor={id}
        data-flint-part="dropzone"
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 'var(--flint-space-8)', border: '2px dashed var(--flint-color-border)', borderRadius: 'var(--flint-radius-md)', cursor: 'pointer', gap: 'var(--flint-space-2)' }}
      >
        <span aria-hidden="true">⬆</span> Drop files or click to upload
      </label>
      <input id={id} name={name} type="file" accept={accept} multiple={multiple} onChange={onChange ? (e) => onChange(e.target.files) : undefined} style={{ position: 'absolute', width: '1px', height: '1px', opacity: 0 }} />
    </div>
  );
}

interface JsonEditorProps { label: string; name: string; value?: string; onChange?: (val: string) => void; error?: string }
export function JsonEditor({ label, name, value, onChange, error }: JsonEditorProps): React.ReactElement {
  const id = `flint-json-${name}`;
  return (
    <div data-flint-component="json-editor">
      <label htmlFor={id} data-flint-part="label" style={{ display: 'block', marginBottom: 'var(--flint-space-1)', fontFamily: 'var(--flint-font-sans)', fontWeight: 500 }}>{label}</label>
      <textarea
        id={id}
        name={name}
        value={value}
        onChange={onChange ? (e) => onChange(e.target.value) : undefined}
        aria-invalid={!!error}
        spellCheck={false}
        data-flint-part="editor"
        style={{ ...inputStyle, fontFamily: 'var(--flint-font-mono)', minHeight: '120px', resize: 'vertical' }}
      />
      {error && <p role="alert" data-flint-part="error" style={{ margin: 'var(--flint-space-1) 0 0', fontSize: 'var(--flint-text-sm)', color: 'var(--flint-color-error)' }}>{error}</p>}
    </div>
  );
}

interface RichEditorProps { label: string; name: string; value?: string; onChange?: (val: string) => void }
export function RichEditor({ label, name, value, onChange }: RichEditorProps): React.ReactElement {
  const id = `flint-rich-${name}`;
  return (
    <div data-flint-component="rich-editor">
      <label htmlFor={id} data-flint-part="label" style={{ display: 'block', marginBottom: 'var(--flint-space-1)', fontFamily: 'var(--flint-font-sans)', fontWeight: 500 }}>{label}</label>
      <textarea
        id={id}
        name={name}
        value={value}
        onChange={onChange ? (e) => onChange(e.target.value) : undefined}
        data-flint-part="editor"
        style={{ ...inputStyle, minHeight: '200px', resize: 'vertical' }}
      />
    </div>
  );
}

interface FormField { name: string; label: string; type?: 'text' | 'email' | 'password' | 'number' | 'select' | 'date' | 'json' | 'file'; options?: Array<{ label: string; value: string }>; required?: boolean }
interface FormProps { fields: FormField[]; onSubmit: (data: Record<string, string>) => void; submitLabel?: string }
export function Form({ fields, onSubmit, submitLabel = 'Submit' }: FormProps): React.ReactElement {
  const [values, setValues] = React.useState<Record<string, string>>({});
  const handleSubmit = (e: React.FormEvent) => { e.preventDefault(); onSubmit(values); };
  const setValue = (name: string) => (val: string) => setValues((prev) => ({ ...prev, [name]: val }));
  return (
    <form data-flint-component="form" onSubmit={handleSubmit} noValidate style={{ display: 'flex', flexDirection: 'column', gap: 'var(--flint-space-4)' }}>
      {fields.map((field) => {
        if (field.type === 'select') {
          return <Select key={field.name} label={field.label} name={field.name} options={field.options ?? []} value={values[field.name]} onChange={setValue(field.name)} required={field.required} />;
        }
        if (field.type === 'date') return <DatePicker key={field.name} label={field.label} name={field.name} value={values[field.name]} onChange={setValue(field.name)} required={field.required} />;
        if (field.type === 'json') return <JsonEditor key={field.name} label={field.label} name={field.name} value={values[field.name]} onChange={setValue(field.name)} />;
        return <TextField key={field.name} label={field.label} name={field.name} type={field.type ?? 'text'} value={values[field.name]} onChange={setValue(field.name)} required={field.required} />;
      })}
      <button type="submit" data-flint-part="submit" style={{ alignSelf: 'flex-end', padding: 'var(--flint-space-2) var(--flint-space-4)', background: 'var(--flint-color-primary)', color: 'white', border: 'none', borderRadius: 'var(--flint-radius-md)', cursor: 'pointer', fontFamily: 'var(--flint-font-sans)', fontSize: 'var(--flint-text-base)' }}>
        {submitLabel}
      </button>
    </form>
  );
}
