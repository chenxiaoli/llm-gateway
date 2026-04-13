import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';

export default function Register() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const register = useAuthStore((s) => s.register);

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const registrationDisabled = authConfig !== undefined && !authConfig.allow_registration;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password !== confirm) {
      toast.error('Passwords do not match');
      return;
    }
    if (!username || !password) return;
    setLoading(true);
    try {
      await register({ username, password });
      navigate('/console/dashboard');
    } catch {
      toast.error('Registration failed');
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
          <span className="font-display font-bold text-xl text-[#ededed]">Create Account</span>
        </div>

        {registrationDisabled && (
          <Alert variant="warning" className="mb-4">Registration is currently disabled</Alert>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Username"
              required
              minLength={3}
              disabled={registrationDisabled}
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors disabled:opacity-50"
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
              minLength={6}
              disabled={registrationDisabled}
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors disabled:opacity-50"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Confirm Password</label>
            <input
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              placeholder="Confirm password"
              required
              disabled={registrationDisabled}
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors disabled:opacity-50"
            />
          </div>
          <div className="mb-5">
            <Button variant="primary" size="lg" loading={loading} disabled={registrationDisabled} className="w-full">
              Register
            </Button>
          </div>
        </form>

        <p className="text-center text-sm text-[#888888]">
          Already have an account?{' '}
          <Link to="/console/login" className="text-accent hover:text-accent-hover transition-colors">Sign in</Link>
        </p>
      </div>
    </div>
  );
}
