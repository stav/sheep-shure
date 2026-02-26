import { useState } from "react";
import { KeyRound } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CARRIERS, relativeTime } from "./utils";
import { useCarriersWithCredentials } from "@/hooks/useCarrierSync";
import { CredentialsDialog } from "./CredentialsDialog";
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
  const { data: carriersWithCreds } = useCarriersWithCredentials();
  const [credDialogCarrier, setCredDialogCarrier] = useState<{
    id: string;
    name: string;
  } | null>(null);

  return (
    <>
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
              const hasCreds = carriersWithCreds?.includes(carrier.id) ?? false;
              return (
                <tr
                  key={carrier.id}
                  className={`border-b transition-colors ${
                    isSelected ? "bg-primary/5" : "hover:bg-muted/50"
                  }`}
                >
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-2">
                      <Button
                        size="sm"
                        variant={isSelected ? "default" : "outline"}
                        title="Click to open carrier portal"
                        disabled={carrier.status === "coming_soon"}
                        onClick={() => onSelectCarrier(carrier.id)}
                      >
                        {carrier.name}
                      </Button>
                      <button
                        type="button"
                        title={hasCreds ? "Credentials saved — click to manage" : "Save portal credentials for auto-login"}
                        onClick={() => setCredDialogCarrier({ id: carrier.id, name: carrier.name })}
                        className={`rounded p-1 transition-colors hover:bg-muted ${
                          hasCreds
                            ? "text-primary"
                            : "text-muted-foreground/40 hover:text-muted-foreground"
                        }`}
                      >
                        <KeyRound className="h-4 w-4" />
                      </button>
                    </div>
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

      {credDialogCarrier && (
        <CredentialsDialog
          open={!!credDialogCarrier}
          onOpenChange={(open) => {
            if (!open) setCredDialogCarrier(null);
          }}
          carrierId={credDialogCarrier.id}
          carrierName={credDialogCarrier.name}
        />
      )}
    </>
  );
}
