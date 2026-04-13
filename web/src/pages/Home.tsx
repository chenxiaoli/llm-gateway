import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Globe, Cloud, KeyRound, Zap, ShieldCheck, LayoutDashboard, ArrowRight } from 'lucide-react';
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
    <div className="min-h-screen bg-black relative overflow-hidden">
      {/* Background atmosphere */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-[radial-gradient(ellipse_at_center,rgba(6,214,160,0.05)_0%,transparent_60%)]" />
        <div className="absolute bottom-0 left-0 w-[400px] h-[400px] bg-[radial-gradient(ellipse_at_center,rgba(59,130,246,0.03)_0%,transparent_60%)]" />
        <div className="absolute bottom-0 right-0 w-[400px] h-[400px] bg-[radial-gradient(ellipse_at_center,rgba(168,85,247,0.03)_0%,transparent_60%)]" />
      </div>

      {/* Grid pattern */}
      <div className="fixed inset-0 bg-[linear-gradient(#1e1e1e_1px,transparent_1px),linear-gradient(90deg,#1e1e1e_1px,transparent_1px)] bg-[size:60px_60px] opacity-20 [mask-image:radial-gradient(ellipse_at_center,black_20%,transparent_70%)] pointer-events-none" />

      {/* Hero */}
      <div className="relative mx-auto max-w-[900px] px-6 pt-24 pb-16 text-center">
        <div className="animate-fade-in-up">
          <div className="inline-flex items-center gap-2 rounded-full px-3 py-1 mb-6 text-[11px] font-medium" style={{ background: 'rgba(6, 214, 160, 0.06)', color: '#06d6a0', border: '1px solid rgba(6, 214, 160, 0.12)' }}>
            <div className="pulse-dot h-1.5 w-1.5 rounded-full bg-accent" />
            Open Source LLM Gateway
          </div>
        </div>

        <h1 className="font-display text-5xl font-extrabold text-[#ededed] mb-4 animate-fade-in-up" style={{ animationDelay: '80ms' }}>
          Route. Rate Limit.
          <br />
          <span style={{ background: 'linear-gradient(135deg, #06d6a0 0%, #3b82f6 50%, #a855f7 100%)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
            Observe.
          </span>
        </h1>
        <p className="mx-auto max-w-[560px] text-base text-[#666666] mb-10 leading-relaxed animate-fade-in-up" style={{ animationDelay: '160ms' }}>
          A unified API gateway for LLM providers. Manage keys, enforce rate limits,
          track usage, and proxy requests to OpenAI and Anthropic from a single endpoint.
        </p>
        <div className="flex items-center justify-center gap-4 animate-fade-in-up" style={{ animationDelay: '240ms' }}>
          <Button
            variant="primary"
            size="lg"
            onClick={() => navigate('/console/login')}
            className="gap-2"
          >
            Go to Dashboard
            <ArrowRight className="h-4 w-4" />
          </Button>
          <Button
            variant="secondary"
            size="lg"
            onClick={() => window.open('https://github.com/chenxiaoli/llm-gateway', '_blank')}
          >
            GitHub
          </Button>
        </div>
      </div>

      {/* Features */}
      <div className="relative mx-auto max-w-[1000px] px-6 pb-20">
        <div className="flex items-center justify-center mb-10 animate-fade-in-up" style={{ animationDelay: '300ms' }}>
          <div className="flex items-center gap-4 w-full max-w-md">
            <div className="flex-1 h-px" style={{ background: 'linear-gradient(90deg, transparent, rgba(30, 30, 30, 0.8))' }} />
            <span className="text-[10px] font-semibold uppercase tracking-[0.15em] text-[#444444]">Features</span>
            <div className="flex-1 h-px" style={{ background: 'linear-gradient(90deg, rgba(30, 30, 30, 0.8), transparent)' }} />
          </div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {features.map((f, i) => {
            const Icon = f.icon;
            return (
              <div
                key={f.title}
                className="group h-full rounded-xl p-6 transition-all duration-300 animate-fade-in-up"
                style={{
                  animationDelay: `${350 + i * 60}ms`,
                  background: '#0a0a0a',
                  border: '1px solid rgba(30, 30, 30, 0.8)',
                }}
                onMouseEnter={(e) => {
                  const el = e.currentTarget as HTMLElement;
                  el.style.borderColor = `${f.accent}30`;
                  el.style.background = `${f.accent}08`;
                  el.style.transform = 'translateY(-2px)';
                  el.style.boxShadow = `0 8px 32px -8px ${f.accent}15`;
                }}
                onMouseLeave={(e) => {
                  const el = e.currentTarget as HTMLElement;
                  el.style.borderColor = 'rgba(30, 30, 30, 0.8)';
                  el.style.background = '#0a0a0a';
                  el.style.transform = 'translateY(0)';
                  el.style.boxShadow = 'none';
                }}
              >
                <div className="mb-4 flex justify-center">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg" style={{ background: `${f.accent}12` }}>
                    <Icon className="h-5 w-5" style={{ color: f.accent }} />
                  </div>
                </div>
                <h3 className="font-display font-semibold text-[#ededed] mb-2 text-center">{f.title}</h3>
                <p className="text-sm text-[#555555] leading-relaxed text-center">{f.desc}</p>
              </div>
            );
          })}
        </div>
      </div>

      {/* Quick start */}
      <div className="relative mx-auto max-w-[700px] px-6 pb-20">
        <div className="flex items-center justify-center mb-10 animate-fade-in-up" style={{ animationDelay: '700ms' }}>
          <div className="flex items-center gap-4 w-full max-w-md">
            <div className="flex-1 h-px" style={{ background: 'linear-gradient(90deg, transparent, rgba(30, 30, 30, 0.8))' }} />
            <span className="text-[10px] font-semibold uppercase tracking-[0.15em] text-[#444444]">Quick Start</span>
            <div className="flex-1 h-px" style={{ background: 'linear-gradient(90deg, rgba(30, 30, 30, 0.8), transparent)' }} />
          </div>
        </div>
        <div
          className="rounded-xl p-6 animate-fade-in-up"
          style={{
            animationDelay: '750ms',
            background: '#0a0a0a',
            border: '1px solid rgba(30, 30, 30, 0.8)',
          }}
        >
          <div className="font-mono text-sm leading-8">
            <div className="text-[#444444]"># Create an API key via the management dashboard</div>
            <div className="text-[#444444]"># Then use it as a drop-in replacement:</div>
            <div className="h-4" />
            <div className="text-[#ededed]">curl -X POST http://localhost:8080/v1/chat/completions \</div>
            <div className="text-[#ededed]">&nbsp;&nbsp;-H "Authorization: Bearer <span style={{ color: '#06d6a0' }}>your-api-key</span>" \</div>
            <div className="text-[#ededed]">&nbsp;&nbsp;-H "Content-Type: application/json" \</div>
            <div className="text-[#ededed]">&nbsp;&nbsp;-d {'\'{{"model": "gpt-4", "messages": [{{"role": "user", "content": "Hello"}}]}}\''}</div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="relative py-8 text-center text-sm text-[#333333] font-mono">
        LLM Gateway{version ? ` v${version}` : ''} &mdash; Open Source
      </div>
    </div>
  );
}
