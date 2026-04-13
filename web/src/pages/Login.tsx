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
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_20%_50%,rgba(6,214,160,0.07)_0%,transparent_60%),radial-gradient(ellipse_at_80%_50%,rgba(59,130,246,0.05)_0%,transparent_60%)] pointer-events-none" />
      <div className="absolute inset-0 bg-[linear-gradient(#1e1e1e_1px,transparent_1px),linear-gradient(90deg,#1e1e1e_1px,transparent_1px)] bg-[size:60px_60px] opacity-25 [mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_70%)] pointer-events-none" />

      <div className="relative w-[400px] max-w-[calc(100vw-48px)] rounded-2xl border border-[#1e1e1e] bg-[#111111] p-10 shadow-2xl animate-fade-in-up">
        <div className="flex items-center justify-center gap-3 mb-8">
          <div className="h-11 w-11 rounded-xl bg-gradient-to-br from-accent to-emerald-600 flex items-center justify-center font-display font-extrabold text-lg text-black tracking-tight">
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
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
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
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
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
            <Link to="/console/register" className="text-accent hover:text-accent-hover transition-colors">Create one</Link>
          </p>
        )}
      </div>
    </div>
  );
}
