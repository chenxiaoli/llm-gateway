import { useState } from 'react';
import { ChevronDown, ChevronRight } from 'lucide-react';

interface JsonViewerProps {
  data: string;
  maxHeight?: number;
}

export default function JsonViewer({ data, maxHeight = 400 }: JsonViewerProps) {
  const [collapsed, setCollapsed] = useState(true);

  let formatted: string;
  try {
    formatted = JSON.stringify(JSON.parse(data), null, 2);
  } catch {
    formatted = data;
  }

  return (
    <div>
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="btn btn-ghost btn-xs gap-1 mb-2 text-base-content/50"
      >
        {collapsed ? <ChevronRight className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
        {collapsed ? 'Expand' : 'Collapse'}
      </button>
      <pre
        className="mono text-[12px] leading-relaxed bg-base-300/50 text-base-content/80 rounded-lg p-3 overflow-auto whitespace-pre-wrap break-all border border-base-300"
        style={{ maxHeight: collapsed ? 100 : maxHeight }}
      >
        {formatted}
      </pre>
    </div>
  );
}
