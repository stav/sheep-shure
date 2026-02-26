import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { Client, ClientListItem, ClientFilters, PaginatedResult, Carrier, DuplicateCandidate, DuplicateGroup } from "@/types";

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

export function useHardDeleteClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tauriInvoke("hard_delete_client", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["clients"] });
    },
  });
}

export function useCheckClientDuplicates() {
  return useMutation({
    mutationFn: (input: {
      firstName: string;
      lastName: string;
      dob?: string | null;
      mbi?: string | null;
    }) =>
      tauriInvoke<DuplicateCandidate[]>("check_client_duplicates", {
        firstName: input.firstName,
        lastName: input.lastName,
        dob: input.dob || null,
        mbi: input.mbi || null,
      }),
  });
}

export function useFindDuplicateClients() {
  return useQuery({
    queryKey: ["duplicate-clients"],
    queryFn: () => tauriInvoke<DuplicateGroup[]>("find_duplicate_clients"),
    enabled: false,
  });
}

export function useMergeClients() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ keeperId, sourceId }: { keeperId: string; sourceId: string }) =>
      tauriInvoke<Client>("merge_clients", { keeperId, sourceId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["clients"] });
      queryClient.invalidateQueries({ queryKey: ["duplicate-clients"] });
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
