import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { SyncResult, SyncLogEntry } from "@/types";

export function useCarrierSync() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      carrierId,
      authToken,
    }: {
      carrierId: string;
      authToken: string;
    }) =>
      tauriInvoke<SyncResult>("sync_carrier_portal", { carrierId, authToken }),
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

export function useOpenCarrierLogin() {
  return useMutation({
    mutationFn: (carrierId: string) =>
      tauriInvoke<string>("open_carrier_login", { carrierId }),
  });
}

export function useCarrierLoginUrl(carrierId: string) {
  return useQuery({
    queryKey: ["carrier-login-url", carrierId],
    queryFn: () =>
      tauriInvoke<string>("get_carrier_login_url", { carrierId }),
    enabled: !!carrierId,
  });
}
