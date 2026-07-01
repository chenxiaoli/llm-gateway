import { useState } from 'react';
import { useTranslation } from 'react-i18next';
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
import { useAllChannels } from '../hooks/useChannels';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { CopyButton } from '../components/ui/CopyButton';
import { Drawer } from '../components/ui/Drawer';
import JsonViewer from '../components/JsonViewer';
import type { AuditLogSummary } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Logs() {
  const reducedMotion = useReducedMotion();
  const { t } = useTranslation();

  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [channelFilter, setChannelFilter] = useState('');
  const [requestIdFilter, setRequestIdFilter] = useState('');
  const [selectedLogId, setSelectedLogId] = useState<string | null>(null);
  const [routesModalLog, setRoutesModalLog] = useState<AuditLogSummary | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);

  const { data, isLoading } = useLogs(
    {
      since: since || undefined,
      until: until || undefined,
      key_id: keyFilter || undefined,
      channel_id: channelFilter || undefined,
      request_id: requestIdFilter || undefined,
    },
    page,
    pageSize,
  );
  const { data: keys } = useKeys();
  const { data: channels } = useAllChannels();
  const { data: selectedLog, isLoading: isLoadingDetail } = useLog(selectedLogId);

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  const clearFilters = () => {
    setSince('');
    setUntil('');
    setKeyFilter('');
    setChannelFilter('');
    setRequestIdFilter('');
    setPage(1);
  };

  const hasFilters = since || until || keyFilter || channelFilter || requestIdFilter;
  const filterCount = [since, until, keyFilter, channelFilter, requestIdFilter].filter(Boolean).length;

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
          <h1 className="text-3xl font-black tracking-tight text-base-content">{t('logs.title')}</h1>
          <p className="text-base text-base-content/50 mt-1">
            {t('logs.description')}
          </p>
        </div>
        {data?.total != null && (
          <div className="flex items-center gap-2 px-4 py-2.5 rounded-xl border border-base-300/40 bg-base-100 backdrop-blur-sm">
            <FileText className="h-4 w-4 text-base-content/40" />
            <span className="mono text-sm font-medium">{data.total.toLocaleString()}</span>
            <span className="text-xs text-base-content/40">{t('logs.records')}</span>
          </div>
        )}
      </motion.div>

      {/* Filters */}
      <motion.div {...anim(0.05)} className="mb-5">
        <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
          <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60 flex items-center justify-between">
            <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25 flex items-center gap-1.5">
              <Filter className="h-3 w-3" />
              {t('logs.filters')}
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
                {t('logs.clear')}
              </Button>
            )}
          </div>
          <div className="p-4 flex flex-wrap items-end gap-3">
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                <Clock className="h-3 w-3 inline mr-1" />
                {t('logs.from')}
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
                {t('logs.to')}
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
                {t('logs.apiKey')}
              </label>
              <select
                value={keyFilter}
                onChange={(e) => { setKeyFilter(e.target.value); setPage(1); }}
                className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              >
                <option value="">{t('logs.allKeys')}</option>
                {keys?.items?.map((k) => (
                  <option key={k.id} value={k.id}>{k.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                {t('logs.channel')}
              </label>
              <select
                value={channelFilter}
                onChange={(e) => { setChannelFilter(e.target.value); setPage(1); }}
                className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              >
                <option value="">{t('logs.allChannels')}</option>
                {channels?.map((ch) => (
                  <option key={ch.id} value={ch.id}>{ch.name}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                {t('logs.requestId')}
              </label>
              <input
                type="text"
                value={requestIdFilter}
                onChange={(e) => { setRequestIdFilter(e.target.value); setPage(1); }}
                placeholder={t('logs.requestIdPlaceholder')}
                className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
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
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.requestId')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.time')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.model')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.channel')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.key')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.protocol')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.stream')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.status')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.routes')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.latency')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.input')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.output')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.actions')}</th>
                  </tr>
                </thead>
                <tbody>
                  {data?.items?.length === 0 && (
                    <tr>
                      <td colSpan={13} className="text-center py-12 text-base-content/30">
                        <Search className="h-8 w-8 mx-auto mb-2 opacity-40" />
                        <div>{t('logs.empty.title')}</div>
                        {hasFilters && <div className="text-xs mt-1">{t('logs.empty.tryAdjusting')}</div>}
                      </td>
                    </tr>
                  )}
                  {data?.items?.map((log) => (
                    <tr
                      key={log.id}
                      className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors cursor-pointer"
                      onClick={() => handleView(log)}
                    >
                      <td className="mono text-xs text-base-content/55" onClick={(e) => e.stopPropagation()}>
                        {log.request_id ? (
                          <div className="flex items-center gap-1">
                            <span>{log.request_id.substring(0, 8)}</span>
                            <CopyButton value={log.request_id} />
                          </div>
                        ) : (
                          <span>-</span>
                        )}
                      </td>
                      <td className="mono text-[13px] text-base-content/55">
                        {new Date(log.created_at).toLocaleString()}
                      </td>
                      <td className="mono font-medium">{log.model_name}</td>
                      <td className="mono text-[13px] text-base-content/55">
                        {log.channel_name ?? '-'}
                      </td>
                      <td className="mono text-[13px] text-base-content/55">
                        <span className="px-1.5 py-0.5 rounded bg-base-200/80 text-base-content/70 font-medium">{log.key_id.slice(0, 8)}…</span>
                      </td>
                      <td>
                        <Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>
                          {log.protocol}
                        </Badge>
                      </td>
                      <td>
                        {log.stream
                          ? <Badge variant="blue">{t('logs.stream')}</Badge>
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
                      <td onClick={(e) => e.stopPropagation()}>
                        {log.routes && log.routes.length > 1 ? (
                          <button
                            className="badge badge-ghost cursor-pointer hover:badge-primary"
                            onClick={() => setRoutesModalLog(log)}
                          >
                            {t('logs.routesCount', { count: log.routes.length })}
                          </button>
                        ) : (
                          <span className="text-base-content/40">—</span>
                        )}
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
                          {t('logs.table.view')}
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
                {t('logs.total', { count: data?.total ?? 0 })}
              </span>
              <div className="join">
                <Button
                  variant="ghost"
                  size="sm"
                  icon={<ChevronLeft className="h-4 w-4" />}
                  disabled={page <= 1}
                  onClick={() => setPage(page - 1)}
                >
                  {t('usage.pagination.previous')}
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
                  {t('usage.pagination.next')}
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
        title={t('logs.detail.title')}
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
                  {t('logs.detail.requestDetails').toUpperCase()}
                </span>
              </div>
              <div className="p-4 grid grid-cols-2 sm:grid-cols-3 gap-3">
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.time')}</div>
                  <div className="mono text-[13px]">{new Date(selectedLog.created_at).toLocaleString()}</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.model')}</div>
                  <div className="mono text-[13px] font-medium">{selectedLog.model_name}</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.protocol')}</div>
                  <Badge variant={selectedLog.protocol === 'openai' ? 'blue' : 'purple'}>
                    {selectedLog.protocol}
                  </Badge>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.stream')}</div>
                  {selectedLog.stream
                    ? <Badge variant="blue">{t('logs.stream')}</Badge>
                    : <span className="text-base-content/30">-</span>}
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.status')}</div>
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
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.latency')}</div>
                  <div className="mono text-[13px]">{selectedLog.latency_ms}ms</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.tokens')}</div>
                  <div className="mono text-[13px]">
                    {t('logs.detail.tokenSummary', { input: selectedLog.input_tokens ?? 0, output: selectedLog.output_tokens ?? 0 })}
                  </div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.provider')}</div>
                  <div className="mono text-[13px] text-base-content/70">
                    {selectedLog.provider_id.slice(0, 8)}…
                  </div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.channel')}</div>
                  <div className="mono text-[13px] text-base-content/70">
                    {selectedLog.channel_name ?? '-'}
                  </div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('logs.detail.apiKey')}</div>
                  <div className="mono text-[13px] text-base-content/70">{selectedLog.key_id}</div>
                </div>
              </div>
            </div>

            {/* Routing info */}
            {(selectedLog.request_path || selectedLog.upstream_url || selectedLog.original_model || selectedLog.upstream_model) && (
              <div className="rounded-xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-4 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    {t('logs.detail.routing').toUpperCase()}
                  </span>
                </div>
                <div className="p-4 grid grid-cols-1 sm:grid-cols-2 gap-3">
                  {selectedLog.request_path && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">{t('logs.detail.requestPath')}</div>
                      <div className="mono text-[13px] font-medium">{selectedLog.request_path}</div>
                    </div>
                  )}
                  {selectedLog.upstream_url && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">{t('logs.detail.upstreamUrl')}</div>
                      <div className="mono text-[13px] text-wrap break-all">{selectedLog.upstream_url}</div>
                    </div>
                  )}
                  {selectedLog.original_model && selectedLog.original_model !== selectedLog.model_name && (
                    <>
                      <div>
                        <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">{t('logs.detail.originalModel')}</div>
                        <div className="mono text-[13px]">{selectedLog.original_model}</div>
                      </div>
                      <div>
                        <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">{t('logs.detail.upstreamModel')}</div>
                        <div className="mono text-[13px]">{selectedLog.upstream_model ?? selectedLog.model_name}</div>
                      </div>
                      {selectedLog.model_override_reason && (
                        <div className="sm:col-span-2">
                          <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-0.5">{t('logs.detail.overrideReason')}</div>
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
                    {t('logs.detail.headers').toUpperCase()}
                  </span>
                </div>
                <div className="p-4 grid grid-cols-1 sm:grid-cols-2 gap-4">
                  {selectedLog.request_headers && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-2">{t('logs.detail.request')}</div>
                      <JsonViewer data={selectedLog.request_headers} />
                    </div>
                  )}
                  {selectedLog.response_headers && (
                    <div>
                      <div className="text-[10px] text-base-content/40 uppercase tracking-wider mb-2">{t('logs.detail.response')}</div>
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
                  {t('logs.detail.requestBody').toUpperCase()}
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
                  {t('logs.detail.responseBody').toUpperCase()}
                </span>
              </div>
              <div className="p-4">
                <JsonViewer data={selectedLog.response_body} />
              </div>
            </div>
          </div>
        )}
      </Drawer>

      {/* Routes modal */}
      {routesModalLog && (
        <div className="modal modal-open">
          <div className="modal-box max-w-3xl">
            <h3 className="font-bold text-lg mb-4">
              {t('logs.routesModal.title')} — {routesModalLog.request_id?.slice(0, 8) ?? '?'}
            </h3>
            <div className="overflow-x-auto">
              <table className="table table-sm">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>{t('logs.routesModal.model')}</th>
                    <th>{t('logs.routesModal.channel')}</th>
                    <th>{t('logs.routesModal.status')}</th>
                    <th>{t('logs.routesModal.latency')}</th>
                    <th>{t('logs.routesModal.startedAt')}</th>
                    <th>{t('logs.routesModal.errorMessage')}</th>
                  </tr>
                </thead>
                <tbody>
                  {routesModalLog.routes?.map((r, i) => (
                    <tr key={i}>
                      <td className="text-base-content/40">{i + 1}</td>
                      <td className="font-mono text-xs">{r.model}</td>
                      <td>{r.channel_name ?? r.channel_id.slice(0, 8)}</td>
                      <td>
                        <span className={
                          r.status_code === 0
                            ? 'text-error'
                            : r.status_code < 400
                              ? 'text-success'
                              : r.status_code < 500
                                ? 'text-warning'
                                : 'text-error'
                        }>
                          {r.status_code === 0 ? 'CONN' : r.status_code}
                        </span>
                      </td>
                      <td>{r.latency_ms}ms</td>
                      <td className="font-mono text-xs text-base-content/55">
                        {r.started_at}
                      </td>
                      <td className="text-xs text-base-content/60 max-w-md truncate" title={r.error_message ?? ''}>
                        {r.error_message ?? '—'}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="modal-action">
              <button className="btn btn-sm" onClick={() => setRoutesModalLog(null)}>
                {t('logs.routesModal.close')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
