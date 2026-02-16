import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ExternalLink,
  CheckCircle2,
  AlertTriangle,
  Users,
  ArrowRightLeft,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  useOpenCarrierLogin,
  useTriggerCarrierFetch,
  useProcessPortalMembers,
  useSyncLogs,
} from "@/hooks/useCarrierSync";
import type { SyncResult } from "@/types";

interface CarrierConfig {
  id: string;
  name: string;
  description: string;
  status: "available" | "coming_soon";
}

const CARRIERS: CarrierConfig[] = [
  {
    id: "carrier-devoted",
    name: "Devoted Health",
    description: "React SPA, GraphQL API",
    status: "available",
  },
  {
    id: "carrier-caresource",
    name: "CareSource",
    description: "DestinationRx, REST API",
    status: "available",
  },
];

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

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-2xl font-bold tracking-tight">
          Carrier Portal Sync
        </h2>
        <p className="text-muted-foreground">
          Verify your book of business against carrier portals and auto-update
          enrollment statuses.
        </p>
      </div>

      {/* Carrier cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {CARRIERS.map((carrier) => {
          const latestLog = syncLogs?.find((l) => l.carrier_id === carrier.id);
          const isSelected = selectedCarrier === carrier.id;
          return (
            <Card
              key={carrier.id}
              className={
                carrier.status === "coming_soon"
                  ? "opacity-60"
                  : isSelected
                    ? "ring-2 ring-primary"
                    : ""
              }
            >
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-base">{carrier.name}</CardTitle>
                  {carrier.status === "coming_soon" ? (
                    <Badge variant="secondary">Coming Soon</Badge>
                  ) : (
                    <Badge variant="outline">Available</Badge>
                  )}
                </div>
                <CardDescription>{carrier.description}</CardDescription>
              </CardHeader>
              <CardContent>
                {latestLog && (
                  <div className="mb-3 text-xs text-muted-foreground">
                    Last sync:{" "}
                    {new Date(latestLog.synced_at).toLocaleString()}
                    <span className="ml-2">
                      ({latestLog.matched} matched, {latestLog.disenrolled}{" "}
                      disenrolled)
                    </span>
                  </div>
                )}
                <Button
                  size="sm"
                  className="w-full"
                  disabled={carrier.status === "coming_soon"}
                  onClick={() => handleOpenPortal(carrier.id)}
                >
                  <ExternalLink className="mr-2 h-4 w-4" />
                  Open Portal Login
                </Button>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {/* Sync controls */}
      {selectedCarrier && (
        <>
          <Separator />
          <Card>
            <CardHeader>
              <CardTitle className="text-base">
                Sync{" "}
                {CARRIERS.find((c) => c.id === selectedCarrier)?.name}
              </CardTitle>
              <CardDescription>
                {syncPhase === "login" &&
                  "Log in to the carrier portal in the opened window, then click Sync Now."}
                {syncPhase === "fetching" &&
                  "Fetching member data from the carrier portal..."}
                {syncPhase === "processing" &&
                  "Comparing portal data against local enrollments..."}
                {syncPhase === "idle" &&
                  !lastResult &&
                  "Open the portal, log in, then click Sync Now to fetch and compare member data."}
                {syncPhase === "idle" &&
                  lastResult &&
                  "Sync complete. You can run another sync or open a different carrier."}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
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

              {syncError && (
                <div className="flex items-start gap-2 rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
                  <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                  <span>{syncError}</span>
                </div>
              )}
            </CardContent>
          </Card>
        </>
      )}

      {/* Sync results */}
      {lastResult && (
        <>
          <Separator />
          <SyncResultsPanel result={lastResult} />
        </>
      )}

      {/* Sync history */}
      {syncLogs && syncLogs.length > 0 && (
        <>
          <Separator />
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Sync History</CardTitle>
            </CardHeader>
            <CardContent>
              <ScrollArea className="h-48">
                <div className="space-y-2">
                  {syncLogs.map((log) => (
                    <div
                      key={log.id}
                      className="flex items-center justify-between rounded-md border p-3 text-sm"
                    >
                      <div>
                        <span className="font-medium">
                          {log.carrier_name ?? log.carrier_id}
                        </span>
                        <span className="ml-2 text-muted-foreground">
                          {new Date(log.synced_at).toLocaleString()}
                        </span>
                      </div>
                      <div className="flex items-center gap-3 text-xs text-muted-foreground">
                        <span>{log.portal_count} portal</span>
                        <span>{log.matched} matched</span>
                        {log.disenrolled > 0 && (
                          <Badge variant="destructive" className="text-xs">
                            {log.disenrolled} disenrolled
                          </Badge>
                        )}
                        {log.new_found > 0 && (
                          <Badge variant="secondary" className="text-xs">
                            {log.new_found} new
                          </Badge>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}

function SyncResultsPanel({ result }: { result: SyncResult }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <CheckCircle2 className="h-5 w-5 text-green-500" />
          Sync Complete — {result.carrier_name}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Summary stats */}
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          <div className="rounded-md border p-3 text-center">
            <div className="text-2xl font-bold">{result.portal_count}</div>
            <div className="text-xs text-muted-foreground">In Portal</div>
          </div>
          <div className="rounded-md border p-3 text-center">
            <div className="text-2xl font-bold">{result.local_count}</div>
            <div className="text-xs text-muted-foreground">Local</div>
          </div>
          <div className="rounded-md border p-3 text-center">
            <div className="text-2xl font-bold text-green-600">
              {result.matched}
            </div>
            <div className="text-xs text-muted-foreground">Matched</div>
          </div>
          <div className="rounded-md border p-3 text-center">
            <div className="text-2xl font-bold text-red-600">
              {result.disenrolled.length}
            </div>
            <div className="text-xs text-muted-foreground">Disenrolled</div>
          </div>
        </div>

        {/* Disenrolled list */}
        {result.disenrolled.length > 0 && (
          <div>
            <h4 className="mb-2 flex items-center gap-2 text-sm font-medium">
              <AlertTriangle className="h-4 w-4 text-red-500" />
              Auto-Disenrolled ({result.disenrolled.length})
            </h4>
            <ScrollArea className="h-40">
              <div className="space-y-1">
                {result.disenrolled.map((d) => (
                  <div
                    key={d.enrollment_id}
                    className="flex items-center justify-between rounded-md border border-red-200 bg-red-50 p-2 text-sm dark:border-red-900 dark:bg-red-950"
                  >
                    <span className="font-medium">{d.client_name}</span>
                    <span className="text-muted-foreground">
                      {d.plan_name ?? "—"}
                    </span>
                    <Badge variant="destructive" className="text-xs">
                      Disenrolled
                    </Badge>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </div>
        )}

        {/* New in portal list */}
        {result.new_in_portal.length > 0 && (
          <div>
            <h4 className="mb-2 flex items-center gap-2 text-sm font-medium">
              <Users className="h-4 w-4 text-blue-500" />
              New in Portal ({result.new_in_portal.length})
            </h4>
            <ScrollArea className="h-40">
              <div className="space-y-1">
                {result.new_in_portal.map((m, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between rounded-md border border-blue-200 bg-blue-50 p-2 text-sm dark:border-blue-900 dark:bg-blue-950"
                  >
                    <span className="font-medium">
                      {m.first_name} {m.last_name}
                    </span>
                    <span className="text-muted-foreground">
                      {[m.city, m.state].filter(Boolean).join(", ") || "—"}
                    </span>
                    <span className="text-muted-foreground">
                      {m.plan_name ?? "—"}
                    </span>
                    <Badge variant="secondary" className="text-xs">
                      {m.status ?? "New"}
                    </Badge>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </div>
        )}

        {result.disenrolled.length === 0 &&
          result.new_in_portal.length === 0 && (
            <p className="text-sm text-muted-foreground">
              All local enrollments matched the portal. No changes needed.
            </p>
          )}
      </CardContent>
    </Card>
  );
}
