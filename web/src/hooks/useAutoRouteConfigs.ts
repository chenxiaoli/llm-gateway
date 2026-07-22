import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  listAutoRouteConfigs,
  createAutoRouteConfig,
  updateAutoRouteConfig,
  deleteAutoRouteConfig,
} from '../api/autoRoutes';
import type { CreateAutoRouteConfigRequest, UpdateAutoRouteConfigRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useAutoRouteConfigs() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'auto-route-configs'],
    queryFn: listAutoRouteConfigs,
    enabled: !!slug,
  });
}

export function useCreateAutoRouteConfig() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateAutoRouteConfigRequest) => createAutoRouteConfig(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'auto-route-configs'] });
      toast.success(i18n.t('toasts.autoRouteCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.autoRouteCreateFailed'))); },
  });
}

export function useUpdateAutoRouteConfig() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateAutoRouteConfigRequest }) =>
      updateAutoRouteConfig(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'auto-route-configs'] });
      toast.success(i18n.t('toasts.autoRouteUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.autoRouteUpdateFailed'))); },
  });
}

export function useDeleteAutoRouteConfig() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteAutoRouteConfig(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'auto-route-configs'] });
      toast.success(i18n.t('toasts.autoRouteDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.autoRouteDeleteFailed'))); },
  });
}
