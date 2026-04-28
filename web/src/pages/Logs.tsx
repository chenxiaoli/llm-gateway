import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  Search,
  RotateCcw,
  FileText,
  Clock,
  ChevronLeft,
  ChevronRight,
  Filter,
} from 'lucide-react';
import { useLogs, useLog } from '../hooks/useLogs';
import { useKeys } from '../hooks/useKeys';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Drawer } from '../components/ui/Drawer';
import JsonViewer from '../components/JsonViewer';
import type { AuditLogSummary } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Logs() {
  const reducedMotion = useReducedMotion();

  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [selectedLogId, setSelectedLogId] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);

  const { data, isLoading } = useLogs(
    { since: since || undefined, until: until || undefined, key_id: keyFilter || undefined },
    page,
    pageSize,
  );
  const { data: keys } = useKeys();
  const { data: selectedLog, isLoading: isLoadingDetail } = useLog(selectedLogId);

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  const clearFilters = () => {
    setSince('');
    setUntil('');
    setKeyFilter('');
    setPage(1);
  };

  const hasFilters = since || until || keyFilter;
  const filterCount = [since, until, keyFilter].filter(Boolean).length;

  const handleView = (log: AuditLogSummary) => {
    setSelectedLogId(log.id);
  };

  const anim = (delay = 0) =>
    reducedMotion
      ? false
      : {
          initial: { opacity: 0, y: 12 },
          animate: { opacity: 1, y: 0 },
          transition: { duration: 0.35, delay, ease: EASE },
        };

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-6 pt-8 flex items-center justify-between"
      >
        <div>
          <h1 className="text-3xl font-black tracking-tight text-base-content">Audit Logs</h1>
          <p className="text-base text-base-content/50 mt-1">
            API request history with full request/response bodies
          </p>
        </div>
        {data?.total != null && (
          <div className="flex items-center gap-2 px-4 py-2.5 rounded-xl border border-base-300/40 bg-base-100 backdrop-blur-sm">
            <FileText className="h-4 w-4 text-base-content/40" />
            <span className="mono text-sm font-medium">{data.total.toLocaleString()}</span>
            <span className="text-xs text-base-content/40">records</span>
          </div>
        )}
      </motion.div>

      {/* Filters */}
      <motion.div {...anim(0.05)} className="mb-5">
        <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
          <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60 flex items-center justify-between">
            <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25 flex items-center gap-1.5">
              <Filter className="h-3 w-3" />
              Filters
              {filterCount > 0 && (
                <span className="ml-1 inline-flex items-center justify-center w-4 h-4 rounded-full bg-primary/20 text-primary text-[10px] font-bold">
                  {filterCount}
                </span>
              )}
            </span>
            {hasFilters && (
              <Button
                variant="ghost"
                size="sm"
                icon={<RotateCcw className="h-3.5 w-3.5" />}
                onClick={clearFilters}
              >
                Clear
              </Button>
            )}
          </div>
          <div className="p-4 flex flex-wrap items-end gap-3">
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                <Clock className="h-3 w-3 inline mr-1" />
                From
              </label>
              <input
                type="date"
                value={since}
                onChange={(e) => { setSince(e.target.value); setPage(1); }}
                className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                <Clock className="h-3 w-3 inline mr-1" />
                To
              </label>
              <input
                type="date"
                value={until}
                onChange={(e) => { setUntil(e.target.value); setPage(1); }}
                className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                API Key
              </label>
              <select
                value={keyFilter}
                onChange={(e) => { setKeyFilter(e.target.value); setPage(1); }}
                className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              >
                <option value="">All Keys</option>
                {keys?.items?.map((k) => (
                  <option key={k.id} value={k.id}>{k.name}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <span className="loading loading-spinner loading-lg text-base-content/20" />
        </div>
      ) : (
        <>
          <motion.div {...anim(0.1)}>
            <div className="overflow-x-auto rounded-2xl border border-base-300/40 bg-base-100">
              <table className="table table-sm">
                <thead>
                  <tr className="border-b border-base-300/40">
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Time</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Model</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Protocol</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Stream</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Status</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Latency</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Input</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Output</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {data?.items?.length === 0 && (
                    <tr>
                      <td colSpan={9} className="text-center py-12 text-base-content/30">
                        <Search className="h-8 w-8 mx-auto mb-2 opacity-40" />
                        <div>No logs found</div>
                        {hasFilters && <div className="text-xs mt-1">Try adjusting your filters</div>}
                      </td>
                    </tr>
                  )}
                  {data?.items?.map((log) => (
                    <tr
                      key={log.id}
                      className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors cursor-pointer"
                      onClick={() => handleView(log)}
                    >
                      <td className="mono text-[13px] text-base-content/55">
                        {new Date(log.created_at).toLocaleString()}
                      </td>
                      <td className="mono font-medium">{log.model_name}</td>
                      <td>
                        <Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>
                          {log.protocol}
                        </Badge>
                      </td>
                      <td>
                        {log.stream
                          ? <Badge variant="blue">stream</Badge>
                          : <span className="text-base-content/30">-</span>}
                      </td>
                      <td>
                        <Badge
                          variant={
                            log.status_code < 400
                              ? 'green'
                              : log.status_code < 500
                                ? 'amber'
                                : 'red'
                          }
                        >
                          {log.status_code}
                        </Badge>
                      </td>
                      <td className="mono text-base-content/55">{log.latency_ms}ms</td>
                      <td className="mono text-base-content/55">{log.input_tokens ?? '-'}</td>
                      <td className="mono text-base-content/55">{log.output_tokens ?? '-'}</td>
                      <td>
                        <button
                          onClick={(e) => { e.stopPropagation(); handleView(log); }}
                          className="btn btn-ghost btn-xs gap-1 text-primary"
                        >
                          <FileText className="h-3.5 w-3.5" />
                          View
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </motion.div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-xs text-base-content/40 mono">
                {data?.total ?? 0} total
              </span>
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
                <span className="px-3 flex items-center text-sm text-base-content/60 mono">
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

      {/* Detail Drawer */}
      <Drawer
        open={!!selectedLogId}
        onClose={() => setSelectedLogId(null)}
        title="Log Detail"
        width={720}
      >
        {isLoadingDetail && (
          <div className="flex justify-center py-12">
            <span className="loading loading-spinner loading-md" />
          </div>
        )}
        {selectedLog && (
          <div className="space-y-5">
            {/* Metadata grid */}
            <div className="rounded-xl border border-base-300/40 bg-base-100 overflow-hidden">
              <div className="px-4 py-3 border-b border-base-300/60 bg-base-100/60">
                <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                  REQUEST DETAILS
                </span>
              </div>
              <div className="p-4 grid grid-cols-2 sm:grid-cols-3 gap-3">
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Time</div>
                  <div className="mono text-[13px]">{new Date(selectedLog.created_at).toLocaleString()}</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Model</div>
                  <div className="mono text-[13px] font-medium">{selectedLog.model_name}</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Protocol</div>
                  <Badge variant={selectedLog.protocol === 'openai' ? 'blue' : 'purple'}>
                    {selectedLog.protocol}
                  </Badge>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Stream</div>
                  {selectedLog.stream
                    ? <Badge variant="blue">stream</Badge>
                    : <span className="text-base-content/30">-</span>}
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Status</div>
                  <Badge
                    variant={
                      selectedLog.status_code < 400
                        ? 'green'
                        : selectedLog.status_code < 500
                          ? 'amber'
                          : 'red'
                    }
                  >
                    {selectedLog.status_code}
                  </Badge>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Latency</div>
                  <div className="mono text-[13px]">{selectedLog.latency_ms}ms</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Tokens</div>
                  <div className="mono text-[13px]">
                    {selectedLog.input_tokens ?? 0} in / {selectedLog.output_tokens ?? 0} out
                  </div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Provider</div>
                  <div className="mono text-[13px] text-base-content/70">
                    {selectedLog.provider_id.slice(0, 8)}…
                  </div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">Channel</div>
                  <div className="mono text-[13px] text-base-content/70">
                    {selectedLog.channel_id ? `${selectedLog.channel_id.slice(0, 8)}…` : '-'}
                  </div>
                </div>
              </div>
            </div>

            {/* Routing info */}
            {(selectedLog.request_path || selectedLog.upstream_url || selectedLog.original_model || selectedLog.upstream_model) && (
              <div className="rounded-xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-4 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    ROUTING
                  </span>
                </div>
                <div className="p-4 grid grid-cols-1 sm:grid-cols-2 gap-3">
                  {selectedLog.request_path && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">Request Path</div>
                      <div className="mono text-[13px] font-medium">{selectedLog.request_path}</div>
                    </div>
                  )}
                  {selectedLog.upstream_url && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">Upstream URL</div>
                      <div className="mono text-[13px] text-wrap break-all">{selectedLog.upstream_url}</div>
                    </div>
                  )}
                  {selectedLog.original_model && selectedLog.original_model !== selectedLog.model_name && (
                    <>
                      <div>
                        <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">Original Model</div>
                        <div className="mono text-[13px]">{selectedLog.original_model}</div>
                      </div>
                      <div>
                        <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">Upstream Model</div>
                        <div className="mono text-[13px]">{selectedLog.upstream_model ?? selectedLog.model_name}</div>
                      </div>
                      {selectedLog.model_override_reason && (
                        <div className="sm:col-span-2">
                          <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">Override Reason</div>
                          <div className="mono text-[13px]">{selectedLog.model_override_reason}</div>
                        </div>
                      )}
                    </>
                  )}
                </div>
              </div>
            )}

            {/* Headers */}
            {(selectedLog.request_headers || selectedLog.response_headers) && (
              <div className="rounded-xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-4 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    HEADERS
                  </span>
                </div>
                <div className="p-4 grid grid-cols-1 sm:grid-cols-2 gap-4">
                  {selectedLog.request_headers && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-2">Request</div>
                      <JsonViewer data={selectedLog.request_headers} />
                    </div>
                  )}
                  {selectedLog.response_headers && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-2">Response</div>
                      <JsonViewer data={selectedLog.response_headers} />
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Request body */}
            <div className="rounded-xl border border-base-300/40 bg-base-100 overflow-hidden">
              <div className="px-4 py-3 border-b border-base-300/60 bg-base-100/60">
                <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                  REQUEST BODY
                </span>
              </div>
              <div className="p-4">
                <JsonViewer data={selectedLog.request_body} />
              </div>
            </div>

            {/* Response body */}
            <div className="rounded-xl border border-base-300/40 bg-base-100 overflow-hidden">
              <div className="px-4 py-3 border-b border-base-300/60 bg-base-100/60">
                <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                  RESPONSE BODY
                </span>
              </div>
              <div className="p-4">
                <JsonViewer data={selectedLog.response_body} />
              </div>
            </div>
          </div>
        )}
      </Drawer>
    </div>
  );
}
