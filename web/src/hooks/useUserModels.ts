import { useQuery } from '@tanstack/react-query';
import { listUserModels } from '../api/userModels';
import { useAuthStore } from '../stores/authStore';

export function useUserModels() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'user-models'], queryFn: listUserModels, enabled: !!slug });
}
