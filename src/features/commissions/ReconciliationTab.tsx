import { useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useReconciliationEntries,
  useReconcileCommissions,
  useFindMissingCommissions,
  useCarriers,
} from "@/hooks";
import { StatusBadge } from "./components/StatusBadge";
import type { CommissionFilters } from "@/types";

export function ReconciliationTab({
  initialCarrierId,
  initialMonth,
}: {
  initialCarrierId?: string;
  initialMonth?: string;
}) {
  const [carrierId, setCarrierId] = useState<string | undefined>(initialCarrierId);
  const [month, setMonth] = useState<string | undefined>(initialMonth);
  const [statusFilter, setStatusFilter] = useState<string | undefined>();

  const { data: carriers } = useCarriers();

  const filters: CommissionFilters = {
    carrier_id: carrierId,
    commission_month: month,
    status: statusFilter,
  };

  const { data: entries, isLoading } = useReconciliationEntries(filters);
  const reconcile = useReconcileCommissions();
  const findMissing = useFindMissingCommissions();

  const handleReconcile = () => {
    reconcile.mutate({ carrierId: carrierId, month: month });
  };

  const handleFindMissing = () => {
    if (carrierId && month) {
      findMissing.mutate({ carrierId, month });
    }
  };

  const fmt = (v?: number) =>
    v != null ? `$${v.toFixed(2)}` : "—";

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Select
            value={carrierId ?? "all"}
            onValueChange={(v) => setCarrierId(v === "all" ? undefined : v)}
          >
            <SelectTrigger className="w-48">
              <SelectValue placeholder="All Carriers" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Carriers</SelectItem>
              {carriers?.map((c) => (
                <SelectItem key={c.id} value={c.id}>
                  {c.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Input
            type="month"
            value={month ?? ""}
            onChange={(e) => setMonth(e.target.value || undefined)}
            className="w-44"
          />

          <Select
            value={statusFilter ?? "all"}
            onValueChange={(v) => setStatusFilter(v === "all" ? undefined : v)}
          >
            <SelectTrigger className="w-40">
              <SelectValue placeholder="All Statuses" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Statuses</SelectItem>
              <SelectItem value="OK">OK</SelectItem>
              <SelectItem value="UNDERPAID">Underpaid</SelectItem>
              <SelectItem value="OVERPAID">Overpaid</SelectItem>
              <SelectItem value="MISSING">Missing</SelectItem>
              <SelectItem value="ZERO_RATE">No Rate</SelectItem>
              <SelectItem value="UNMATCHED">Unmatched</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleFindMissing}
            disabled={!carrierId || !month || findMissing.isPending}
          >
            <Search className="mr-2 h-4 w-4" />
            Find Missing
          </Button>
          <Button
            size="sm"
            onClick={handleReconcile}
            disabled={reconcile.isPending}
          >
            <RefreshCw className="mr-2 h-4 w-4" />
            {reconcile.isPending ? "Reconciling..." : "Reconcile"}
          </Button>
        </div>
      </div>

      <Card>
        <CardContent className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="px-4 py-3 text-left font-medium">Client</th>
                  <th className="px-4 py-3 text-left font-medium">Carrier</th>
                  <th className="px-4 py-3 text-left font-medium">Month</th>
                  <th className="px-4 py-3 text-left font-medium">Eff. Date</th>
                  <th className="px-4 py-3 text-left font-medium">Type</th>
                  <th className="px-4 py-3 text-right font-medium">Expected</th>
                  <th className="px-4 py-3 text-right font-medium">Statement</th>
                  <th className="px-4 py-3 text-right font-medium">Paid</th>
                  <th className="px-4 py-3 text-right font-medium">Diff</th>
                  <th className="px-4 py-3 text-left font-medium">Status</th>
                </tr>
              </thead>
              <tbody>
                {isLoading ? (
                  <tr>
                    <td colSpan={10} className="px-4 py-8 text-center text-muted-foreground">
                      Loading...
                    </td>
                  </tr>
                ) : !entries?.length ? (
                  <tr>
                    <td colSpan={10} className="px-4 py-8 text-center text-muted-foreground">
                      No entries found. Import a statement and run reconciliation to see results.
                    </td>
                  </tr>
                ) : (
                  entries.map((row) => (
                    <tr key={row.id} className="border-b last:border-b-0 hover:bg-muted/25">
                      <td className="px-4 py-3">
                        {row.client_name ?? (
                          <span className="text-muted-foreground italic">
                            {row.member_name ?? "Unknown"}
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3">{row.carrier_name}</td>
                      <td className="px-4 py-3">{row.commission_month}</td>
                      <td className="px-4 py-3">{row.effective_date ?? "—"}</td>
                      <td className="px-4 py-3">
                        {row.is_initial != null
                          ? row.is_initial === 1
                            ? "Initial"
                            : "Renewal"
                          : "—"}
                      </td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(row.expected_rate)}</td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(row.statement_amount)}</td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(row.paid_amount)}</td>
                      <td className="px-4 py-3 text-right font-mono">
                        {row.rate_difference != null ? (
                          <span
                            className={
                              row.rate_difference < -0.01
                                ? "text-red-600"
                                : row.rate_difference > 0.01
                                  ? "text-orange-600"
                                  : ""
                            }
                          >
                            {row.rate_difference >= 0 ? "+" : ""}
                            ${row.rate_difference.toFixed(2)}
                          </span>
                        ) : (
                          "—"
                        )}
                      </td>
                      <td className="px-4 py-3">
                        <StatusBadge status={row.status} />
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>

      {entries && entries.length > 0 && (
        <div className="text-sm text-muted-foreground">
          {entries.length} entries
          {" | "}
          {entries.filter((e) => e.status === "OK").length} OK
          {" | "}
          {entries.filter((e) => e.status && e.status !== "OK").length} issues
        </div>
      )}
    </div>
  );
}
