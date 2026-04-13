import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { useKey, useUpdateKey, useDeleteKey } from '../hooks/useKeys';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';

export default function KeyDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: key, isLoading } = useKey(id!);
  const updateMutation = useUpdateKey();
  const deleteMutation = useDeleteKey();

  const [name, setName] = useState('');
  const [enabled, setEnabled] = useState(false);
  const [rateLimit, setRateLimit] = useState('');
  const [budgetMonthly, setBudgetMonthly] = useState('');

  useEffect(() => {
    if (key) {
      setName(key.name);
      setEnabled(key.enabled);
      setRateLimit(key.rate_limit != null ? String(key.rate_limit) : '');
      setBudgetMonthly(key.budget_monthly != null ? String(key.budget_monthly) : '');
    }
  }, [key]);

  if (isLoading) return <div className="flex justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>;
  if (!key) return <div className="text-base-content/40">Key not found</div>;

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    await updateMutation.mutateAsync({
      id: key.id,
      input: {
        name,
        rate_limit: rateLimit ? Number(rateLimit) : null,
        budget_monthly: budgetMonthly ? Number(budgetMonthly) : null,
        enabled,
      },
    });
  };

  const handleDelete = async () => {
    await deleteMutation.mutateAsync(key.id);
    navigate('/console/keys');
  };

  return (
    <div>
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/console/keys')} className="mb-4">
        Back to Keys
      </Button>

      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold">Edit Key: {key.name}</h1>
      </div>

      <form onSubmit={handleUpdate} className="max-w-lg space-y-4">
        <div className="form-control">
          <label className="label"><span className="label-text">Name</span></label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            className="input input-bordered w-full"
          />
        </div>
        <div className="flex items-center justify-between">
          <label className="label-text">Enabled</label>
          <Toggle checked={enabled} onChange={setEnabled} />
        </div>
        <div className="form-control">
          <label className="label"><span className="label-text">Rate Limit (RPM, empty = unlimited)</span></label>
          <input
            type="number"
            value={rateLimit}
            onChange={(e) => setRateLimit(e.target.value)}
            min={1}
            className="input input-bordered w-full"
          />
        </div>
        <div className="form-control">
          <label className="label"><span className="label-text">Monthly Budget ($, empty = unlimited)</span></label>
          <input
            type="number"
            value={budgetMonthly}
            onChange={(e) => setBudgetMonthly(e.target.value)}
            min={0}
            step={0.01}
            className="input input-bordered w-full"
          />
        </div>
        <div className="flex gap-2">
          <Button variant="primary" type="submit" loading={updateMutation.isPending}>Save</Button>
          <ConfirmDialog title="Delete this key?" onConfirm={handleDelete} okText="Delete">
            <Button variant="danger">Delete Key</Button>
          </ConfirmDialog>
        </div>
      </form>
    </div>
  );
}
