import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type {
  CommissionRateListItem,
  CreateCommissionRateInput,
  UpdateCommissionRateInput,
  CommissionEntryListItem,
  CommissionFilters,
  StatementImportResult,
  CommissionDepositListItem,
  CreateCommissionDepositInput,
  UpdateCommissionDepositInput,
  UpdateCommissionEntryInput,
  ReconciliationRow,
  CarrierMonthSummary,
} from "@/types";

// ── Commission Rates ─────────────────────────────────────────────────────────

export function useCommissionRates(carrierId?: string, planYear?: number) {
  return useQuery({
    queryKey: ["commission-rates", carrierId, planYear],
    queryFn: () =>
      tauriInvoke<CommissionRateListItem[]>("get_commission_rates", {
        carrierId,
        planYear,
      }),
  });
}

export function useCreateCommissionRate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateCommissionRateInput) =>
      tauriInvoke<CommissionRateListItem>("create_commission_rate", { input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-rates"] });
    },
  });
}

export function useUpdateCommissionRate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateCommissionRateInput }) =>
      tauriInvoke("update_commission_rate", { id, input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-rates"] });
    },
  });
}

export function useDeleteCommissionRate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      tauriInvoke("delete_commission_rate", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-rates"] });
    },
  });
}

// ── Commission Entries ───────────────────────────────────────────────────────

export function useCommissionEntries(filters: CommissionFilters) {
  return useQuery({
    queryKey: ["commission-entries", filters],
    queryFn: () =>
      tauriInvoke<CommissionEntryListItem[]>("get_commission_entries", { filters }),
  });
}

export function useDeleteCommissionBatch() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (batchId: string) =>
      tauriInvoke<number>("delete_commission_batch", { batchId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

export function useUpdateCommissionEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateCommissionEntryInput }) =>
      tauriInvoke("update_commission_entry", { id, input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

export function useDeleteCommissionEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      tauriInvoke("delete_commission_entry", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

// ── Statement Import ─────────────────────────────────────────────────────────

export function useImportCommissionStatement() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: {
      filePath: string;
      carrierId: string;
      commissionMonth: string;
      columnMapping: Record<string, string>;
    }) =>
      tauriInvoke<StatementImportResult>("import_commission_statement", args),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

// ── Reconciliation ───────────────────────────────────────────────────────────

export function useReconciliationEntries(filters: CommissionFilters) {
  return useQuery({
    queryKey: ["reconciliation", filters],
    queryFn: () =>
      tauriInvoke<ReconciliationRow[]>("get_reconciliation_entries", { filters }),
  });
}

export function useReconcileCommissions() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { carrierId?: string; month?: string }) =>
      tauriInvoke<number>("reconcile_commissions", args),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

export function useFindMissingCommissions() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { carrierId: string; month: string }) =>
      tauriInvoke<number>("find_missing_commissions", args),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

export function useCommissionSummary(month?: string) {
  return useQuery({
    queryKey: ["commission-summary", month],
    queryFn: () =>
      tauriInvoke<CarrierMonthSummary[]>("get_commission_summary", { month }),
  });
}

// ── Humana Commission Fetch ──────────────────────────────────────────────────

export function useTriggerCommissionFetch() {
  return useMutation({
    mutationFn: (args: { fromDate: string; thruDate: string }) =>
      tauriInvoke("trigger_commission_fetch", args),
  });
}

export function useImportCommissionCsv() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: {
      carrierId: string;
      commissionMonth: string;
      csvContent: string;
    }) => tauriInvoke<StatementImportResult>("import_commission_csv", args),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-entries"] });
      queryClient.invalidateQueries({ queryKey: ["reconciliation"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

// ── Generic Carrier Commission Fetch ─────────────────────────────────────────

export function useTriggerCarrierCommissionFetch() {
  return useMutation({
    mutationFn: (args: { carrierId: string }) =>
      tauriInvoke("trigger_carrier_commission_fetch", args),
  });
}

// ── Commission Deposits ──────────────────────────────────────────────────────

export function useCommissionDeposits(carrierId?: string, month?: string) {
  return useQuery({
    queryKey: ["commission-deposits", carrierId, month],
    queryFn: () =>
      tauriInvoke<CommissionDepositListItem[]>("get_commission_deposits", {
        carrierId,
        month,
      }),
  });
}

export function useCreateCommissionDeposit() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateCommissionDepositInput) =>
      tauriInvoke("create_commission_deposit", { input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-deposits"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

export function useUpdateCommissionDeposit() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateCommissionDepositInput }) =>
      tauriInvoke("update_commission_deposit", { id, input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-deposits"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}

export function useDeleteCommissionDeposit() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      tauriInvoke("delete_commission_deposit", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["commission-deposits"] });
      queryClient.invalidateQueries({ queryKey: ["commission-summary"] });
    },
  });
}
