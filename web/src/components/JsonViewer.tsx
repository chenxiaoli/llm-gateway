import { useState } from 'react';

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
      <a onClick={() => setCollapsed(!collapsed)} style={{ marginBottom: 8, cursor: 'pointer' }}>
        {collapsed ? 'Expand' : 'Collapse'}
      </a>
      <pre style={{
        maxHeight: collapsed ? 100 : maxHeight,
        overflow: 'auto',
        background: '#f5f5f5',
        padding: 8,
        borderRadius: 4,
        fontFamily: 'monospace',
        fontSize: 12,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-all',
      }}>
        {formatted}
      </pre>
    </div>
  );
}
