import { Copy, Check } from 'lucide-react';
import { useState } from 'react';

function CopyBtn({ text, label, copied, onCopy }: { text: string; label: string; copied: string | null; onCopy: (t: string, l: string) => void }) {
  const isCopied = copied === label;
  return (
    <button
      onClick={() => onCopy(text, label)}
      className="btn btn-ghost btn-xs gap-1"
    >
      {isCopied ? <Check className="h-3 w-3 text-success" /> : <Copy className="h-3 w-3" />}
      {isCopied ? 'Copied' : 'Copy'}
    </button>
  );
}

function CodeBlock({ title, code, copyLabel, copied, onCopy }: { title: string; code: string; copyLabel: string; copied: string | null; onCopy: (t: string, l: string) => void }) {
  return (
    <div className="mb-6">
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-base-content/70">{title}</span>
        <CopyBtn text={code} label={copyLabel} copied={copied} onCopy={onCopy} />
      </div>
      <pre className="bg-base-300/50 border border-base-300/60 rounded-lg p-4 text-sm font-mono overflow-x-auto leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  );
}

export default function GettingStarted() {
  const baseUrl = window.location.origin;
  const [copied, setCopied] = useState<string | null>(null);

  const onCopy = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    setCopied(label);
    setTimeout(() => setCopied(null), 2000);
  };

  const openaiCode = `import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'sk-your-api-key',
  baseURL: '${baseUrl}/v1',
});`;

  const anthropicCode = `import Anthropic from '@anthropic-ai/sdk';

const client = new Anthropic({
  apiKey: 'sk-your-api-key',
  baseURL: '${baseUrl}/v1',
});`;

  const curlCode = `curl ${baseUrl}/v1/chat/completions \\
  -H "Authorization: Bearer sk-your-api-key" \\
  -H "Content-Type: application/json" \\
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"Hello"}]}'`;

  return (
    <div className="not-prose space-y-8">
      {/* Title */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight mb-2">快速开始</h1>
        <p className="text-base-content/60">替换两项配置即可接入，兼容 OpenAI / Anthropic SDK。</p>
      </div>

      {/* Base URL card */}
      <div className="rounded-xl border border-base-300/60 bg-base-200/50 p-5">
        <div className="text-xs font-medium text-base-content/40 uppercase tracking-wider mb-2">你的 Base URL</div>
        <div className="flex items-center gap-3">
          <code className="text-base font-mono font-semibold text-primary flex-1 break-all">{baseUrl}</code>
          <CopyBtn text={baseUrl} label="base" copied={copied} onCopy={onCopy} />
        </div>
        <div className="mt-3 text-sm text-base-content/50">
          API 密钥在控制台「<a href="/console/keys" className="text-primary hover:underline">API 密钥</a>」页面创建
        </div>
      </div>

      {/* Config table */}
      <div>
        <h2 className="text-lg font-semibold mb-3">替换配置</h2>
        <div className="rounded-lg border border-base-300/60 overflow-hidden">
          <table className="w-full text-sm">
            <tbody>
              <tr className="border-b border-base-300/60">
                <td className="px-4 py-3 font-mono text-primary/80 bg-base-200/30 w-32">baseURL</td>
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <code className="text-sm">{baseUrl}/v1</code>
                    <CopyBtn text={`${baseUrl}/v1`} label="baseurl" copied={copied} onCopy={onCopy} />
                  </div>
                </td>
              </tr>
              <tr>
                <td className="px-4 py-3 font-mono text-primary/80 bg-base-200/30">apiKey</td>
                <td className="px-4 py-3">你创建的 API 密钥</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Code examples */}
      <div>
        <h2 className="text-lg font-semibold mb-4">代码示例</h2>
        <CodeBlock title="OpenAI SDK" code={openaiCode} copyLabel="openai" copied={copied} onCopy={onCopy} />
        <CodeBlock title="Anthropic SDK" code={anthropicCode} copyLabel="anthropic" copied={copied} onCopy={onCopy} />
        <CodeBlock title="cURL" code={curlCode} copyLabel="curl" copied={copied} onCopy={onCopy} />
      </div>

      {/* Endpoints */}
      <div>
        <h2 className="text-lg font-semibold mb-3">支持的端点</h2>
        <div className="rounded-lg border border-base-300/60 overflow-hidden">
          <table className="w-full text-sm">
            <tbody>
              <tr className="border-b border-base-300/60">
                <td className="px-4 py-2.5 font-mono text-xs">{`/v1/chat/completions`}</td>
                <td className="px-4 py-2.5 text-base-content/60">OpenAI Chat</td>
              </tr>
              <tr>
                <td className="px-4 py-2.5 font-mono text-xs">{`/v1/messages`}</td>
                <td className="px-4 py-2.5 text-base-content/60">Anthropic Messages</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}