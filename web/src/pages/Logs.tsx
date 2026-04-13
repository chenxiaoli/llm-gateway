import { useState } from 'react';
import { useLogs } from '../hooks/useLogs';
import { useKeys } from '../hooks/useKeys';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Drawer } from '../components/ui/Drawer';
import JsonViewer from '../components/JsonViewer';
import type { AuditLog } from '../types';

export default function Logs() {
  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [selectedLog, setSelectedLog] = useState<AuditLog | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);

  const { data, isLoading } = useLogs(
    { since: since || undefined, until: until || undefined, key_id: keyFilter || undefined },
    page,
    pageSize,
  );
  const { data: keys } = useKeys();

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  return (
    <div>
      <div className="mb-6"><h1 className="font-display text-2xl font-bold">Audit Logs</h1></div>

      {/* Filters */}
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <input type="date" value={since} onChange={(e) => setSince(e.target.value)} className="input input-bordered input-sm" />
        <input type="date" value={until} onChange={(e) => setUntil(e.target.value)} className="input input-bordered input-sm" />
        <select value={keyFilter} onChange={(e) => setKeyFilter(e.target.value)} className="select select-bordered select-sm">
          <option value="">All API Keys</option>
          {keys?.items?.map((k) => (<option key={k.id} value={k.id}>{k.name}</option>))}
        </select>
      </div>

      {/* Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Time</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Model</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Protocol</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Status</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Latency</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Input</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Output</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Actions</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((log) => (
                  <tr key={log.id} className="border-b border-base-200 hover">
                    <td className="mono text-[13px]">{new Date(log.created_at).toLocaleString()}</td>
                    <td className="mono">{log.model_name}</td>
                    <td><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                    <td><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                    <td className="mono">{log.latency_ms}ms</td>
                    <td className="mono">{log.input_tokens ?? '-'}</td>
                    <td className="mono">{log.output_tokens ?? '-'}</td>
                    <td>
                      <button onClick={() => setSelectedLog(log)} className="link link-primary text-sm">View</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-base-content/40">Total {data?.total ?? 0}</span>
              <div className="join">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-3 flex items-center text-base-content/60">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}

      <Drawer open={!!selectedLog} onClose={() => setSelectedLog(null)} title="Log Detail" width={700}>
        {selectedLog && (
          <div className="space-y-3">
            <div className="flex flex-wrap gap-4 text-sm">
              <div><span className="text-base-content/50">Time:</span> <span className="mono">{new Date(selectedLog.created_at).toLocaleString()}</span></div>
              <div><span className="text-base-content/50">Model:</span> <span className="mono">{selectedLog.model_name}</span></div>
              <div><span className="text-base-content/50">Protocol:</span> <Badge variant={selectedLog.protocol === 'openai' ? 'blue' : 'purple'}>{selectedLog.protocol}</Badge></div>
              <div><span className="text-base-content/50">Status:</span> <Badge variant={selectedLog.status_code < 400 ? 'green' : 'red'}>{selectedLog.status_code}</Badge></div>
              <div><span className="text-base-content/50">Latency:</span> <span className="mono">{selectedLog.latency_ms}ms</span></div>
              <div><span className="text-base-content/50">Tokens:</span> <span className="mono">{selectedLog.input_tokens ?? 0} in / {selectedLog.output_tokens ?? 0} out</span></div>
            </div>

            <div>
              <h3 className="font-display text-sm font-semibold mt-4 mb-2">Request Body</h3>
              <JsonViewer data={selectedLog.request_body} />
            </div>

            <div>
              <h3 className="font-display text-sm font-semibold mt-4 mb-2">Response Body</h3>
              <JsonViewer data={selectedLog.response_body} />
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}
