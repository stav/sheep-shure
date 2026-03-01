import { useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useCommissionSummary } from "@/hooks";

export function CarrierSummaryTab({
  onDrillDown,
}: {
  onDrillDown: (carrierId: string, month: string) => void;
}) {
  const [month, setMonth] = useState<string | undefined>();
  const { data: summaries, isLoading } = useCommissionSummary(month);

  const fmt = (v?: number | null) =>
    v != null ? `$${v.toFixed(2)}` : "—";

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <Input
          type="month"
          value={month ?? ""}
          onChange={(e) => setMonth(e.target.value || undefined)}
          className="w-44"
          placeholder="All months"
        />
      </div>

      <Card>
        <CardContent className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="px-4 py-3 text-left font-medium">Carrier</th>
                  <th className="px-4 py-3 text-left font-medium">Month</th>
                  <th className="px-4 py-3 text-right font-medium">Expected</th>
                  <th className="px-4 py-3 text-right font-medium">Statement</th>
                  <th className="px-4 py-3 text-right font-medium">Paid</th>
                  <th className="px-4 py-3 text-right font-medium">Deposit</th>
                  <th className="px-4 py-3 text-right font-medium">Deposit Diff</th>
                  <th className="px-4 py-3 text-right font-medium">Entries</th>
                  <th className="px-4 py-3 text-right font-medium">OK</th>
                  <th className="px-4 py-3 text-right font-medium">Issues</th>
                </tr>
              </thead>
              <tbody>
                {isLoading ? (
                  <tr>
                    <td colSpan={10} className="px-4 py-8 text-center text-muted-foreground">
                      Loading...
                    </td>
                  </tr>
                ) : !summaries?.length ? (
                  <tr>
                    <td colSpan={10} className="px-4 py-8 text-center text-muted-foreground">
                      No commission data yet.
                    </td>
                  </tr>
                ) : (
                  summaries.map((s) => (
                    <tr
                      key={`${s.carrier_id}-${s.commission_month}`}
                      className="border-b last:border-b-0 hover:bg-muted/25 cursor-pointer"
                      onClick={() => onDrillDown(s.carrier_id, s.commission_month)}
                    >
                      <td className="px-4 py-3 font-medium">{s.carrier_name}</td>
                      <td className="px-4 py-3">{s.commission_month}</td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(s.total_expected)}</td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(s.total_statement)}</td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(s.total_paid)}</td>
                      <td className="px-4 py-3 text-right font-mono">{fmt(s.deposit_amount)}</td>
                      <td className="px-4 py-3 text-right font-mono">
                        {s.deposit_vs_paid != null ? (
                          <span
                            className={
                              s.deposit_vs_paid < -0.01
                                ? "text-red-600"
                                : s.deposit_vs_paid > 0.01
                                  ? "text-orange-600"
                                  : "text-green-600"
                            }
                          >
                            {s.deposit_vs_paid >= 0 ? "+" : ""}
                            ${s.deposit_vs_paid.toFixed(2)}
                          </span>
                        ) : (
                          "—"
                        )}
                      </td>
                      <td className="px-4 py-3 text-right">{s.entry_count}</td>
                      <td className="px-4 py-3 text-right text-green-600">{s.ok_count}</td>
                      <td className="px-4 py-3 text-right text-red-600">
                        {s.issue_count > 0 ? s.issue_count : "—"}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
