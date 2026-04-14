import { useProviders } from '../hooks/useProviders';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';

export default function Channels() {
  const { data: providers, isLoading } = useProviders();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <span className="loading loading-spinner loading-lg" />
      </div>
    );
  }

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Channels</h1>
          <p className="text-sm text-base-content/40 mt-1">Manage provider failover endpoints</p>
        </div>
        <Link to="/console/providers">
          <Button variant="secondary" size="sm">
            Manage Providers
          </Button>
        </Link>
      </div>

      {!providers?.length ? (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <div className="text-base-content/30 text-sm mb-4">No providers configured</div>
          <Link to="/console/providers">
            <Button variant="secondary" size="sm">Add your first provider</Button>
          </Link>
        </div>
      ) : (
        <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/50">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Provider</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Status</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Channels</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35 w-24">Actions</th>
                </tr>
              </thead>
              <tbody>
                {providers.map((provider) => (
                  <ProviderRow key={provider.id} provider={provider} />
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

function ProviderRow({ provider }: { provider: { id: string; name: string; enabled: boolean } }) {
  return (
    <tr className="border-b border-base-200/50">
      <td className="font-medium">{provider.name}</td>
      <td>
        <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${
          provider.enabled
            ? 'bg-success/10 text-success'
            : 'bg-base-300/50 text-base-content/40'
        }`}>
          <span className={`w-1.5 h-1.5 rounded-full ${provider.enabled ? 'bg-success' : 'bg-base-content/30'}`} />
          {provider.enabled ? 'Active' : 'Disabled'}
        </span>
      </td>
      <td className="text-base-content/60 text-sm">View in provider</td>
      <td>
        <Link
          to={`/console/providers/${provider.id}`}
          className="btn btn-ghost btn-xs"
        >
          Manage
        </Link>
      </td>
    </tr>
  );
}