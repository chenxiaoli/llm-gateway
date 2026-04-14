import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Globe, Cloud, KeyRound, Zap, ShieldCheck,
  LayoutDashboard, Sun, Moon, ArrowRight, Terminal,
} from 'lucide-react';
import { Button } from '../components/ui/Button';
import { apiClient, getToken } from '../api/client';
import { useTheme } from '../hooks/useTheme';
import { useSettings } from '../hooks/useSettings';

const features = [
  { icon: Globe, title: 'Dual Protocol', desc: 'OpenAI and Anthropic compatible endpoints. Drop-in replacement for your existing SDK.' },
  { icon: Cloud, title: 'Multi-Provider', desc: 'Route to multiple upstream LLM providers. Balance load, avoid vendor lock-in.' },
  { icon: KeyRound, title: 'Key Management', desc: 'Create, rotate, and revoke keys with per-key rate limits and monthly budgets.' },
  { icon: Zap, title: 'Rate Limiting', desc: 'Per-key and per-model RPM/TPM limits with in-memory sliding window enforcement.' },
  { icon: ShieldCheck, title: 'Audit Logging', desc: 'Full request logging with latency, token counts, and cost tracking.' },
  { icon: LayoutDashboard, title: 'Web Dashboard', desc: 'Management UI for keys, providers, models, usage analytics, and logs.' },
];

type Protocol = 'openai' | 'anthropic';

export default function Home() {
  const navigate = useNavigate();
  const [version, setVersion] = useState('');
  const [activeProtocol, setActiveProtocol] = useState<Protocol>('openai');
  const { theme, toggleTheme } = useTheme();
  const { data: settings } = useSettings();

  const serverHost = settings?.server_host || 'http://localhost:8080';

  const activeExample = activeProtocol === 'openai'
    ? {
        title: 'OpenAI Compatible',
        curl: [
          { text: `curl -X POST ${serverHost}/v1/chat/completions \\`, prefix: '$' },
          { text: '  -H "Authorization: Bearer sk-xxxx" \\', prefix: '' },
          { text: '  -H "Content-Type: application/json" \\', prefix: '' },
          { text: '  -d \'{"model": "gpt-4o", "messages": [{"role": "user", "content": "Hello"}]}\'', prefix: '' },
        ],
        sdk: `
// OpenAI SDK
import OpenAI from 'openai';
const client = new OpenAI({
  apiKey: 'sk-xxxx',
  baseURL: '${serverHost}/v1',
});
const chat = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: [{ role: 'user', content: 'Hello' }],
});`,
      }
    : {
        title: 'Anthropic Compatible',
        curl: [
          { text: `curl -X POST ${serverHost}/v1/messages \\`, prefix: '$' },
          { text: '  -H "Authorization: Bearer sk-xxxx" \\', prefix: '' },
          { text: '  -H "anthropic-version: 2023-06-01" \\', prefix: '' },
          { text: '  -H "Content-Type: application/json" \\', prefix: '' },
          { text: '  -d \'{"model": "claude-3-5-sonnet-20241022", "max_tokens": 100, "messages": [{"role": "user", "content": "Hello"}]}\'', prefix: '' },
        ],
        sdk: `
// Anthropic SDK
import Anthropic from '@anthropic-ai/sdk';
const client = new Anthropic({
  apiKey: 'sk-xxxx',
  baseURL: '${serverHost}/v1',
});
const message = await client.messages.create({
  model: 'claude-3-5-sonnet-20241022',
  max_tokens: 100,
  messages: [{ role: 'user', content: 'Hello' }],
});`,
      };

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div className="min-h-screen bg-base-200 relative overflow-hidden">
      {/* ── Background layers ── */}
      {/* Dot grid */}
      <div
        className="absolute inset-0 pointer-events-none text-base-content"
        style={{
          backgroundImage: 'radial-gradient(circle, currentColor 1px, transparent 1px)',
          backgroundSize: '32px 32px',
          opacity: 0.03,
        }}
      />
      {/* Primary glow */}
      <div
        className="absolute top-[12%] left-1/2 -translate-x-1/2 w-[700px] h-[420px] rounded-full pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse, rgba(6, 214, 160, 0.07), transparent 70%)',
          animation: 'pulse 8s ease-in-out infinite',
        }}
      />
      {/* Secondary glow offset */}
      <div
        className="absolute top-[40%] left-[30%] w-[400px] h-[300px] rounded-full pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse, rgba(6, 214, 160, 0.03), transparent 70%)',
          animation: 'pulse 12s ease-in-out infinite reverse',
        }}
      />

      {/* ── Content ── */}
      <div className="relative z-10">

        {/* Theme toggle */}
        <button
          className="btn btn-ghost btn-sm btn-circle fixed top-4 right-4 z-50"
          onClick={toggleTheme}
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </button>

        {/* ── Hero ── */}
        <div className="flex flex-col items-center justify-center min-h-[82vh] text-center px-6">
          {/* Badge */}
          <div className="animate-fade-in-up">
            <div className="inline-flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-base-300/60 bg-base-100/50 backdrop-blur-sm mb-10">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-50" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-primary" />
              </span>
              <span className="text-xs font-mono text-base-content/40">Open Source API Gateway</span>
            </div>
          </div>

          {/* Title */}
          <h1
            className="text-5xl sm:text-6xl lg:text-7xl font-bold tracking-tight animate-fade-in-up"
            style={{ animationDelay: '80ms' }}
          >
            LLM{' '}
            <span className="text-primary">Gateway</span>
          </h1>

          {/* Subtitle */}
          <p
            className="mt-6 text-lg text-base-content/40 max-w-lg leading-relaxed animate-fade-in-up"
            style={{ animationDelay: '160ms' }}
          >
            A unified API gateway for LLM providers. Manage keys, enforce rate limits,
            track usage, and proxy requests from a single endpoint.
          </p>

          {/* CTAs */}
          <div
            className="flex items-center gap-3 mt-10 animate-fade-in-up"
            style={{ animationDelay: '260ms' }}
          >
            <Button
              variant="primary"
              size="lg"
              icon={<ArrowRight className="h-4 w-4" />}
              onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}
            >
              Go to Dashboard
            </Button>
            <Button
              variant="secondary"
              size="lg"
              icon={
                <svg className="h-4 w-4" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
                </svg>
              }
              onClick={() => window.open('https://github.com/chenxiaoli/llm-gateway', '_blank')}
            >
              GitHub
            </Button>
          </div>
        </div>

        {/* ── Divider ── */}
        <div className="h-px bg-gradient-to-r from-transparent via-base-content/10 to-transparent" />

        {/* ── Features ── */}
        <div className="max-w-5xl mx-auto px-6 py-24">
          <h2 className="text-xs font-semibold uppercase tracking-[0.2em] text-base-content/25 text-center mb-14">
            Features
          </h2>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {features.map((f) => {
              const Icon = f.icon;
              return (
                <div
                  key={f.title}
                  className="group rounded-xl border border-base-300/50 bg-base-100/30 p-6 transition-all duration-300 hover:border-primary/25 hover:bg-base-100/70 hover:shadow-lg hover:shadow-primary/5"
                >
                  <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary mb-4 transition-colors duration-300 group-hover:bg-primary/20">
                    <Icon className="h-[18px] w-[18px]" strokeWidth={1.8} />
                  </div>
                  <h3 className="text-sm font-semibold tracking-tight mb-1.5">{f.title}</h3>
                  <p className="text-[13px] text-base-content/40 leading-relaxed">{f.desc}</p>
                </div>
              );
            })}
          </div>
        </div>

        {/* ── Divider ── */}
        <div className="h-px bg-gradient-to-r from-transparent via-base-content/10 to-transparent" />

        {/* ── Quick Start ── */}
        <div className="max-w-3xl mx-auto px-6 py-24">
          <h2 className="text-xs font-semibold uppercase tracking-[0.2em] text-base-content/25 text-center mb-14">
            Quick Start
          </h2>

          <div className="rounded-xl border border-base-300/50 bg-base-100/30 overflow-hidden shadow-lg shadow-black/5">
            {/* Protocol tabs */}
            <div className="flex items-center gap-1 px-2 py-2 border-b border-base-300/50 bg-base-200/40">
              {(['openai', 'anthropic'] as Protocol[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setActiveProtocol(p)}
                  className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
                    activeProtocol === p
                      ? 'bg-base-100 text-base-content shadow-sm'
                      : 'text-base-content/40 hover:text-base-content/60 hover:bg-base-100/50'
                  }`}
                >
                  {p === 'openai' ? 'OpenAI Compatible' : 'Anthropic Compatible'}
                </button>
              ))}
            </div>

            {/* Terminal body */}
            <div className="p-5">
              <div className="flex items-center gap-2 mb-4">
                <Terminal className="h-4 w-4 text-base-content/30" />
                <span className="text-[11px] font-mono text-base-content/40">cURL</span>
              </div>
              <div className="font-mono text-[13px] leading-7 mb-6">
                {activeExample.curl.map((line, i) => (
                  <div key={i}>
                    {line.prefix && <span className="text-primary mr-2">{line.prefix}</span>}
                    {line.text}
                  </div>
                ))}
              </div>

              <div className="flex items-center gap-2 mb-4">
                <Terminal className="h-4 w-4 text-base-content/30" />
                <span className="text-[11px] font-mono text-base-content/40">SDK</span>
              </div>
              <pre className="font-mono text-[12px] leading-6 text-base-content/60 bg-base-200/30 p-4 rounded-lg overflow-x-auto">
                <code>{activeExample.sdk}</code>
              </pre>
            </div>
          </div>

          <p className="mt-6 text-center text-sm text-base-content/40">
            Create an API key in the dashboard, then use it as a drop-in replacement
          </p>
        </div>

        {/* ── Footer ── */}
        <div className="h-px bg-gradient-to-r from-transparent via-base-content/10 to-transparent" />
        <div className="py-8 text-center text-xs text-base-content/50 font-mono tracking-wide">
          LLM Gateway{version ? ` v${version}` : ''} — Open Source
        </div>
      </div>
    </div>
  );
}
