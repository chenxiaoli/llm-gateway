import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';

export default function Login() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const login = useAuthStore((s) => s.login);

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username || !password) return;
    setLoading(true);
    try {
      await login({ username, password });
      navigate('/console/dashboard');
    } catch {
      toast.error('Invalid username or password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center relative overflow-hidden bg-black">
      {/* Background atmosphere */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[600px] h-[600px] bg-[radial-gradient(ellipse_at_center,rgba(6,214,160,0.06)_0%,transparent_60%)]" />
        <div className="absolute bottom-0 right-0 w-[400px] h-[400px] bg-[radial-gradient(ellipse_at_center,rgba(59,130,246,0.04)_0%,transparent_60%)]" />
      </div>
      <div className="absolute inset-0 bg-[linear-gradient(#1e1e1e_1px,transparent_1px),linear-gradient(90deg,#1e1e1e_1px,transparent_1px)] bg-[size:60px_60px] opacity-20 [mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_70%)] pointer-events-none" />

      <div
        className="relative w-[400px] max-w-[calc(100vw-48px)] rounded-2xl p-10 shadow-2xl animate-fade-in-up"
        style={{
          background: 'linear-gradient(180deg, #111111 0%, #0c0c0c 100%)',
          border: '1px solid rgba(30, 30, 30, 0.8)',
          boxShadow: '0 0 80px rgba(0, 0, 0, 0.5), 0 0 40px rgba(6, 214, 160, 0.03)',
        }}
      >
        <div className="flex items-center justify-center gap-3 mb-8">
          <div
            className="h-11 w-11 rounded-xl flex items-center justify-center font-display font-extrabold text-lg text-black tracking-tight"
            style={{
              background: 'linear-gradient(135deg, #06d6a0 0%, #059669 100%)',
              boxShadow: '0 0 20px rgba(6, 214, 160, 0.3)',
            }}
          >
            GW
          </div>
          <span className="font-display font-bold text-xl text-[#ededed]">LLM Gateway</span>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Username"
              required
              className="h-10 w-full rounded-lg px-3 text-sm text-[#ededed] placeholder-[#666666] outline-none transition-all duration-200"
              style={{ background: '#0a0a0a', border: '1px solid rgba(38, 38, 38, 0.8)' }}
              onFocus={(e) => { e.target.style.borderColor = 'rgba(6, 214, 160, 0.4)'; e.target.style.boxShadow = '0 0 0 3px rgba(6, 214, 160, 0.08)'; }}
              onBlur={(e) => { e.target.style.borderColor = 'rgba(38, 38, 38, 0.8)'; e.target.style.boxShadow = 'none'; }}
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              required
              className="h-10 w-full rounded-lg px-3 text-sm text-[#ededed] placeholder-[#666666] outline-none transition-all duration-200"
              style={{ background: '#0a0a0a', border: '1px solid rgba(38, 38, 38, 0.8)' }}
              onFocus={(e) => { e.target.style.borderColor = 'rgba(6, 214, 160, 0.4)'; e.target.style.boxShadow = '0 0 0 3px rgba(6, 214, 160, 0.08)'; }}
              onBlur={(e) => { e.target.style.borderColor = 'rgba(38, 38, 38, 0.8)'; e.target.style.boxShadow = 'none'; }}
            />
          </div>
          <div className={authConfig?.allow_registration ? 'mb-5' : ''}>
            <Button variant="primary" size="lg" loading={loading} className="w-full">
              Sign In
            </Button>
          </div>
        </form>

        {authConfig?.allow_registration && (
          <p className="mt-5 text-center text-sm text-[#888888]">
            Don't have an account?{' '}
            <Link to="/console/register" className="transition-colors duration-200" style={{ color: '#06d6a0' }}
              onMouseEnter={(e) => { (e.target as HTMLElement).style.color = '#34d399'; }}
              onMouseLeave={(e) => { (e.target as HTMLElement).style.color = '#06d6a0'; }}
            >
              Create one
            </Link>
          </p>
        )}
      </div>
    </div>
  );
}
