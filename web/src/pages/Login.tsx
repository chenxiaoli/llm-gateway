import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

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
    } catch (err) {
      toast.error(getErrorMessage(err, 'Invalid username or password'));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <div className="card w-[400px] max-w-[calc(100vw-48px)] bg-base-100 shadow-xl animate-fade-in-up">
        <div className="card-body">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="h-11 w-11 rounded-xl bg-primary flex items-center justify-center font-display font-extrabold text-lg text-primary-content tracking-tight">
              GW
            </div>
            <span className="font-display font-bold text-xl">LLM Gateway</span>
          </div>

          <form onSubmit={handleSubmit} className="space-y-4 mt-4">
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">Username</span></label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="Username"
                required
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">Password</span></label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Password"
                required
                className="input input-bordered w-full"
              />
            </div>
            <div className={authConfig?.allow_registration ? 'pt-2' : ''}>
              <Button variant="primary" size="lg" loading={loading} className="w-full">
                Sign In
              </Button>
            </div>
          </form>

          {authConfig?.allow_registration && (
            <p className="text-center text-sm text-base-content/50 mt-5">
              Don't have an account?{' '}
              <Link to="/console/register" className="link link-primary">Create one</Link>
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
