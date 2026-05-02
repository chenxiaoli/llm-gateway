import { useQuery } from '@tanstack/react-query';
import { listUserModels } from '../api/userModels';

export function useUserModels() {
  return useQuery({ queryKey: ['user-models'], queryFn: listUserModels });
}
