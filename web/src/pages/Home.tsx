import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Globe, Cloud, KeyRound, Zap, ShieldCheck, LayoutDashboard, Sun, Moon } from 'lucide-react';
import { Button } from '../components/ui/Button';
import { apiClient, getToken } from '../api/client';
import { useTheme } from '../hooks/useTheme';

const features = [
  { icon: Globe, title: 'Dual Protocol', desc: 'OpenAI and Anthropic compatible API endpoints. Drop-in replacement for your existing SDK.' },
  { icon: Cloud, title: 'Multi-Provider', desc: 'Route to multiple upstream LLM providers. Balance load, avoid vendor lock-in.' },
  { icon: KeyRound, title: 'API Key Management', desc: 'Create, rotate, and revoke keys. Per-key rate limits and monthly budgets.' },
  { icon: Zap, title: 'Rate Limiting', desc: 'Per-key and per-model RPM/TPM limits. In-memory sliding window enforcement.' },
  { icon: ShieldCheck, title: 'Audit Logging', desc: 'Full request logging with latency, tokens, and cost tracking. Retention policies.' },
  { icon: LayoutDashboard, title: 'Web Dashboard', desc: 'Management UI for keys, providers, models, usage analytics, and audit logs.' },
];

export default function Home() {
  const navigate = useNavigate();
  const [version, setVersion] = useState('');
  const { theme, toggleTheme } = useTheme();

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div className="min-h-screen bg-base-200 relative">
      {/* Theme toggle */}
      <button
        className="btn btn-ghost btn-sm btn-circle fixed top-4 right-4 z-50"
        onClick={toggleTheme}
        aria-label="Toggle theme"
      >
        {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
      </button>

      {/* Hero */}
      <div className="hero min-h-[70vh]">
        <div className="hero-content text-center">
          <div className="max-w-2xl">
            <h1 className="text-5xl font-extrabold">LLM Gateway</h1>
            <p className="py-6 text-base-content/60 text-lg">
              A unified API gateway for LLM providers. Manage keys, enforce rate limits,
              track usage, and proxy requests to OpenAI and Anthropic from a single endpoint.
            </p>
            <div className="flex items-center justify-center gap-4">
              <Button variant="primary" size="lg" onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}>
                Go to Dashboard
              </Button>
              <Button variant="secondary" size="lg" onClick={() => window.open('https://github.com/chenxiaoli/llm-gateway', '_blank')}>
                GitHub
              </Button>
            </div>
          </div>
        </div>
      </div>

      {/* Features */}
      <div className="max-w-5xl mx-auto px-6 pb-20">
        <div className="divider text-xs font-semibold uppercase tracking-[0.15em] text-base-content/30">Features</div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5 mt-8">
          {features.map((f) => {
            const Icon = f.icon;
            return (
              <div key={f.title} className="card bg-base-100 shadow-sm hover:shadow-md transition-shadow">
                <div className="card-body items-center text-center">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 mb-2">
                    <Icon className="h-5 w-5 text-primary" />
                  </div>
                  <h3 className="text-[15px] font-semibold tracking-tight text-base-content/90">{f.title}</h3>
                  <p className="text-[13px] text-base-content/55 leading-relaxed mt-1">{f.desc}</p>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Quick start */}
      <div className="max-w-3xl mx-auto px-6 pb-20">
        <div className="divider text-xs font-semibold uppercase tracking-[0.15em] text-base-content/30">Quick Start</div>
        <div className="mt-8 bg-base-100 rounded-box p-6 shadow-sm">
          <div className="font-mono text-sm leading-8">
            <div className="text-base-content/40"># Create an API key via the management dashboard</div>
            <div className="text-base-content/40"># Then use it as a drop-in replacement:</div>
            <div className="h-4" />
            <div>curl -X POST http://localhost:8080/v1/chat/completions \</div>
            <div>&nbsp;&nbsp;-H "Authorization: Bearer <span className="text-primary">your-api-key</span>" \</div>
            <div>&nbsp;&nbsp;-H "Content-Type: application/json" \</div>
            <div>&nbsp;&nbsp;-d {'\'{{"model": "gpt-4", "messages": [{{"role": "user", "content": "Hello"}}]}}\''}</div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="py-8 text-center text-sm text-base-content/30 font-mono">
        LLM Gateway{version ? ` v${version}` : ''} &mdash; Open Source
      </div>
    </div>
  );
}
