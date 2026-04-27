import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  ArrowRight, Terminal, Zap, Shield, BarChart3,
  Sun, Moon, ChevronRight, Layers,
} from 'lucide-react';
import { Button } from '../components/ui/Button';
import { apiClient, getToken } from '../api/client';
import { useTheme } from '../hooks/useTheme';
import { useSettings } from '../hooks/useSettings';

type Protocol = 'openai' | 'anthropic';

const steps = [
  {
    icon: Layers,
    title: 'Configure Providers',
    desc: 'Add upstream LLM providers and set up channels with API keys, models, and routing rules.',
  },
  {
    icon: Zap,
    title: 'Route Requests',
    desc: 'Send requests through a single endpoint. The gateway handles protocol translation and load balancing.',
  },
  {
    icon: BarChart3,
    title: 'Monitor Usage',
    desc: 'Track costs, latency, and token usage per key. Full audit logs for every request.',
  },
];

const codeExamples: Record<Protocol, { curl: { lines: string[]; prompt: string }[]; sdk: string }> = {
  openai: {
    curl: [
      { lines: ['curl -X POST /v1/chat/completions \\', '  -H "Authorization: Bearer sk-xxxx" \\', '  -H "Content-Type: application/json" \\', '  -d \'{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]}\''], prompt: '$' },
    ],
    sdk: `import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'sk-xxxx',
  baseURL: 'http://localhost:8080/v1',
});

const chat = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: [{ role: 'user', content: 'Hello' }],
});`,
  },
  anthropic: {
    curl: [
      { lines: ['curl -X POST /v1/messages \\', '  -H "Authorization: Bearer sk-xxxx" \\', '  -H "anthropic-version: 2023-06-01" \\', '  -H "Content-Type: application/json" \\', '  -d \'{"model":"claude-3-5-sonnet-20241022","max_tokens":100,"messages":[{"role":"user","content":"Hello"}]}\''], prompt: '$' },
    ],
    sdk: `import Anthropic from '@anthropic-ai/sdk';

const client = new Anthropic({
  apiKey: 'sk-xxxx',
  baseURL: 'http://localhost:8080/v1',
});

const message = await client.messages.create({
  model: 'claude-3-5-sonnet-20241022',
  max_tokens: 100,
  messages: [{ role: 'user', content: 'Hello' }],
});`,
  },
};

export default function Home() {
  const navigate = useNavigate();
  const [version, setVersion] = useState('');
  const [activeProtocol, setActiveProtocol] = useState<Protocol>('openai');
  const { theme, toggleTheme } = useTheme();
  const { data: settings } = useSettings();

  const serverHost = settings?.server_host || 'http://localhost:8080';

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div className="min-h-screen bg-base-200 relative">
      {/* Nav */}
      <header className="fixed top-0 inset-x-0 z-50 border-b border-base-300/40 bg-base-200/80 backdrop-blur-md">
        <div className="max-w-6xl mx-auto h-14 flex items-center justify-between px-6">
          <div className="flex items-center gap-2.5">
            <div className="h-8 w-8 rounded-lg bg-primary flex items-center justify-center text-primary-content font-bold text-sm">GW</div>
            <span className="font-semibold text-lg">LLM Gateway</span>
          </div>
          <div className="flex items-center gap-2">
            <button
              className="btn btn-ghost btn-sm btn-circle"
              onClick={toggleTheme}
              aria-label="Toggle theme"
            >
              {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}
            >
              Dashboard
            </Button>
          </div>
        </div>
      </header>

      {/* Hero */}
      <section className="pt-32 pb-20 px-6">
        <div className="max-w-6xl mx-auto">
          <div className="max-w-2xl">
            <h1 className="text-5xl sm:text-6xl font-bold tracking-tight leading-[1.1] mb-6">
              One API endpoint
              <br />
              <span className="text-primary">every LLM provider</span>
            </h1>
            <p className="text-lg text-base-content/55 leading-relaxed mb-8 max-w-lg">
              A unified gateway that manages keys, enforces rate limits, tracks costs, and proxies
              requests to OpenAI, Anthropic, and more through a single endpoint.
            </p>
            <div className="flex items-center gap-3">
              <Button
                variant="primary"
                size="lg"
                icon={<ArrowRight className="h-4 w-4" />}
                onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}
              >
                Get Started
              </Button>
              <Button
                variant="ghost"
                size="lg"
                onClick={() => document.getElementById('quickstart')?.scrollIntoView({ behavior: 'smooth' })}
              >
                See how it works
              </Button>
            </div>
          </div>
        </div>
      </section>

      {/* How it works */}
      <section className="py-20 px-6">
        <div className="max-w-6xl mx-auto">
          <h2 className="text-base font-semibold uppercase tracking-widest text-base-content/40 text-center mb-16">
            How it works
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            {steps.map((step, i) => {
              const Icon = step.icon;
              return (
                <div key={step.title} className="relative">
                  <div className="flex items-center gap-3 mb-4">
                    <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center shrink-0">
                      <Icon className="h-5 w-5 text-primary" />
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-mono text-base-content/30">{String(i + 1).padStart(2, '0')}</span>
                      <h3 className="text-lg font-semibold">{step.title}</h3>
                    </div>
                  </div>
                  <p className="text-base text-base-content/55 leading-relaxed pl-[52px]">{step.desc}</p>
                  {i < steps.length - 1 && (
                    <div className="hidden md:block absolute top-5 -right-4 w-8">
                      <ChevronRight className="h-4 w-4 text-base-content/20" />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </section>

      {/* Quick Start */}
      <section id="quickstart" className="py-20 px-6">
        <div className="max-w-6xl mx-auto">
          <h2 className="text-base font-semibold uppercase tracking-widest text-base-content/40 text-center mb-16">
            Quick Start
          </h2>
          <div className="max-w-3xl mx-auto">
            {/* Protocol tabs */}
            <div className="flex items-center gap-1 mb-4">
              {(['openai', 'anthropic'] as Protocol[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setActiveProtocol(p)}
                  className={`px-4 py-2 rounded-lg text-base font-medium transition-all cursor-pointer ${
                    activeProtocol === p
                      ? 'bg-base-100 text-base-content shadow-sm border border-base-300/50'
                      : 'text-base-content/50 hover:text-base-content/70'
                  }`}
                >
                  {p === 'openai' ? 'OpenAI SDK' : 'Anthropic SDK'}
                </button>
              ))}
            </div>

            {/* Code block */}
            <div className="rounded-2xl border border-base-300/50 bg-base-100 overflow-hidden">
              {/* Window chrome */}
              <div className="flex items-center gap-3 px-5 py-3 border-b border-base-300/40 bg-base-200/30">
                <div className="flex gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-base-300/60" />
                  <div className="w-3 h-3 rounded-full bg-base-300/60" />
                  <div className="w-3 h-3 rounded-full bg-base-300/60" />
                </div>
                <span className="text-xs font-mono text-base-content/40">
                  {activeProtocol === 'openai' ? 'openai-client.ts' : 'anthropic-client.ts'}
                </span>
              </div>

              {/* cURL */}
              <div className="px-5 py-4 border-b border-base-300/30">
                <div className="flex items-center gap-2 mb-3">
                  <Terminal className="h-3.5 w-3.5 text-base-content/30" />
                  <span className="text-xs font-mono text-base-content/40 uppercase tracking-wider">cURL</span>
                </div>
                <div className="font-mono text-sm leading-relaxed">
                  {codeExamples[activeProtocol].curl.map((block, i) => (
                    <div key={i}>
                      {block.prompt && <span className="text-primary mr-2 select-none">{block.prompt}</span>}
                      <span>curl -X POST {serverHost}{block.lines[0].replace('/v1/chat/completions', '').replace('/v1/messages', '')}<wbr />{block.lines[0]}</span>
                      {block.lines.slice(1).map((line, j) => (
                        <div key={j}>{line}</div>
                      ))}
                    </div>
                  ))}
                </div>
              </div>

              {/* SDK */}
              <div className="px-5 py-4">
                <div className="flex items-center gap-2 mb-3">
                  <Shield className="h-3.5 w-3.5 text-base-content/30" />
                  <span className="text-xs font-mono text-base-content/40 uppercase tracking-wider">
                    {activeProtocol === 'openai' ? 'OpenAI SDK' : 'Anthropic SDK'}
                  </span>
                </div>
                <pre className="font-mono text-sm leading-relaxed text-base-content/60 overflow-x-auto">
                  <code>{codeExamples[activeProtocol].sdk}</code>
                </pre>
              </div>
            </div>

            <p className="mt-6 text-center text-base text-base-content/50">
              Drop in your existing SDK — just change the <code className="font-mono text-base-content/70 bg-base-300/30 px-1.5 py-0.5 rounded">baseURL</code>
            </p>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-20 px-6">
        <div className="max-w-6xl mx-auto text-center">
          <div className="rounded-2xl border border-base-300/50 bg-base-100 p-12">
            <h2 className="text-3xl font-bold mb-4">Ready to get started?</h2>
            <p className="text-base text-base-content/55 mb-8 max-w-md mx-auto">
              Set up your gateway in minutes. Configure providers, create API keys, and start routing requests.
            </p>
            <div className="flex items-center justify-center gap-3">
              <Button
                variant="primary"
                size="lg"
                icon={<ArrowRight className="h-4 w-4" />}
                onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}
              >
                Go to Dashboard
              </Button>
              <Button
                variant="ghost"
                size="lg"
                onClick={() => window.open('https://github.com/chenxiaoli/llm-gateway', '_blank')}
                icon={
                  <svg className="h-4 w-4" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
                  </svg>
                }
              >
                Star on GitHub
              </Button>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-base-300/40 py-8 px-6">
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <span className="text-sm text-base-content/40 font-mono">
            LLM Gateway{version ? ` v${version}` : ''}
          </span>
          <span className="text-sm text-base-content/30">Open Source</span>
        </div>
      </footer>
    </div>
  );
}
