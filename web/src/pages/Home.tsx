import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Globe, Cloud, KeyRound, Zap, ShieldCheck, LayoutDashboard } from 'lucide-react';
import { Button } from '../components/ui/Button';
import { apiClient } from '../api/client';

const features = [
  {
    icon: Globe,
    accent: '#06d6a0',
    title: 'Dual Protocol',
    desc: 'OpenAI and Anthropic compatible API endpoints. Drop-in replacement for your existing SDK.',
  },
  {
    icon: Cloud,
    accent: '#3b82f6',
    title: 'Multi-Provider',
    desc: 'Route to multiple upstream LLM providers. Balance load, avoid vendor lock-in.',
  },
  {
    icon: KeyRound,
    accent: '#f59e0b',
    title: 'API Key Management',
    desc: 'Create, rotate, and revoke keys. Per-key rate limits and monthly budgets.',
  },
  {
    icon: Zap,
    accent: '#ef4444',
    title: 'Rate Limiting',
    desc: 'Per-key and per-model RPM/TPM limits. In-memory sliding window enforcement.',
  },
  {
    icon: ShieldCheck,
    accent: '#a855f7',
    title: 'Audit Logging',
    desc: 'Full request logging with latency, tokens, and cost tracking. Retention policies.',
  },
  {
    icon: LayoutDashboard,
    accent: '#06b6d4',
    title: 'Web Dashboard',
    desc: 'Management UI for keys, providers, models, usage analytics, and audit logs.',
  },
];

export default function Home() {
  const navigate = useNavigate();
  const [version, setVersion] = useState('');

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div className="min-h-screen bg-black relative">
      {/* Hero */}
      <div className="mx-auto max-w-[900px] px-6 py-20 text-center">
        <h1 className="font-display text-5xl font-extrabold text-[#ededed] mb-4">LLM Gateway</h1>
        <p className="mx-auto max-w-[600px] text-lg text-[#888888] mb-8">
          A unified API gateway for LLM providers. Manage keys, enforce rate limits,
          track usage, and proxy requests to OpenAI and Anthropic from a single endpoint.
        </p>
        <div className="flex items-center justify-center gap-4">
          <Button variant="primary" size="lg" onClick={() => navigate('/console/login')}>
            Go to Dashboard
          </Button>
          <Button variant="secondary" size="lg" onClick={() => window.open('https://github.com/chenxiaoli/llm-gateway', '_blank')}>
            GitHub
          </Button>
        </div>
      </div>

      {/* Features */}
      <div className="mx-auto max-w-[1000px] px-6 pb-20">
        <div className="flex items-center justify-center mb-10">
          <div className="flex items-center gap-4 w-full">
            <hr className="flex-1 border-[#1e1e1e]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.1em] text-[#555555]">Features</span>
            <hr className="flex-1 border-[#1e1e1e]" />
          </div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
          {features.map((f) => {
            const Icon = f.icon;
            return (
              <div key={f.title} className="h-full rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-7 text-center">
                <div className="mb-4 flex justify-center">
                  <Icon className="h-7 w-7" style={{ color: f.accent }} />
                </div>
                <h3 className="font-display font-semibold text-[#ededed] mb-2">{f.title}</h3>
                <p className="text-sm text-[#555555] leading-relaxed">{f.desc}</p>
              </div>
            );
          })}
        </div>
      </div>

      {/* Quick start */}
      <div className="mx-auto max-w-[700px] px-6 pb-20">
        <div className="flex items-center justify-center mb-10">
          <div className="flex items-center gap-4 w-full">
            <hr className="flex-1 border-[#1e1e1e]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.1em] text-[#555555]">Quick Start</span>
            <hr className="flex-1 border-[#1e1e1e]" />
          </div>
        </div>
        <div className="rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-6">
          <div className="font-mono text-sm leading-8">
            <div className="text-[#555555]"># Create an API key via the management dashboard</div>
            <div className="text-[#555555]"># Then use it as a drop-in replacement:</div>
            <br />
            <div className="text-[#ededed]">curl -X POST http://localhost:8080/v1/chat/completions \</div>
            <div className="text-[#ededed]">&nbsp;&nbsp;-H "Authorization: Bearer <span className="text-accent">your-api-key</span>" \</div>
            <div className="text-[#ededed]">&nbsp;&nbsp;-H "Content-Type: application/json" \</div>
            <div className="text-[#ededed]">&nbsp;&nbsp;-d {'\'{{"model": "gpt-4", "messages": [{{"role": "user", "content": "Hello"}}]}}\''}</div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="py-6 text-center text-sm text-[#555555]">
        LLM Gateway{version ? ` ${version}` : ''} &mdash; Open Source
      </div>
    </div>
  );
}
