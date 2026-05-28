import { useParams, useNavigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { ChevronLeft } from 'lucide-react';
import { Button } from '../components/ui/Button';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';

const modules = import.meta.glob('../../docs/**/*.md', { eager: true });

export default function DocsPage() {
  const { section, slug } = useParams<{ section: string; slug: string }>();
  const navigate = useNavigate();
  const [content, setContent] = useState('');

  useEffect(() => {
    setContent('');
    if (!section || !slug) {
      navigate('/docs/user/getting-started', { replace: true });
      return;
    }
    const key = `../../docs/${section}/${slug}.md`;
    const mod = modules[key] as any;
    if (mod && mod.default) {
      setContent(mod.default);
    }
  }, [section, slug, navigate]);

  if (!section || !slug) return null;

  if (!content) {
    return (
      <div className="text-center py-20">
        <h1 className="text-2xl font-bold mb-4">404</h1>
        <p className="text-base-content/60 mb-4">Document not found</p>
        <Button onClick={() => navigate('/docs/user/getting-started')}>
          <ChevronLeft className="h-4 w-4" /> Go to Home
        </Button>
      </div>
    );
  }

  return (
    <div className="prose prose-sm max-w-none">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code({ className, children, ...props }) {
            const match = /language-(\w+)/.exec(className || '');
            const code = String(children).replace(/\n$/, '');
            if (!match) {
              return <code className={className} {...props}>{code}</code>;
            }
            return (
              <SyntaxHighlighter
                style={oneDark}
                language={match[1]}
                PreTag="div"
              >
                {code}
              </SyntaxHighlighter>
            );
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}