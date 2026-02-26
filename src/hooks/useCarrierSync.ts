import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { SyncResult, SyncLogEntry, ImportPortalResult, ConfirmDisenrollmentResult, CarrierSyncInfo } from "@/types";

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

export function useConfirmDisenrollments() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (enrollmentIds: string[]) =>
      tauriInvoke<ConfirmDisenrollmentResult>("confirm_disenrollments", {
        enrollmentIds,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["enrollments"] });
      queryClient.invalidateQueries({ queryKey: ["clients"] });
      queryClient.invalidateQueries({ queryKey: ["sync-logs"] });
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

export function useCarrierSyncInfo(carrierId: string | null) {
  return useQuery({
    queryKey: ["carrier-sync-info", carrierId],
    queryFn: () =>
      tauriInvoke<CarrierSyncInfo>("get_carrier_sync_info", {
        carrierId: carrierId!,
      }),
    enabled: !!carrierId,
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

export function useSavePortalCredentials() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      carrierId,
      username,
      password,
    }: {
      carrierId: string;
      username: string;
      password: string;
    }) =>
      tauriInvoke<void>("save_portal_credentials", {
        carrierId,
        username,
        password,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["carriers-with-credentials"] });
    },
  });
}

export function useGetPortalCredentials(carrierId: string | null) {
  return useQuery({
    queryKey: ["portal-credentials", carrierId],
    queryFn: () =>
      tauriInvoke<{ username: string; password: string } | null>(
        "get_portal_credentials",
        { carrierId: carrierId! }
      ),
    enabled: !!carrierId,
  });
}

export function useDeletePortalCredentials() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (carrierId: string) =>
      tauriInvoke<void>("delete_portal_credentials", { carrierId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["carriers-with-credentials"] });
      queryClient.invalidateQueries({ queryKey: ["portal-credentials"] });
    },
  });
}

export function useCarriersWithCredentials() {
  return useQuery({
    queryKey: ["carriers-with-credentials"],
    queryFn: () =>
      tauriInvoke<string[]>("get_carriers_with_credentials"),
  });
}
