import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { Client, ClientListItem, ClientFilters, PaginatedResult, Carrier } from "@/types";

export function useClients(filters: ClientFilters, page: number, perPage: number) {
  return useQuery({
    queryKey: ["clients", filters, page, perPage],
    queryFn: () =>
      tauriInvoke<PaginatedResult<ClientListItem>>("get_clients", {
        filters,
        page,
        perPage,
      }),
  });
}

export function useClient(id: string | undefined) {
  return useQuery({
    queryKey: ["client", id],
    queryFn: () => tauriInvoke<Client>("get_client", { id }),
    enabled: !!id,
  });
}

export function useCreateClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: Partial<Client>) =>
      tauriInvoke<Client>("create_client", { input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["clients"] });
    },
  });
}

export function useUpdateClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: Partial<Client> }) =>
      tauriInvoke<Client>("update_client", { id, input }),
    onSuccess: (_, vars) => {
      queryClient.invalidateQueries({ queryKey: ["clients"] });
      queryClient.invalidateQueries({ queryKey: ["client", vars.id] });
    },
  });
}

export function useDeleteClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tauriInvoke("delete_client", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["clients"] });
    },
  });
}

export function useCarriers() {
  return useQuery({
    queryKey: ["carriers"],
    queryFn: () => tauriInvoke<Carrier[]>("get_carriers"),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
