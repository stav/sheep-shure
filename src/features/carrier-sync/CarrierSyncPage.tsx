import { useState, useEffect, useCallback, useRef } from "react";
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
  useImportPortalMembers,
  useSyncLogs,
  useUpdateExpectedActive,
} from "@/hooks/useCarrierSync";
import { useCarriers } from "@/hooks/useClients";
import { Checkbox } from "@/components/ui/checkbox";
import type { Carrier, SyncResult, PortalMember, ImportPortalResult } from "@/types";

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
  {
    id: "carrier-medmutual",
    name: "Medical Mutual of Ohio",
    description: "MyBrokerLink, server-rendered",
    status: "available",
  },
  {
    id: "carrier-uhc",
    name: "UnitedHealthcare",
    description: "Jarvis portal, REST APIs",
    status: "available",
  },
  {
    id: "carrier-humana",
    name: "Humana",
    description: "Vantage agent portal",
    status: "available",
  },
  {
    id: "carrier-anthem",
    name: "Anthem/Elevance",
    description: "Broker portal, BOB",
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
  const { data: dbCarriers } = useCarriers();
  const updateExpectedActive = useUpdateExpectedActive();

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
          const dbCarrier = dbCarriers?.find((c) => c.id === carrier.id);
          const expected = dbCarrier?.expected_active ?? 0;
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
                  ) : expected > 0 ? (
                    <Badge variant="outline">
                      Expected: {expected}
                    </Badge>
                  ) : (
                    <Badge variant="outline">Available</Badge>
                  )}
                </div>
                <CardDescription>{carrier.description}</CardDescription>
              </CardHeader>
              <CardContent>
                {latestLog && (
                  <div className="mb-3 text-xs text-muted-foreground">
                    <div>
                      Last sync:{" "}
                      {new Date(latestLog.synced_at).toLocaleString()}
                    </div>
                    <div className="mt-0.5">
                      {latestLog.portal_count} in portal, {latestLog.matched} matched
                      {latestLog.disenrolled > 0 && (
                        <span className="text-red-600">
                          , {latestLog.disenrolled} disenrolled
                        </span>
                      )}
                      {latestLog.new_found > 0 && (
                        <span className="text-blue-600">
                          , {latestLog.new_found} new
                        </span>
                      )}
                    </div>
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
                // Remove successfully imported members from the displayed result
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
          />
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

type StatView = "portal" | "active" | "inactive" | "matched" | "disenrolled" | null;

function SyncResultsPanel({
  result,
  carrierId,
  carrier,
  onUpdateExpected,
  onImported,
}: {
  result: SyncResult;
  carrierId: string;
  carrier?: Carrier;
  onUpdateExpected: (count: number) => void;
  onImported: (result: ImportPortalResult, members: PortalMember[]) => void;
}) {
  // All portal members = matched + new
  const allPortalMembers: PortalMember[] = [
    ...result.matched_members.map((m) => m.portal_member),
    ...result.new_in_portal,
  ];
  const isActiveFn = (s?: string) => {
    if (!s) return false;
    const l = s.toLowerCase();
    return l.includes("active") && !l.includes("inactive");
  };
  const isInactiveFn = (s?: string) => !!s && s.toLowerCase().includes("inactive");
  const activeMembers = allPortalMembers.filter((m) => isActiveFn(m.status));
  const inactiveMembers = allPortalMembers.filter((m) => isInactiveFn(m.status));

  const expectedActive = carrier?.expected_active ?? 0;
  const hasExpected = expectedActive > 0;
  const matchesExpected = hasExpected && activeMembers.length === expectedActive;

  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(String(expectedActive));
  const inputRef = useRef<HTMLInputElement>(null);
  const [expandedStat, setExpandedStat] = useState<StatView>(null);

  // Import state
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(new Set());
  const [importResult, setImportResult] = useState<ImportPortalResult | null>(null);
  const importMembers = useImportPortalMembers();

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  const handleSaveExpected = () => {
    const parsed = parseInt(editValue, 10);
    if (!isNaN(parsed) && parsed >= 0) {
      onUpdateExpected(parsed);
    }
    setEditing(false);
  };

  const toggleStat = (stat: StatView) =>
    setExpandedStat((prev) => (prev === stat ? null : stat));

  const toggleSelect = (index: number) => {
    setSelectedIndices((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const toggleAll = () => {
    if (selectedIndices.size === result.new_in_portal.length) {
      setSelectedIndices(new Set());
    } else {
      setSelectedIndices(new Set(result.new_in_portal.map((_, i) => i)));
    }
  };

  const handleImport = () => {
    const members = result.new_in_portal.filter((_, i) => selectedIndices.has(i));
    if (members.length === 0) return;

    setImportResult(null);
    importMembers.mutate(
      { carrierId, membersJson: JSON.stringify(members) },
      {
        onSuccess: (res) => {
          setImportResult(res);
          setSelectedIndices(new Set());
          onImported(res, members);
        },
        onError: (err) => {
          setImportResult({ imported: 0, errors: [String(err)] });
        },
      }
    );
  };

  const statBox = (
    label: string,
    count: number,
    stat: StatView,
    color: string,
    extra?: React.ReactNode,
  ) => (
    <div
      className={`cursor-pointer rounded-md border p-3 text-center transition-colors hover:bg-muted/50 ${
        expandedStat === stat ? "ring-2 ring-primary" : ""
      }`}
      onClick={() => toggleStat(stat)}
    >
      <div className={`text-2xl font-bold ${color}`}>{count}{extra}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );

  const portalMemberRow = (m: PortalMember, key: string | number) => (
    <div
      key={key}
      className="flex items-center justify-between rounded-md border p-2 text-sm"
    >
      <span className="font-medium">
        {m.first_name} {m.last_name}
      </span>
      <span className="text-muted-foreground">
        {[m.city, m.state].filter(Boolean).join(", ") || "—"}
      </span>
      <span className="text-muted-foreground">{m.plan_name ?? "—"}</span>
      <Badge variant="secondary" className="text-xs">
        {m.status ?? "—"}
      </Badge>
    </div>
  );

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <CheckCircle2 className="h-5 w-5 text-green-500" />
          Sync Complete — {result.carrier_name}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Summary stat boxes — clickable */}
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-5">
          {statBox("In Portal", result.portal_count, "portal", "")}
          <div
            className={`cursor-pointer rounded-md border p-3 text-center transition-colors hover:bg-muted/50 ${
              expandedStat === "active" ? "ring-2 ring-primary" : ""
            }`}
            onClick={() => {
              if (editing) return;
              toggleStat("active");
            }}
            onDoubleClick={() => {
              setEditValue(String(expectedActive));
              setEditing(true);
            }}
            title="Click to view members, double-click to set expected count"
          >
            <div className="text-2xl font-bold">
              <span className={hasExpected ? (matchesExpected ? "text-green-600" : "text-red-600") : "text-green-600"}>
                {activeMembers.length}
              </span>
              {hasExpected && (
                <span className="text-base font-normal text-muted-foreground">
                  {" / "}
                  {expectedActive}
                </span>
              )}
            </div>
            <div className="text-xs text-muted-foreground">
              Active{hasExpected ? (matchesExpected ? " ✓" : " ✗") : ""}
            </div>
            {editing && (
              <div
                className="mt-2 flex items-center gap-1"
                onClick={(e) => e.stopPropagation()}
              >
                <input
                  ref={inputRef}
                  type="number"
                  min="0"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleSaveExpected();
                    if (e.key === "Escape") setEditing(false);
                  }}
                  onBlur={handleSaveExpected}
                  className="h-7 w-16 rounded border bg-background px-2 text-center text-sm"
                />
              </div>
            )}
          </div>
          {statBox("Inactive", inactiveMembers.length, "inactive", "text-red-600")}
          {statBox("Matched", result.matched, "matched", "text-green-600")}
          {statBox("Disenrolled", result.disenrolled.length, "disenrolled", "text-red-600")}
        </div>

        {/* Expanded stat detail panel */}
        {expandedStat === "portal" && (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {allPortalMembers.map((m, i) => portalMemberRow(m, i))}
            </div>
          </ScrollArea>
        )}

        {expandedStat === "active" && (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {activeMembers.map((m, i) => portalMemberRow(m, i))}
            </div>
          </ScrollArea>
        )}

        {expandedStat === "inactive" && (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {inactiveMembers.length === 0 ? (
                <p className="py-4 text-center text-sm text-muted-foreground">
                  No inactive members.
                </p>
              ) : (
                inactiveMembers.map((m, i) => portalMemberRow(m, i))
              )}
            </div>
          </ScrollArea>
        )}

        {expandedStat === "matched" && (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {result.matched_members.length === 0 ? (
                <p className="py-4 text-center text-sm text-muted-foreground">
                  No matched members.
                </p>
              ) : (
                result.matched_members.map((m, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between rounded-md border border-green-200 bg-green-50 p-2 text-sm dark:border-green-900 dark:bg-green-950"
                  >
                    <span className="font-medium">{m.client_name}</span>
                    <span className="text-muted-foreground">
                      {m.portal_member.plan_name ?? "—"}
                    </span>
                    <Badge variant="outline" className="text-xs text-green-700">
                      Matched
                    </Badge>
                  </div>
                ))
              )}
            </div>
          </ScrollArea>
        )}

        {expandedStat === "disenrolled" && (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {result.disenrolled.length === 0 ? (
                <p className="py-4 text-center text-sm text-muted-foreground">
                  No disenrolled members.
                </p>
              ) : (
                result.disenrolled.map((d) => (
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
                ))
              )}
            </div>
          </ScrollArea>
        )}

        {/* New in portal list with import */}
        {result.new_in_portal.length > 0 && (
          <div>
            <div className="mb-2 flex items-center justify-between">
              <h4 className="flex items-center gap-2 text-sm font-medium">
                <Users className="h-4 w-4 text-blue-500" />
                New in Portal ({result.new_in_portal.length})
              </h4>
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  onClick={toggleAll}
                >
                  {selectedIndices.size === result.new_in_portal.length
                    ? "Deselect All"
                    : "Select All"}
                </Button>
                <Button
                  size="sm"
                  disabled={selectedIndices.size === 0 || importMembers.isPending}
                  onClick={handleImport}
                >
                  {importMembers.isPending ? (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  ) : null}
                  Import Selected ({selectedIndices.size})
                </Button>
              </div>
            </div>

            {/* Import result feedback */}
            {importResult && (
              <div
                className={`mb-2 rounded-md border p-3 text-sm ${
                  importResult.errors.length > 0
                    ? "border-yellow-300 bg-yellow-50 dark:border-yellow-800 dark:bg-yellow-950"
                    : "border-green-300 bg-green-50 dark:border-green-800 dark:bg-green-950"
                }`}
              >
                <p className="font-medium">
                  Imported {importResult.imported} member{importResult.imported !== 1 ? "s" : ""} successfully.
                </p>
                {importResult.errors.map((err, i) => (
                  <p key={i} className="mt-1 text-destructive">{err}</p>
                ))}
              </div>
            )}

            <ScrollArea className="h-40">
              <div className="space-y-1">
                {result.new_in_portal.map((m, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-3 rounded-md border border-blue-200 bg-blue-50 p-2 text-sm dark:border-blue-900 dark:bg-blue-950"
                  >
                    <Checkbox
                      checked={selectedIndices.has(i)}
                      onCheckedChange={() => toggleSelect(i)}
                    />
                    <span className="min-w-[140px] font-medium">
                      {m.first_name} {m.last_name}
                    </span>
                    <span className="min-w-[120px] text-muted-foreground">
                      {[m.city, m.state].filter(Boolean).join(", ") || "—"}
                    </span>
                    <span className="flex-1 text-muted-foreground">
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
          result.new_in_portal.length === 0 &&
          result.matched_members.length === 0 && (
            <p className="text-sm text-muted-foreground">
              No portal data to display.
            </p>
          )}
      </CardContent>
    </Card>
  );
}
