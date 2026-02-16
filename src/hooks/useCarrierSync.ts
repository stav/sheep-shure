import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { SyncResult, SyncLogEntry } from "@/types";

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

export function useSyncLogs(carrierId?: string) {
  return useQuery({
    queryKey: ["sync-logs", carrierId],
    queryFn: () =>
      tauriInvoke<SyncLogEntry[]>("get_sync_logs", {
        carrierId: carrierId ?? null,
      }),
  });
}
