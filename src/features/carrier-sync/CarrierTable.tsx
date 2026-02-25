import { Button } from "@/components/ui/button";
import { CARRIERS, relativeTime } from "./utils";
import type { Carrier, SyncLogEntry } from "@/types";

export function CarrierTable({
  syncLogs,
  dbCarriers,
  selectedCarrier,
  onSelectCarrier,
}: {
  syncLogs?: SyncLogEntry[];
  dbCarriers?: Carrier[];
  selectedCarrier: string | null;
  onSelectCarrier: (carrierId: string) => void;
}) {
  return (
    <div className="rounded-md border">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b bg-muted/50">
            <th className="h-10 px-4 text-left font-medium text-muted-foreground">Carrier</th>
            <th className="h-10 px-4 text-left font-medium text-muted-foreground">Last Sync</th>
            <th className="h-10 px-4 text-right font-medium text-muted-foreground">Found</th>
            <th className="h-10 px-4 text-right font-medium text-muted-foreground">Active</th>
            <th className="h-10 px-4 text-right font-medium text-muted-foreground">Expected</th>
            <th className="h-10 px-4 text-right font-medium text-muted-foreground">+/−</th>
          </tr>
        </thead>
        <tbody>
          {CARRIERS.map((carrier) => {
            const latestLog = syncLogs?.find((l) => l.carrier_id === carrier.id);
            const dbCarrier = dbCarriers?.find((c) => c.id === carrier.id);
            const expected = dbCarrier?.expected_active ?? 0;
            const found = latestLog?.portal_count ?? null;
            const active = latestLog?.matched ?? null;
            const diff = expected > 0 && active !== null ? active - expected : null;
            const isSelected = selectedCarrier === carrier.id;
            return (
              <tr
                key={carrier.id}
                className={`border-b transition-colors ${
                  isSelected ? "bg-primary/5" : "hover:bg-muted/50"
                }`}
              >
                <td className="px-4 py-3">
                  <Button
                    size="sm"
                    variant={isSelected ? "default" : "outline"}
                    title="Click to open carrier portal"
                    disabled={carrier.status === "coming_soon"}
                    onClick={() => onSelectCarrier(carrier.id)}
                  >
                    {carrier.name}
                  </Button>
                </td>
                <td className="px-4 py-3 text-muted-foreground">
                  {latestLog ? (
                    <>
                      {new Date(latestLog.synced_at).toLocaleDateString()}
                      <span className="ml-2 text-xs opacity-60">
                        {relativeTime(latestLog.synced_at)}
                      </span>
                    </>
                  ) : (
                    "—"
                  )}
                </td>
                <td className="px-4 py-3 text-right tabular-nums">
                  {found ?? "—"}
                </td>
                <td className="px-4 py-3 text-right tabular-nums">
                  {active ?? "—"}
                </td>
                <td className="px-4 py-3 text-right tabular-nums">
                  {expected > 0 ? expected : "—"}
                </td>
                <td className="px-4 py-3 text-right tabular-nums">
                  {diff !== null ? (
                    <span
                      className={
                        diff === 0
                          ? "text-green-600"
                          : diff > 0
                            ? "text-blue-600"
                            : "text-red-600"
                      }
                    >
                      {diff > 0 ? `+${diff}` : diff}
                    </span>
                  ) : (
                    "—"
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
