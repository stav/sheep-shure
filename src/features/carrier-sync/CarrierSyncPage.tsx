import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, ArrowRightLeft, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  useOpenCarrierLogin,
  useTriggerCarrierFetch,
  useProcessPortalMembers,
  useSyncLogs,
  useUpdateExpectedActive,
  useCarrierSyncInfo,
} from "@/hooks/useCarrierSync";
import { useCarriers } from "@/hooks/useClients";
import { CARRIERS } from "./utils";
import { CarrierTable } from "./CarrierTable";
import { SyncResultsPanel } from "./SyncResultsPanel";
import type { SyncResult } from "@/types";

type SyncPhase = "idle" | "login" | "fetching" | "processing";

export function CarrierSyncPage() {
  const [selectedCarrier, setSelectedCarrier] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<SyncResult | null>(null);
  const [syncPhase, setSyncPhase] = useState<SyncPhase>("idle");
  const [syncError, setSyncError] = useState<string | null>(null);

  const openLogin = useOpenCarrierLogin();
  const triggerFetch = useTriggerCarrierFetch();
  const processMembers = useProcessPortalMembers();
  const { data: syncLogs } = useSyncLogs();
  const { data: dbCarriers } = useCarriers();
  const updateExpectedActive = useUpdateExpectedActive();
  const { data: syncInfo } = useCarrierSyncInfo(selectedCarrier);

  const isAutoFetch = syncInfo?.auto_fetch ?? false;

  // Listen for data coming back from the carrier webview
  const handleSyncData = useCallback(
    (carrierId: string, membersJson: string) => {
      setSyncPhase("processing");
      setSyncError(null);
      processMembers.mutate(
        { carrierId, membersJson },
        {
          onSuccess: (result) => {
            setLastResult(result);
            setSyncPhase("idle");
          },
          onError: (err) => {
            setSyncError(String(err));
            setSyncPhase("idle");
          },
        }
      );
    },
    [processMembers]
  );

  // Set up Tauri event listeners
  useEffect(() => {
    const unlistenData = listen<string>("carrier-sync-data", (event) => {
      if (selectedCarrier) {
        handleSyncData(selectedCarrier, event.payload);
      }
    });

    const unlistenError = listen<string>("carrier-sync-error", (event) => {
      setSyncError(event.payload);
      setSyncPhase("idle");
    });

    return () => {
      unlistenData.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, [selectedCarrier, handleSyncData]);

  const handleOpenPortal = (carrierId: string) => {
    setSelectedCarrier(carrierId);
    setSyncError(null);
    setLastResult(null);
    setSyncPhase("login");
    openLogin.mutate(carrierId, {
      onError: (err) => {
        setSyncError(String(err));
        setSyncPhase("idle");
      },
    });
  };

  const handleTriggerSync = () => {
    if (!selectedCarrier) return;
    setSyncPhase("fetching");
    setSyncError(null);
    triggerFetch.mutate(selectedCarrier, {
      onError: (err) => {
        setSyncError(String(err));
        setSyncPhase("idle");
      },
    });
  };

  // Description text based on phase and auto_fetch
  const getDescription = () => {
    if (syncPhase === "fetching") return "Fetching member data from the carrier portal...";
    if (syncPhase === "processing") return "Comparing portal data against local enrollments...";
    if (syncPhase === "idle" && lastResult) return "Sync complete. You can run another sync or open a different carrier.";

    // Login or idle-without-result: show carrier-specific instruction
    if (syncInfo?.sync_instruction) return syncInfo.sync_instruction;
    return "Open the portal, log in, then click Sync Now to fetch and compare member data.";
  };

  return (
    <div className="space-y-6">
      <CarrierTable
        syncLogs={syncLogs}
        dbCarriers={dbCarriers}
        selectedCarrier={selectedCarrier}
        onSelectCarrier={handleOpenPortal}
      />

      {/* Sync controls */}
      {selectedCarrier && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">
              Sync{" "}
              {CARRIERS.find((c) => c.id === selectedCarrier)?.name}
            </CardTitle>
            <CardDescription>{getDescription()}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Auto-fetch carriers: show spinner during login, hide Sync Now button */}
            {isAutoFetch && syncPhase === "login" && !lastResult && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                Waiting for login — data will sync automatically...
              </div>
            )}

            {/* Always show Sync Now for manual carriers, or as re-sync for auto carriers */}
            {(!isAutoFetch || syncPhase !== "login") && (
              <Button
                onClick={handleTriggerSync}
                disabled={syncPhase === "fetching" || syncPhase === "processing"}
              >
                {syncPhase === "fetching" || syncPhase === "processing" ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <ArrowRightLeft className="mr-2 h-4 w-4" />
                )}
                {syncPhase === "fetching"
                  ? "Fetching from portal..."
                  : syncPhase === "processing"
                    ? "Processing..."
                    : "Sync Now"}
              </Button>
            )}

            {syncError && (
              <div className="flex items-start gap-2 rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
                <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                <span>{syncError}</span>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Sync results */}
      {lastResult && (
        <SyncResultsPanel
          result={lastResult}
          carrierId={selectedCarrier!}
          carrier={dbCarriers?.find((c) => c.id === selectedCarrier)}
          onUpdateExpected={(count) => {
            if (selectedCarrier) {
              updateExpectedActive.mutate({
                carrierId: selectedCarrier,
                expectedActive: count,
              });
            }
          }}
          onImported={(result, importedMembers) => {
            if (result.imported > 0) {
              setLastResult((prev) => {
                if (!prev) return prev;
                const importedNames = new Set(
                  importedMembers.map((m) => `${m.first_name}|${m.last_name}|${m.dob ?? ""}`)
                );
                return {
                  ...prev,
                  new_in_portal: prev.new_in_portal.filter(
                    (m) => !importedNames.has(`${m.first_name}|${m.last_name}|${m.dob ?? ""}`)
                  ),
                };
              });
            }
          }}
          onDisenrolled={(confirmedIds) => {
            setLastResult((prev) => {
              if (!prev) return prev;
              const ids = new Set(confirmedIds);
              return {
                ...prev,
                disenrolled: prev.disenrolled.filter(
                  (d) => !ids.has(d.enrollment_id)
                ),
              };
            });
          }}
        />
      )}
    </div>
  );
}
