import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { SyncResult, SyncLogEntry, ImportPortalResult } from "@/types";

export function useOpenCarrierLogin() {
  return useMutation({
    mutationFn: (carrierId: string) =>
      tauriInvoke<string>("open_carrier_login", { carrierId }),
  });
}

export function useTriggerCarrierFetch() {
  return useMutation({
    mutationFn: (carrierId: string) =>
      tauriInvoke<void>("trigger_carrier_fetch", { carrierId }),
  });
}

export function useProcessPortalMembers() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      carrierId,
      membersJson,
    }: {
      carrierId: string;
      membersJson: string;
    }) =>
      tauriInvoke<SyncResult>("process_portal_members", {
        carrierId,
        membersJson,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["enrollments"] });
      queryClient.invalidateQueries({ queryKey: ["clients"] });
      queryClient.invalidateQueries({ queryKey: ["sync-logs"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard-stats"] });
    },
  });
}

export function useImportPortalMembers() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      carrierId,
      membersJson,
    }: {
      carrierId: string;
      membersJson: string;
    }) =>
      tauriInvoke<ImportPortalResult>("import_portal_members", {
        carrierId,
        membersJson,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["carriers"] });
      queryClient.invalidateQueries({ queryKey: ["enrollments"] });
      queryClient.invalidateQueries({ queryKey: ["clients"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard-stats"] });
    },
  });
}

export function useUpdateExpectedActive() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      carrierId,
      expectedActive,
    }: {
      carrierId: string;
      expectedActive: number;
    }) =>
      tauriInvoke<void>("update_carrier_expected_active", {
        carrierId,
        expectedActive,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["carriers"] });
    },
  });
}

export function useSyncLogs(carrierId?: string) {
  return useQuery({
    queryKey: ["sync-logs", carrierId],
    queryFn: () =>
      tauriInvoke<SyncLogEntry[]>("get_sync_logs", {
        carrierId: carrierId ?? null,
      }),
  });
}
