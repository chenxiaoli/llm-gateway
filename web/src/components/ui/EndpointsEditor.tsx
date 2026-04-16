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
    const newValue = { ...value };
    // Find first protocol that doesn't already exist
    for (const opt of PROTOCOL_OPTIONS) {
      if (!newValue[opt.value]) {
        newValue[opt.value] = '';
        onChange(newValue);
        return;
      }
    }
    // All protocols exist, add custom
    newValue['custom'] = '';
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
          <Button variant="outline" size="sm" icon={<Plus className="h-4 w-4" />} onClick={(e) => { e.stopPropagation(); handleAdd(); }} type="button">
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
                  onClick={(e) => { e.stopPropagation(); handleRemove(key); }}
                  aria-label="Remove endpoint"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            ))}
          </div>
          <Button variant="outline" size="sm" icon={<Plus className="h-4 w-4" />} onClick={(e) => { e.stopPropagation(); handleAdd(); }} type="button">
            Add Endpoint
          </Button>
        </>
      )}
    </div>
  );
}