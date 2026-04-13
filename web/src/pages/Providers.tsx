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
      <div className="mb-6 flex items-center justify-between">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Providers</h1>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Add Provider
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-[#555555]">Loading...</div>
      ) : (
        <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[#1e1e1e]">
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Name</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Protocols</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Created</th>
              </tr>
            </thead>
            <tbody>
              {providers?.map((provider) => (
                <tr key={provider.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                  <td className="px-4 py-2.5">
                    <button onClick={() => navigate(`/console/providers/${provider.id}`)} className="text-accent hover:text-accent-hover transition-colors">
                      {provider.name}
                    </button>
                  </td>
                  <td className="px-4 py-2.5">
                    <div className="flex gap-1.5">
                      {provider.openai_base_url && <Badge variant="blue">OpenAI</Badge>}
                      {provider.anthropic_base_url && <Badge variant="purple">Anthropic</Badge>}
                    </div>
                  </td>
                  <td className="px-4 py-2.5"><Badge variant={provider.enabled ? 'green' : 'red'}>{provider.enabled ? 'Active' : 'Disabled'}</Badge></td>
                  <td className="px-4 py-2.5"><span className="mono">{new Date(provider.created_at).toLocaleDateString()}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <Modal open={createOpen} onClose={() => setCreateOpen(false)} title="Add Provider">
        <form onSubmit={handleCreate} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., OpenAI"
              required
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">OpenAI Base URL</label>
            <input
              type="text"
              value={openaiUrl}
              onChange={(e) => setOpenaiUrl(e.target.value)}
              placeholder="https://api.openai.com/v1"
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Anthropic Base URL</label>
            <input
              type="text"
              value={anthropicUrl}
              onChange={(e) => setAnthropicUrl(e.target.value)}
              placeholder="https://api.anthropic.com"
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
            />
          </div>
          <Button variant="primary" loading={createMutation.isPending}>Create</Button>
        </form>
      </Modal>
    </div>
  );
}
