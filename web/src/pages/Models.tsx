import { useAllModels } from '../hooks/useModels';
import { useProviders } from '../hooks/useProviders';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Globe } from 'lucide-react';

export default function Models() {
  const { data: models, isLoading: modelsLoading } = useAllModels();
  const { data: providers } = useProviders();

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

  return (
    <div>
      <div className="mb-8">
        <div>
          <h1 className="text-2xl font-bold">Models</h1>
          <p className="text-sm text-base-content/40 mt-1">Manage available AI models</p>
        </div>
      </div>

      {!models?.length ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <div className="text-base-content/30 text-sm mb-4">No models configured</div>
          <Link to="/console/providers">
            <Button variant="secondary" size="sm">Add your first model</Button>
          </Link>
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
    </div>
  );
}