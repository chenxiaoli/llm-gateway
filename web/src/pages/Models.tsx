import { useState } from 'react';
import { useAllModels, useCreateGlobalModel } from '../hooks/useModels';
import { useProviders } from '../hooks/useProviders';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Globe, Plus } from 'lucide-react';
import type { CreateGlobalModelRequest } from '../types';

export default function Models() {
  const { data: models, isLoading: modelsLoading } = useAllModels();
  const { data: providers } = useProviders();
  const createMutation = useCreateGlobalModel();

  const [isAdding, setIsAdding] = useState(false);
  const [newModel, setNewModel] = useState({
    provider_id: '',
    name: '',
    billing_type: 'token' as 'token' | 'request',
    input_price: '',
    output_price: '',
    request_price: '',
  });

  if (modelsLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <span className="loading loading-spinner loading-lg" />
      </div>
    );
  }

  const getProviderName = (providerId: string) => {
    return providers?.find(p => p.id === providerId)?.name ?? providerId;
  };

  const handleAddModel = async (e: React.FormEvent) => {
    e.preventDefault();
    const input: CreateGlobalModelRequest = {
      provider_id: newModel.provider_id,
      name: newModel.name,
      billing_type: newModel.billing_type,
      input_price: newModel.input_price ? parseFloat(newModel.input_price) : undefined,
      output_price: newModel.output_price ? parseFloat(newModel.output_price) : undefined,
      request_price: newModel.request_price ? parseFloat(newModel.request_price) : undefined,
    };
    await createMutation.mutateAsync(input);
    setIsAdding(false);
    setNewModel({ provider_id: '', name: '', billing_type: 'token', input_price: '', output_price: '', request_price: '' });
  };

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Models</h1>
          <p className="text-sm text-base-content/40 mt-1">Manage available AI models</p>
        </div>
        {providers?.length ? (
          <Button icon={<Plus className="h-4 w-4" />} onClick={() => setIsAdding(true)}>
            Add Model
          </Button>
        ) : null}
      </div>

      {!models?.length ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <div className="text-base-content/30 text-sm mb-4">No models configured</div>
          {providers?.length ? (
            <Button variant="secondary" size="sm" onClick={() => setIsAdding(true)}>Add your first model</Button>
          ) : (
            <Link to="/console/providers">
              <Button variant="secondary" size="sm">Add provider first</Button>
            </Link>
          )}
        </div>
      ) : (
        <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/50">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Name</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Provider</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Status</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Billing</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35 w-24">Actions</th>
                </tr>
              </thead>
              <tbody>
                {models.map((model) => (
                  <tr key={model.id} className="border-b border-base-200/50">
                    <td className="font-medium font-mono">{model.name}</td>
                    <td className="flex items-center gap-2">
                      <Globe className="h-3.5 w-3.5 text-base-content/40" />
                      {model.provider_name || getProviderName(model.provider_id)}
                    </td>
                    <td>
                      <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${
                        model.enabled
                          ? 'bg-success/10 text-success'
                          : 'bg-base-300/50 text-base-content/40'
                      }`}>
                        <span className={`w-1.5 h-1.5 rounded-full ${model.enabled ? 'bg-success' : 'bg-base-content/30'}`} />
                        {model.enabled ? 'Active' : 'Disabled'}
                      </span>
                    </td>
                    <td className="text-sm text-base-content/60">
                      {model.billing_type === 'token' ? 'Token' : 'Request'}
                    </td>
                    <td>
                      <Link
                        to={`/console/providers/${model.provider_id}`}
                        className="btn btn-ghost btn-xs"
                      >
                        Manage
                      </Link>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal open={isAdding} onClose={() => setIsAdding(false)} title="Add Model">
        <form onSubmit={handleAddModel} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text">Provider</span>
            </label>
            <select
              value={newModel.provider_id}
              onChange={(e) => setNewModel({ ...newModel, provider_id: e.target.value })}
              required
              className="select select-bordered w-full"
            >
              <option value="">Select provider</option>
              {providers?.map((p) => (
                <option key={p.id} value={p.id}>{p.name}</option>
              ))}
            </select>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">Model Name</span>
            </label>
            <input
              type="text"
              value={newModel.name}
              onChange={(e) => setNewModel({ ...newModel, name: e.target.value })}
              required
              placeholder="e.g., gpt-4o"
              className="input input-bordered w-full"
            />
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">Billing Type</span>
            </label>
            <select
              value={newModel.billing_type}
              onChange={(e) => setNewModel({ ...newModel, billing_type: e.target.value as 'token' | 'request' })}
              className="select select-bordered w-full"
            >
              <option value="token">Token</option>
              <option value="request">Request</option>
            </select>
          </div>

          <div className="grid grid-cols-3 gap-3">
            <div className="form-control">
              <label className="label">
                <span className="label-text">Input Price</span>
              </label>
              <input
                type="number"
                step="0.0001"
                value={newModel.input_price}
                onChange={(e) => setNewModel({ ...newModel, input_price: e.target.value })}
                placeholder="0.00"
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label">
                <span className="label-text">Output Price</span>
              </label>
              <input
                type="number"
                step="0.0001"
                value={newModel.output_price}
                onChange={(e) => setNewModel({ ...newModel, output_price: e.target.value })}
                placeholder="0.00"
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label">
                <span className="label-text">Request Price</span>
              </label>
              <input
                type="number"
                step="0.0001"
                value={newModel.request_price}
                onChange={(e) => setNewModel({ ...newModel, request_price: e.target.value })}
                placeholder="0.00"
                className="input input-bordered w-full"
              />
            </div>
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={createMutation.isPending}>
              Add Model
            </Button>
            <Button variant="ghost" type="button" onClick={() => setIsAdding(false)}>
              Cancel
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}