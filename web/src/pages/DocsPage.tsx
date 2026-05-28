import { useParams, useNavigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import { ChevronLeft } from 'lucide-react';
import { Button } from '../components/ui/Button';

const modules = import.meta.glob('../../docs/**/*.mdx');

export default function DocsPage() {
  const { section, slug } = useParams<{ section: string; slug: string }>();
  const navigate = useNavigate();
  const [Content, setContent] = useState<React.ComponentType | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    setContent(null);
    if (!section || !slug) {
      navigate('/docs/user/getting-started', { replace: true });
      return;
    }
    const key = `../../docs/${section}/${slug}.mdx`;
    const loader = modules[key];
    if (!loader) {
      setLoading(false);
      return;
    }
    loader().then((mod: any) => {
      setContent(() => mod.default);
      setLoading(false);
    });
  }, [section, slug, navigate]);

  if (!section || !slug) return null;

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <span className="loading loading-spinner loading-lg" />
      </div>
    );
  }

  if (!Content) {
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

  return <Content />;
}