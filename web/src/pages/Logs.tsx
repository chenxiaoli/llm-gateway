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
      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Audit Logs</h1>
      </div>

      {/* Filters */}
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <input
          type="date"
          value={since}
          onChange={(e) => setSince(e.target.value)}
          className="h-9 rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors"
        />
        <input
          type="date"
          value={until}
          onChange={(e) => setUntil(e.target.value)}
          className="h-9 rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors"
        />
        <select
          value={keyFilter}
          onChange={(e) => setKeyFilter(e.target.value)}
          className="h-9 rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors"
        >
          <option value="">All API Keys</option>
          {keys?.items?.map((k) => (
            <option key={k.id} value={k.id}>{k.name}</option>
          ))}
        </select>
      </div>

      {/* Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-[#555555]">Loading...</div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e1e1e]">
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Time</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Model</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Protocol</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Latency</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Input</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Output</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Actions</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((log) => (
                  <tr key={log.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                    <td className="px-4 py-2.5"><span className="mono text-[13px]">{new Date(log.created_at).toLocaleString()}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{log.model_name}</span></td>
                    <td className="px-4 py-2.5"><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                    <td className="px-4 py-2.5"><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                    <td className="px-4 py-2.5"><span className="mono">{log.latency_ms}ms</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{log.input_tokens ?? '-'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{log.output_tokens ?? '-'}</span></td>
                    <td className="px-4 py-2.5">
                      <button onClick={() => setSelectedLog(log)} className="text-accent hover:text-accent-hover transition-colors text-sm">
                        View
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-[#555555]">Total {data?.total ?? 0}</span>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-2 text-[#888888]">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Detail Drawer */}
      <Drawer open={!!selectedLog} onClose={() => setSelectedLog(null)} title="Log Detail" width={700}>
        {selectedLog && (
          <div className="space-y-3">
            <div className="flex gap-8 text-sm">
              <div><span className="text-[#555555]">Time:</span> <span className="mono">{new Date(selectedLog.created_at).toLocaleString()}</span></div>
              <div><span className="text-[#555555]">Model:</span> <span className="mono">{selectedLog.model_name}</span></div>
              <div><span className="text-[#555555]">Protocol:</span> <Badge variant={selectedLog.protocol === 'openai' ? 'blue' : 'purple'}>{selectedLog.protocol}</Badge></div>
              <div><span className="text-[#555555]">Status:</span> <Badge variant={selectedLog.status_code < 400 ? 'green' : 'red'}>{selectedLog.status_code}</Badge></div>
              <div><span className="text-[#555555]">Latency:</span> <span className="mono">{selectedLog.latency_ms}ms</span></div>
              <div><span className="text-[#555555]">Tokens:</span> <span className="mono">{selectedLog.input_tokens ?? 0} in / {selectedLog.output_tokens ?? 0} out</span></div>
            </div>

            <div>
              <h3 className="font-display text-sm font-semibold text-[#ededed] mt-4 mb-2">Request Body</h3>
              <JsonViewer data={selectedLog.request_body} />
            </div>

            <div>
              <h3 className="font-display text-sm font-semibold text-[#ededed] mt-4 mb-2">Response Body</h3>
              <JsonViewer data={selectedLog.response_body} />
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}
