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
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <div className="card w-[400px] max-w-[calc(100vw-48px)] bg-base-100 shadow-xl animate-fade-in-up">
        <div className="card-body">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="h-11 w-11 rounded-xl bg-primary flex items-center justify-center font-display font-extrabold text-lg text-primary-content tracking-tight">
              GW
            </div>
            <span className="font-display font-bold text-xl">Create Account</span>
          </div>

          {registrationDisabled && (
            <Alert variant="warning" className="mb-4">Registration is currently disabled</Alert>
          )}

          <form onSubmit={handleSubmit} className="space-y-4 mt-4">
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">Username</span></label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="Username"
                required
                minLength={3}
                disabled={registrationDisabled}
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
                minLength={6}
                disabled={registrationDisabled}
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">Confirm Password</span></label>
              <input
                type="password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                placeholder="Confirm password"
                required
                disabled={registrationDisabled}
                className="input input-bordered w-full"
              />
            </div>
            <div className="pt-2">
              <Button variant="primary" size="lg" loading={loading} disabled={registrationDisabled} className="w-full">
                Register
              </Button>
            </div>
          </form>

          <p className="text-center text-sm text-base-content/50 mt-5">
            Already have an account?{' '}
            <Link to="/console/login" className="link link-primary">Sign in</Link>
          </p>
        </div>
      </div>
    </div>
  );
}
