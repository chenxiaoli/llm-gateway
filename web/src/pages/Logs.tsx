import { useState } from 'react';
import { Search, RotateCcw, FileText, Clock, ChevronLeft, ChevronRight } from 'lucide-react';
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

  const clearFilters = () => {
    setSince('');
    setUntil('');
    setKeyFilter('');
    setPage(1);
  };

  const hasFilters = since || until || keyFilter;

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Audit Logs</h1>
          <p className="text-sm text-base-content/40 mt-1">API request history with full request/response bodies</p>
        </div>
        {data?.total != null && (
          <div className="flex items-center gap-2 text-sm text-base-content/50">
            <FileText className="h-4 w-4" />
            <span className="mono">{data.total.toLocaleString()} records</span>
          </div>
        )}
      </div>

      {/* Filters */}
      <div className="mb-5 bg-base-100 rounded-box border border-base-300 p-4">
        <div className="flex flex-wrap items-end gap-3">
          <div className="form-control">
            <label className="label py-1">
              <span className="label-text text-xs font-medium uppercase tracking-wider text-base-content/50">
                <Clock className="h-3 w-3 inline mr-1" />From
              </span>
            </label>
            <input
              type="date"
              value={since}
              onChange={(e) => { setSince(e.target.value); setPage(1); }}
              className="input input-bordered input-sm w-44"
            />
          </div>
          <div className="form-control">
            <label className="label py-1">
              <span className="label-text text-xs font-medium uppercase tracking-wider text-base-content/50">
                <Clock className="h-3 w-3 inline mr-1" />To
              </span>
            </label>
            <input
              type="date"
              value={until}
              onChange={(e) => { setUntil(e.target.value); setPage(1); }}
              className="input input-bordered input-sm w-44"
            />
          </div>
          <div className="form-control">
            <label className="label py-1">
              <span className="label-text text-xs font-medium uppercase tracking-wider text-base-content/50">
                API Key
              </span>
            </label>
            <select
              value={keyFilter}
              onChange={(e) => { setKeyFilter(e.target.value); setPage(1); }}
              className="select select-bordered select-sm w-48"
            >
              <option value="">All Keys</option>
              {keys?.items?.map((k) => (<option key={k.id} value={k.id}>{k.name}</option>))}
            </select>
          </div>
          <div className="flex gap-2 self-end">
            {hasFilters && (
              <Button variant="ghost" size="sm" icon={<RotateCcw className="h-3.5 w-3.5" />} onClick={clearFilters}>
                Clear
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <span className="loading loading-spinner loading-lg text-primary" />
        </div>
      ) : (
        <>
          <div className="overflow-x-auto bg-base-100 rounded-box border border-base-300 shadow-sm">
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
                {data?.items?.length === 0 && (
                  <tr>
                    <td colSpan={8} className="text-center py-12 text-base-content/30">
                      <Search className="h-8 w-8 mx-auto mb-2 opacity-40" />
                      <div>No logs found</div>
                      {hasFilters && <div className="text-xs mt-1">Try adjusting your filters</div>}
                    </td>
                  </tr>
                )}
                {data?.items?.map((log) => (
                  <tr key={log.id} className="border-b border-base-200 hover">
                    <td className="mono text-[13px] text-base-content/60">{new Date(log.created_at).toLocaleString()}</td>
                    <td className="mono font-medium">{log.model_name}</td>
                    <td><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                    <td><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                    <td className="mono text-base-content/60">{log.latency_ms}ms</td>
                    <td className="mono text-base-content/60">{log.input_tokens ?? '-'}</td>
                    <td className="mono text-base-content/60">{log.output_tokens ?? '-'}</td>
                    <td>
                      <button onClick={() => setSelectedLog(log)} className="btn btn-ghost btn-xs gap-1 text-primary">
                        <FileText className="h-3.5 w-3.5" />
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
              <span className="text-base-content/40 mono text-xs">{data?.total ?? 0} total</span>
              <div className="join">
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<ChevronLeft className="h-4 w-4" />}
                  disabled={page <= 1}
                  onClick={() => setPage(page - 1)}
                >
                  Previous
                </Button>
                <span className="px-3 flex items-center text-base-content/60 mono text-xs">
                  {page} / {totalPages}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={page >= totalPages}
                  onClick={() => setPage(page + 1)}
                >
                  Next
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          )}
        </>
      )}

      <Drawer open={!!selectedLog} onClose={() => setSelectedLog(null)} title="Log Detail" width={700}>
        {selectedLog && (
          <div className="space-y-5">
            {/* Metadata grid */}
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
              <div className="bg-base-200/60 rounded-lg p-3">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Time</div>
                <div className="mono text-[13px]">{new Date(selectedLog.created_at).toLocaleString()}</div>
              </div>
              <div className="bg-base-200/60 rounded-lg p-3">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Model</div>
                <div className="mono text-[13px] font-medium">{selectedLog.model_name}</div>
              </div>
              <div className="bg-base-200/60 rounded-lg p-3">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Protocol</div>
                <Badge variant={selectedLog.protocol === 'openai' ? 'blue' : 'purple'}>{selectedLog.protocol}</Badge>
              </div>
              <div className="bg-base-200/60 rounded-lg p-3">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Status</div>
                <Badge variant={selectedLog.status_code < 400 ? 'green' : selectedLog.status_code < 500 ? 'amber' : 'red'}>{selectedLog.status_code}</Badge>
              </div>
              <div className="bg-base-200/60 rounded-lg p-3">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Latency</div>
                <div className="mono text-[13px]">{selectedLog.latency_ms}ms</div>
              </div>
              <div className="bg-base-200/60 rounded-lg p-3">
                <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Tokens</div>
                <div className="mono text-[13px]">{selectedLog.input_tokens ?? 0} in / {selectedLog.output_tokens ?? 0} out</div>
              </div>
            </div>

            {/* Request body */}
            <div>
              <h3 className="text-sm font-semibold mb-2 text-base-content/70 uppercase tracking-wider">Request Body</h3>
              <JsonViewer data={selectedLog.request_body} />
            </div>

            {/* Response body */}
            <div>
              <h3 className="text-sm font-semibold mb-2 text-base-content/70 uppercase tracking-wider">Response Body</h3>
              <JsonViewer data={selectedLog.response_body} />
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}
