import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listProviders, getProvider, createProvider, updateProvider, deleteProvider, listProviderModels, updateProviderModels } from '../api/providers';
import type { CreateProviderRequest, UpdateProviderRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useProviders() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'providers'], queryFn: listProviders, enabled: !!slug });
}

export function useProvider(id: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'providers', id], queryFn: () => getProvider(id), enabled: !!slug && !!id });
}

export function useCreateProvider() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateProviderRequest) => createProvider(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'providers'] }); toast.success(i18n.t('toasts.providerCreated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerCreateFailed'))); },
  });
}

export function useUpdateProvider() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateProviderRequest }) => updateProvider(id, input),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: [slug, 'providers'] });
      queryClient.invalidateQueries({ queryKey: [slug, 'providers', variables.id] });
      toast.success(i18n.t('toasts.providerUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerUpdateFailed'))); },
  });
}

export function useDeleteProvider() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteProvider(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'providers'] }); toast.success(i18n.t('toasts.providerDeleted')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerDeleteFailed'))); },
  });
}

export function useProviderModels(providerId: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'providers', providerId, 'models'],
    queryFn: () => listProviderModels(providerId),
    enabled: !!slug && !!providerId,
  });
}

export function useUpdateProviderModels() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ providerId, models }: { providerId: string; models: { model_id: string; upstream_name?: string; pricing_policy_id?: string | null }[] }) =>
      updateProviderModels(providerId, models),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: [slug, 'providers', variables.providerId, 'models'] });
      toast.success(i18n.t('toasts.providerModelsUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerModelsUpdateFailed'))); },
  });
}
