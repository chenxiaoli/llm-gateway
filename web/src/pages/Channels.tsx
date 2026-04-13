import { useProviders } from '../hooks/useProviders';
import { Link } from 'react-router-dom';

export default function Channels() {
  const { data: providers } = useProviders();

  return (
    <div>
      <div className="mb-8">
        <h1 className="text-2xl font-bold">Channels</h1>
        <p className="text-sm text-base-content/40 mt-1">Manage provider failover endpoints</p>
      </div>

      <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300/50">
                <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Provider</th>
                <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Channel</th>
                <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Endpoint</th>
                <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Priority</th>
                <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Status</th>
              </tr>
            </thead>
            <tbody>
              {providers?.map((provider) => (
                <ProviderRow key={provider.id} provider={provider} />
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

function ProviderRow({ provider }: { provider: { id: string; name: string; enabled: boolean } }) {
  return (
    <tr className="border-b border-base-200/50">
      <td className="font-medium">{provider.name}</td>
      <td colSpan={4}>
        <Link
          to={`/console/providers/${provider.id}`}
          className="text-primary hover:underline text-sm"
        >
          View channels →
        </Link>
      </td>
    </tr>
  );
}