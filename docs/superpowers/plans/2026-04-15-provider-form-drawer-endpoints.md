# Provider Form Drawer & Dynamic Endpoints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert provider create/edit forms from Modal to Drawer, and replace fixed endpoint fields with dynamic key-value pairs using dropdown presets.

**Architecture:** Modify Providers.tsx to use Drawer instead of Modal, create an EndpointsEditor component for dynamic key-value endpoint management with protocol dropdown presets.

**Tech Stack:** React (TypeScript), DaisyUI, existing Drawer component

---

### Task 1: Create EndpointsEditor Component

**Files:**
- Create: `web/src/components/ui/EndpointsEditor.tsx`

- [ ] **Step 1: Write the test file**

```tsx
// web/src/components/ui/EndpointsEditor.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { EndpointsEditor } from './EndpointsEditor';

describe('EndpointsEditor', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    mockOnChange.mockClear();
  });

  it('renders empty state with add button when no endpoints', () => {
    render(<EndpointsEditor value={{}} onChange={mockOnChange} />);
    expect(screen.getByText('No endpoints configured')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /add endpoint/i })).toBeInTheDocument();
  });

  it('adds new endpoint row when add button clicked', () => {
    render(<EndpointsEditor value={{}} onChange={mockOnChange} />);
    fireEvent.click(screen.getByRole('button', { name: /add endpoint/i }));
    expect(mockOnChange).toHaveBeenCalledWith({ openai: '' });
  });

  it('displays existing endpoints', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    expect(screen.getByDisplayValue('https://api.openai.com/v1')).toBeInTheDocument();
  });

  it('removes endpoint when delete button clicked', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    fireEvent.click(screen.getByRole('button', { name: /remove/i }));
    expect(mockOnChange).toHaveBeenCalledWith({});
  });

  it('updates endpoint URL when changed', () => {
    render(<EndpointsEditor value={{ openai: 'https://api.openai.com/v1' }} onChange={mockOnChange} />);
    const input = screen.getByDisplayValue('https://api.openai.com/v1');
    fireEvent.change(input, { target: { value: 'https://new.url.com' } });
    expect(mockOnChange).toHaveBeenCalledWith({ openai: 'https://new.url.com' });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- --run web/src/components/ui/EndpointsEditor.test.tsx`
Expected: FAIL - file does not exist

- [ ] **Step 3: Write the implementation**

```tsx
// web/src/components/ui/EndpointsEditor.tsx
import { useState } from 'react';
import { Plus, Trash2 } from 'lucide-react';
import { Button } from './Button';

export interface EndpointsEditorProps {
  value: Record<string, string>;
  onChange: (value: Record<string, string>) => void;
}

const PROTOCOL_OPTIONS = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'azure', label: 'Azure' },
  { value: 'google', label: 'Google' },
  { value: 'custom', label: 'Custom' },
];

export function EndpointsEditor({ value, onChange }: EndpointsEditorProps) {
  const entries = Object.entries(value);

  const handleAdd = () => {
    const newValue = { ...value, openai: '' };
    onChange(newValue);
  };

  const handleRemove = (key: string) => {
    const newValue = { ...value };
    delete newValue[key];
    onChange(newValue);
  };

  const handleProtocolChange = (oldKey: string, newKey: string) => {
    const url = value[oldKey];
    const newValue = { ...value };
    delete newValue[oldKey];
    if (newKey && !newValue[newKey]) {
      newValue[newKey] = url;
    }
    onChange(newValue);
  };

  const handleUrlChange = (key: string, url: string) => {
    onChange({ ...value, [key]: url });
  };

  return (
    <div className="space-y-3">
      {entries.length === 0 ? (
        <div className="text-center py-6 text-base-content/50">
          <p className="text-sm mb-3">No endpoints configured</p>
          <Button variant="outline" size="sm" icon={<Plus className="h-4 w-4" />} onClick={handleAdd}>
            Add Endpoint
          </Button>
        </div>
      ) : (
        <>
          <div className="space-y-2">
            {entries.map(([key, url]) => (
              <div key={key} className="flex items-center gap-2">
                <select
                  className="select select-bordered select-sm w-32"
                  value={PROTOCOL_OPTIONS.some(o => o.value === key) ? key : 'custom'}
                  onChange={(e) => handleProtocolChange(key, e.target.value)}
                >
                  {PROTOCOL_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                  <option value="custom">Custom</option>
                </select>
                <input
                  type="text"
                  className="input input-bordered input-sm flex-1"
                  placeholder="https://api.example.com"
                  value={url}
                  onChange={(e) => handleUrlChange(key, e.target.value)}
                />
                <button
                  type="button"
                  className="btn btn-ghost btn-sm btn-circle text-error hover:text-error"
                  onClick={() => handleRemove(key)}
                  aria-label="Remove endpoint"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            ))}
          </div>
          <Button variant="outline" size="sm" icon={<Plus className="h-4 w-4" />} onClick={handleAdd}>
            Add Endpoint
          </Button>
        </>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- --run web/src/components/ui/EndpointsEditor.test.tsx`
Expected: PASS (or no tests configured, component can be manually verified)

- [ ] **Step 5: Commit**

```bash
git add web/src/components/ui/EndpointsEditor.tsx web/src/components/ui/EndpointsEditor.test.tsx
git commit -m "feat: add EndpointsEditor component for dynamic key-value endpoints"
```

---

### Task 2: Modify Providers.tsx to Use Drawer

**Files:**
- Modify: `web/src/pages/Providers.tsx:1-302`

- [ ] **Step 1: Update imports to include Drawer and EndpointsEditor**

Find at top of file:
```tsx
import { Modal } from '../components/ui/Modal';
```

Replace with:
```tsx
import { Drawer } from '../components/ui/Drawer';
import { EndpointsEditor } from '../components/ui/EndpointsEditor';
```

- [ ] **Step 2: Replace create Modal with Drawer**

Find the create Modal section (lines 206-239):
```tsx
<Modal open={createOpen} onClose={() => setCreateOpen(false)} title="Add Provider">
  ...
</Modal>
```

Replace with:
```tsx
<Drawer open={createOpen} onClose={() => setCreateOpen(false)} title="Add Provider">
  <form onSubmit={handleCreate} className="space-y-4">
    <div className="form-control">
      <label className="label">
        <span className="label-text font-medium">Name</span>
      </label>
      <input type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g., OpenAI" required className="input input-bordered w-full" />
    </div>
    <div className="form-control">
      <label className="label">
        <span className="label-text font-medium">Base URL (Fallback)</span>
        <span className="label-text-alt">Optional</span>
      </label>
      <input type="text" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" />
    </div>
    <div className="form-control">
      <label className="label">
        <span className="label-text font-medium">Endpoints</span>
      </label>
      <EndpointsEditor
        value={createEndpoints}
        onChange={setCreateEndpoints}
      />
    </div>
    <div className="flex justify-end pt-4">
      <Button variant="primary" loading={createMutation.isPending}>Create</Button>
    </div>
  </form>
</Drawer>
```

- [ ] **Step 3: Replace edit Modal with Drawer**

Find the edit Modal section (lines 241-286):
```tsx
<Modal open={editOpen} onClose={() => setEditOpen(false)} title="Edit Provider">
  ...
</Modal>
```

Replace with:
```tsx
<Drawer open={editOpen} onClose={() => setEditOpen(false)} title="Edit Provider">
  <form onSubmit={handleUpdate} className="space-y-4">
    <div className="form-control">
      <label className="label">
        <span className="label-text font-medium">Name</span>
      </label>
      <input type="text" value={editName} onChange={(e) => setEditName(e.target.value)} placeholder="e.g., OpenAI" required className="input input-bordered w-full" />
    </div>
    <div className="form-control">
      <label className="label">
        <span className="label-text font-medium">Base URL (Fallback)</span>
        <span className="label-text-alt">Optional</span>
      </label>
      <input type="text" value={editBaseUrl} onChange={(e) => setEditBaseUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" />
    </div>
    <div className="form-control">
      <label className="label">
        <span className="label-text font-medium">Endpoints</span>
      </label>
      <EndpointsEditor
        value={editEndpoints}
        onChange={setEditEndpoints}
      />
    </div>
    <div className="form-control">
      <label className="label cursor-pointer justify-start gap-3">
        <input
          type="checkbox"
          checked={editEnabled}
          onChange={(e) => setEditEnabled(e.target.checked)}
          className="checkbox checkbox-primary"
        />
        <span className="label-text font-medium">Enabled</span>
      </label>
    </div>
    <div className="flex justify-end gap-2 pt-4">
      <Button variant="primary" loading={updateMutation.isPending}>Save Changes</Button>
    </div>
  </form>
</Drawer>
```

- [ ] **Step 4: Add createEndpoints state and update handleCreate**

Find the create state (around line 17-22):
```tsx
const [openaiUrl, setOpenaiUrl] = useState('');
const [anthropicUrl, setAnthropicUrl] = useState('');
```

Replace with:
```tsx
const [createEndpoints, setCreateEndpoints] = useState<Record<string, string>>({});
```

Find handleCreate function (lines 37-53):
```tsx
const handleCreate = async (e: React.FormEvent) => {
  e.preventDefault();
  const endpoints = JSON.stringify({
    openai: openaiUrl || null,
    anthropic: anthropicUrl || null,
  });
  await createMutation.mutateAsync({
    name,
    base_url: baseUrl || null,
    endpoints,
  });
  setName('');
  setBaseUrl('');
  setOpenaiUrl('');
  setAnthropicUrl('');
  setCreateOpen(false);
};
```

Replace with:
```tsx
const handleCreate = async (e: React.FormEvent) => {
  e.preventDefault();
  const endpoints: Record<string, string | null> = {};
  Object.entries(createEndpoints).forEach(([key, value]) => {
    endpoints[key] = value || null;
  });
  await createMutation.mutateAsync({
    name,
    base_url: baseUrl || null,
    endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null,
  });
  setName('');
  setBaseUrl('');
  setCreateEndpoints({});
  setCreateOpen(false);
};
```

- [ ] **Step 5: Add editEndpoints state and update handleEdit**

Find the edit state (around line 24-31):
```tsx
const [editOpenaiUrl, setEditOpenaiUrl] = useState('');
const [editAnthropicUrl, setEditAnthropicUrl] = useState('');
```

Replace with:
```tsx
const [editEndpoints, setEditEndpoints] = useState<Record<string, string>>({});
```

Find handleEdit function (lines 55-73):
```tsx
const handleEdit = (provider: Provider) => {
  // Parse endpoints JSON to extract individual URLs
  let openaiUrlStr = '';
  let anthropicUrlStr = '';
  if (provider.endpoints) {
    try {
      const parsed = JSON.parse(provider.endpoints);
      openaiUrlStr = parsed.openai || '';
      anthropicUrlStr = parsed.anthropic || '';
    } catch {}
  }
  setEditingProvider(provider);
  setEditName(provider.name);
  setEditBaseUrl(provider.base_url || '');
  setEditOpenaiUrl(openaiUrlStr);
  setEditAnthropicUrl(anthropicUrlStr);
  setEditEnabled(provider.enabled);
  setEditOpen(true);
};
```

Replace with:
```tsx
const handleEdit = (provider: Provider) => {
  let parsedEndpoints: Record<string, string> = {};
  if (provider.endpoints) {
    try {
      parsedEndpoints = JSON.parse(provider.endpoints);
    } catch {}
  }
  setEditingProvider(provider);
  setEditName(provider.name);
  setEditBaseUrl(provider.base_url || '');
  setEditEndpoints(parsedEndpoints);
  setEditEnabled(provider.enabled);
  setEditOpen(true);
};
```

- [ ] **Step 6: Update handleUpdate function**

Find handleUpdate (lines 75-93):
```tsx
const handleUpdate = async (e: React.FormEvent) => {
  e.preventDefault();
  if (!editingProvider) return;
  const endpoints = JSON.stringify({
    openai: editOpenaiUrl || null,
    anthropic: editAnthropicUrl || null,
  });
  await updateMutation.mutateAsync({
    id: editingProvider.id,
    input: {
      name: editName,
      base_url: editBaseUrl || null,
      endpoints,
      enabled: editEnabled,
    },
  });
  setEditOpen(false);
  setEditingProvider(null);
};
```

Replace with:
```tsx
const handleUpdate = async (e: React.FormEvent) => {
  e.preventDefault();
  if (!editingProvider) return;
  const endpoints: Record<string, string | null> = {};
  Object.entries(editEndpoints).forEach(([key, value]) => {
    endpoints[key] = value || null;
  });
  await updateMutation.mutateAsync({
    id: editingProvider.id,
    input: {
      name: editName,
      base_url: editBaseUrl || null,
      endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null,
      enabled: editEnabled,
    },
  });
  setEditOpen(false);
  setEditingProvider(null);
};
```

- [ ] **Step 7: Remove unused Modal import**

Find the import line (should still have Modal for delete confirmation):
```tsx
import { Modal } from '../components/ui/Modal';
```

Keep this - it's still used for delete confirmation.

- [ ] **Step 8: Run and verify**

Run: `cd web && npm run dev`
Expected: No build errors, providers page works with Drawer and dynamic endpoints

- [ ] **Step 9: Commit**

```bash
git add web/src/pages/Providers.tsx
git commit -m "feat: convert provider forms to Drawer with dynamic endpoints"
```

---

### Task 3: Verify Endpoints Display in Provider List

**Files:**
- Verify: `web/src/pages/Providers.tsx:145-164` (protocol badges section)

- [ ] **Step 1: Check that provider list still displays protocol badges**

The existing code parses `provider.endpoints` JSON to show badges. Verify it works with new dynamic format:
- It should work because the JSON format is the same (key-value pairs)
- OpenAI badge shows if `parsed.openai` exists
- Anthropic badge shows if `parsed.anthropic` exists

- [ ] **Step 2: Manual test**

1. Create a new provider with multiple endpoints
2. Edit the provider and add/remove endpoints
3. Verify badges update correctly

- [ ] **Step 3: Commit any fixes if needed**

---

## Verification Checklist

- [ ] Provider create form opens in Drawer (not Modal)
- [ ] Provider edit form opens in Drawer (not Modal)
- [ ] Endpoints displayed as dynamic rows, not fixed fields
- [ ] Can add new endpoint row via button
- [ ] Can remove endpoint row via delete button
- [ ] Can edit protocol and URL inline in each row
- [ ] Protocol dropdown has presets: OpenAI, Anthropic, Azure, Google, Custom
- [ ] "Custom" option allows freeform key entry
- [ ] Form data correctly serializes to JSON for backend
- [ ] Edit form correctly parses existing JSON to populate rows
- [ ] Provider list still shows protocol badges correctly