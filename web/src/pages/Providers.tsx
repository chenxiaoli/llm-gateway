import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { useProviders, useCreateProvider } from '../hooks/useProviders';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';

export default function Providers() {
  const { data: providers, isLoading } = useProviders();
  const createMutation = useCreateProvider();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [openaiUrl, setOpenaiUrl] = useState('');
  const [anthropicUrl, setAnthropicUrl] = useState('');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    await createMutation.mutateAsync({
      name,
      openai_base_url: openaiUrl || null,
      anthropic_base_url: anthropicUrl || null,
    });
    setName('');
    setOpenaiUrl('');
    setAnthropicUrl('');
    setCreateOpen(false);
  };

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Providers</h1>
          <p className="text-sm text-base-content/40 mt-1">Configure upstream LLM provider endpoints</p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Add Provider
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/50">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Name</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Protocols</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Status</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Created</th>
                </tr>
              </thead>
              <tbody>
                {providers?.map((provider) => (
                  <tr key={provider.id} className="border-b border-base-200/50 hover:bg-base-200/30 transition-colors">
                    <td>
                      <button onClick={() => navigate(`/console/providers/${provider.id}`)} className="link link-primary text-sm font-medium">
                        {provider.name}
                      </button>
                    </td>
                    <td>
                      <div className="flex gap-1.5">
                        {provider.openai_base_url && <Badge variant="blue">OpenAI</Badge>}
                        {provider.anthropic_base_url && <Badge variant="purple">Anthropic</Badge>}
                      </div>
                    </td>
                    <td><Badge variant={provider.enabled ? 'green' : 'red'}>{provider.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td className="mono text-[13px] text-base-content/50">{new Date(provider.created_at).toLocaleDateString()}</td>
                  </tr>
                ))}
                {(!providers?.length) && (
                  <tr>
                    <td colSpan={4} className="text-center py-16 text-base-content/25 text-sm">
                      No providers configured
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal open={createOpen} onClose={() => setCreateOpen(false)} title="Add Provider">
        <form onSubmit={handleCreate} className="space-y-4">
          <div className="form-control">
            <label className="label"><span className="label-text font-medium">Name</span></label>
            <input type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g., OpenAI" required className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text font-medium">OpenAI Base URL</span></label>
            <input type="text" value={openaiUrl} onChange={(e) => setOpenaiUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text font-medium">Anthropic Base URL</span></label>
            <input type="text" value={anthropicUrl} onChange={(e) => setAnthropicUrl(e.target.value)} placeholder="https://api.anthropic.com" className="input input-bordered w-full" />
          </div>
          <Button variant="primary" loading={createMutation.isPending}>Create</Button>
        </form>
      </Modal>
    </div>
  );
}