import { useState } from "react";
import { tauriInvoke } from "@/lib/tauri";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Loader2,
  RefreshCw,
  Upload,
  Download,
  SkipForward,
  Clock,
  ArrowLeftRight,
  CheckCircle2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type {
  ReconciliationResult,
  LocalClientSummary,
  CloudClientSummary,
  ClientConflict,
  MatchedPair,
  SyncDecision,
} from "@/types";

// ── Field label map ────────────────────────────────────────────────────────────

const FIELD_LABELS: Record<string, string> = {
  first_name: "First Name",
  last_name: "Last Name",
  dob: "Date of Birth",
  mbi: "MBI",
  phone: "Phone",
  email: "Email",
  address_line1: "Address",
  city: "City",
  state: "State",
  zip: "ZIP",
};

// ── Sub-components ─────────────────────────────────────────────────────────────

function ClientRow({ label, value }: { label: string; value?: string }) {
  if (!value) return null;
  return (
    <div className="flex gap-2 text-sm">
      <span className="text-muted-foreground w-24 shrink-0">{label}</span>
      <span>{value}</span>
    </div>
  );
}

function LocalCard({ client }: { client: LocalClientSummary }) {
  return (
    <div className="space-y-1">
      <p className="font-medium">{client.first_name} {client.last_name}</p>
      <ClientRow label="DOB" value={client.dob} />
      <ClientRow label="MBI" value={client.mbi} />
      <ClientRow label="Phone" value={client.phone} />
      <ClientRow label="Email" value={client.email} />
      <ClientRow label="Address" value={client.address_line1} />
      {(client.city || client.state || client.zip) && (
        <div className="flex gap-2 text-sm">
          <span className="text-muted-foreground w-24 shrink-0">City/State</span>
          <span>{[client.city, client.state, client.zip].filter(Boolean).join(", ")}</span>
        </div>
      )}
    </div>
  );
}

function CloudCard({ client }: { client: CloudClientSummary }) {
  const name = [client.first_name, client.last_name].filter(Boolean).join(" ") || "—";
  return (
    <div className="space-y-1">
      <p className="font-medium">{name}</p>
      <ClientRow label="DOB" value={client.dob} />
      <ClientRow label="MBI" value={client.mbi} />
      <ClientRow label="Phone" value={client.phone} />
      <ClientRow label="Email" value={client.email} />
      <ClientRow label="Address" value={client.address_line1} />
      {(client.city || client.state || client.zip) && (
        <div className="flex gap-2 text-sm">
          <span className="text-muted-foreground w-24 shrink-0">City/State</span>
          <span>{[client.city, client.state, client.zip].filter(Boolean).join(", ")}</span>
        </div>
      )}
    </div>
  );
}

// ── Only Local tab ─────────────────────────────────────────────────────────────

function OnlyLocalTab({
  clients,
  onDecision,
  deciding,
}: {
  clients: LocalClientSummary[];
  onDecision: (cloudId: string, decision: string, diff?: string) => Promise<void>;
  deciding: string | null;
}) {
  if (clients.length === 0)
    return <p className="text-sm text-muted-foreground py-6 text-center">All local clients exist in the cloud.</p>;

  return (
    <div className="space-y-3">
      {clients.map((c) => (
        <Card key={c.id}>
          <CardContent className="pt-4 flex items-start justify-between gap-4">
            <LocalCard client={c} />
            <div className="flex flex-col gap-2 shrink-0">
              <Button
                size="sm"
                variant="outline"
                disabled={deciding !== null}
                onClick={() => onDecision(c.id, "pushed")}
              >
                {deciding === c.id ? (
                  <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Upload className="mr-1.5 h-3.5 w-3.5" />
                )}
                Push
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={deciding !== null}
                onClick={() => onDecision(c.id, "skip")}
              >
                <SkipForward className="mr-1.5 h-3.5 w-3.5" />
                Skip
              </Button>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// ── Only Cloud tab ─────────────────────────────────────────────────────────────

function OnlyCloudTab({
  clients,
  onDecision,
  deciding,
}: {
  clients: CloudClientSummary[];
  onDecision: (cloudId: string, decision: string, diff?: string) => Promise<void>;
  deciding: string | null;
}) {
  if (clients.length === 0)
    return <p className="text-sm text-muted-foreground py-6 text-center">No cloud-only clients found.</p>;

  return (
    <div className="space-y-3">
      {clients.map((c, i) => {
        const id = c.cloud_id ?? `cloud-${i}`;
        return (
          <Card key={id}>
            <CardContent className="pt-4 flex items-start justify-between gap-4">
              <CloudCard client={c} />
              <div className="flex flex-col gap-2 shrink-0">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={deciding !== null}
                  onClick={() => onDecision(id, "pulled")}
                >
                  {deciding === id ? (
                    <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <Download className="mr-1.5 h-3.5 w-3.5" />
                  )}
                  Pull
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={deciding !== null}
                  onClick={() => onDecision(id, "skip")}
                >
                  <SkipForward className="mr-1.5 h-3.5 w-3.5" />
                  Skip
                </Button>
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}

// ── Conflicts tab ──────────────────────────────────────────────────────────────

function ConflictsTab({
  conflicts,
  onDecision,
  deciding,
}: {
  conflicts: ClientConflict[];
  onDecision: (cloudId: string, decision: string, diff?: string) => Promise<void>;
  deciding: string | null;
}) {
  if (conflicts.length === 0)
    return <p className="text-sm text-muted-foreground py-6 text-center">No conflicting records found.</p>;

  return (
    <div className="space-y-4">
      {conflicts.map((conflict, i) => {
        const id = conflict.cloud.cloud_id ?? `conflict-${i}`;
        const diffJson = JSON.stringify(
          Object.fromEntries(conflict.diffs.map((d) => [d.field, [d.local, d.cloud]]))
        );
        const diffFields = new Set(conflict.diffs.map((d) => d.field));

        return (
          <Card key={id}>
            <CardHeader className="pb-2">
              <CardTitle className="text-base flex items-center gap-2">
                <ArrowLeftRight className="h-4 w-4 text-orange-500" />
                {conflict.local.first_name} {conflict.local.last_name}
                <div className="flex gap-1 ml-auto">
                  {conflict.diffs.map((d) => (
                    <Badge key={d.field} variant="outline" className="text-xs text-orange-600 border-orange-300">
                      {FIELD_LABELS[d.field] ?? d.field}
                    </Badge>
                  ))}
                </div>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {/* Side-by-side */}
              <div className="grid grid-cols-2 gap-4">
                <div className="rounded-md border p-3 space-y-1.5">
                  <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Local</p>
                  {(["first_name","last_name","dob","mbi","phone","email","address_line1","city","state","zip"] as const).map((f) => {
                    const val = conflict.local[f as keyof LocalClientSummary] as string | undefined;
                    if (!val) return null;
                    return (
                      <div key={f} className={cn("flex gap-2 text-sm", diffFields.has(f) && "rounded px-1 -mx-1 bg-orange-500/10")}>
                        <span className="text-muted-foreground w-24 shrink-0">{FIELD_LABELS[f]}</span>
                        <span className={cn(diffFields.has(f) && "font-medium text-orange-700 dark:text-orange-400")}>{val}</span>
                      </div>
                    );
                  })}
                </div>
                <div className="rounded-md border p-3 space-y-1.5">
                  <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Cloud</p>
                  {(["first_name","last_name","dob","mbi","phone","email","address_line1","city","state","zip"] as const).map((f) => {
                    const val = conflict.cloud[f as keyof CloudClientSummary] as string | undefined;
                    if (!val) return null;
                    return (
                      <div key={f} className={cn("flex gap-2 text-sm", diffFields.has(f) && "rounded px-1 -mx-1 bg-orange-500/10")}>
                        <span className="text-muted-foreground w-24 shrink-0">{FIELD_LABELS[f]}</span>
                        <span className={cn(diffFields.has(f) && "font-medium text-orange-700 dark:text-orange-400")}>{val}</span>
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* Actions */}
              <div className="flex gap-2 pt-1">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={deciding !== null}
                  onClick={() => onDecision(id, "kept_local", diffJson)}
                >
                  {deciding === `kept_local:${id}` ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : <Upload className="mr-1.5 h-3.5 w-3.5" />}
                  Keep Local
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={deciding !== null}
                  onClick={() => onDecision(id, "kept_cloud", diffJson)}
                >
                  {deciding === `kept_cloud:${id}` ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : <Download className="mr-1.5 h-3.5 w-3.5" />}
                  Keep Cloud
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={deciding !== null}
                  onClick={() => onDecision(id, "skip", diffJson)}
                >
                  <SkipForward className="mr-1.5 h-3.5 w-3.5" />
                  Skip
                </Button>
              </div>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}

// ── Matched tab ────────────────────────────────────────────────────────────────

function MatchedTab({ pairs }: { pairs: MatchedPair[] }) {
  if (pairs.length === 0)
    return <p className="text-sm text-muted-foreground py-6 text-center">No matched clients yet.</p>;

  return (
    <div className="space-y-2">
      {pairs.map((p, i) => {
        const name = [p.local.first_name, p.local.last_name].filter(Boolean).join(" ");
        return (
          <div key={p.cloud.cloud_id ?? i} className="flex items-center gap-3 rounded-md border px-3 py-2 text-sm">
            <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />
            <span className="font-medium flex-1">{name}</span>
            {p.local.mbi && <span className="text-xs text-muted-foreground font-mono">{p.local.mbi}</span>}
            {p.local.dob && <span className="text-xs text-muted-foreground">{p.local.dob}</span>}
          </div>
        );
      })}
    </div>
  );
}

// ── Skipped tab ────────────────────────────────────────────────────────────────

function SkippedTab({ entries }: { entries: SkippedEntry[] }) {
  if (entries.length === 0)
    return <p className="text-sm text-muted-foreground py-6 text-center">No clients skipped this session.</p>;

  const sourceLabel: Record<string, string> = {
    local: "Local only",
    cloud: "Cloud only",
    conflict: "Conflict",
  };

  return (
    <div className="space-y-2">
      {entries.map((e) => (
        <div key={e.id} className="flex items-center gap-3 rounded-md border px-3 py-2 text-sm">
          <SkipForward className="h-4 w-4 text-muted-foreground shrink-0" />
          <span className="font-medium flex-1">{e.name}</span>
          <Badge variant="outline" className="text-xs">{sourceLabel[e.source]}</Badge>
        </div>
      ))}
    </div>
  );
}

// ── History tab ────────────────────────────────────────────────────────────────

function HistoryTab({ decisions }: { decisions: SyncDecision[] }) {
  if (decisions.length === 0)
    return <p className="text-sm text-muted-foreground py-6 text-center">No decisions recorded yet.</p>;

  const decisionLabel: Record<string, string> = {
    skip: "Skipped",
    pulled: "Pulled to local",
    pushed: "Pushed to cloud",
    kept_local: "Kept local",
    kept_cloud: "Kept cloud",
  };

  return (
    <div className="space-y-2">
      {decisions.map((d) => (
        <div key={d.cloud_record_id} className="flex items-center justify-between rounded-md border px-3 py-2 text-sm">
          <div className="flex items-center gap-3">
            <Clock className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            <span className="font-mono text-xs text-muted-foreground">{d.cloud_record_id.slice(0, 16)}…</span>
            <Badge variant="outline">{decisionLabel[d.decision] ?? d.decision}</Badge>
            {d.diff && (
              <span className="text-xs text-muted-foreground">
                ({Object.keys(JSON.parse(d.diff)).join(", ")})
              </span>
            )}
          </div>
          <span className="text-xs text-muted-foreground">{new Date(d.decided_at + "Z").toLocaleString()}</span>
        </div>
      ))}
    </div>
  );
}

// ── Main page ──────────────────────────────────────────────────────────────────

interface SkippedEntry {
  id: string;
  name: string;
  source: "local" | "cloud" | "conflict";
}

export function CloudSyncPage() {
  const [result, setResult] = useState<ReconciliationResult | null>(null);
  const [decisions, setDecisions] = useState<SyncDecision[]>([]);
  const [skipped, setSkipped] = useState<SkippedEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [deciding, setDeciding] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState("matched");

  const loadDecisions = async () => {
    try {
      const d = await tauriInvoke<SyncDecision[]>("get_sync_decisions");
      setDecisions(d);
    } catch {
      // non-critical
    }
  };

  const handleLoad = async () => {
    setLoading(true);
    setSkipped([]);
    try {
      const [r] = await Promise.all([
        tauriInvoke<ReconciliationResult>("compare_with_convex"),
        loadDecisions(),
      ]);
      setResult(r);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to compare databases");
    } finally {
      setLoading(false);
    }
  };

  const handleDecision = async (cloudId: string, decision: string, diff?: string) => {
    setDeciding(cloudId);
    try {
      // For "pushed", invoke the real single-client push first
      if (decision === "pushed") {
        await tauriInvoke("push_client_to_convex", { clientId: cloudId });
      }
      // For "pulled", create the client locally from cloud data
      if (decision === "pulled") {
        const cloudClient = result?.only_cloud.find((c) => (c.cloud_id ?? "") === cloudId);
        if (cloudClient) {
          await tauriInvoke("pull_client_from_cloud", { client: cloudClient });
        }
      }

      await tauriInvoke("save_sync_decision", {
        input: {
          cloud_record_id: cloudId,
          decision,
          diff: diff ?? null,
          expires_days: null,
        },
      });

      const label: Record<string, string> = {
        pushed: "Pushed to cloud",
        pulled: "Pulled to local",
        kept_local: "Kept local",
        kept_cloud: "Kept cloud",
        skip: "Skipped",
      };
      toast.success(label[decision] ?? decision);
      // Track skipped entries for the Skipped tab
      if (decision === "skip" && result) {
        const localClient = result.only_local.find((c) => c.id === cloudId);
        const cloudClient = result.only_cloud.find((c) => (c.cloud_id ?? "") === cloudId);
        const conflict = result.conflicts.find((c) => (c.cloud.cloud_id ?? "") === cloudId);
        const entry = localClient
          ? { id: cloudId, name: `${localClient.first_name} ${localClient.last_name}`, source: "local" as const }
          : cloudClient
          ? { id: cloudId, name: [cloudClient.first_name, cloudClient.last_name].filter(Boolean).join(" "), source: "cloud" as const }
          : conflict
          ? { id: cloudId, name: `${conflict.local.first_name} ${conflict.local.last_name}`, source: "conflict" as const }
          : null;
        if (entry) setSkipped((prev) => [...prev, entry]);
      }
      // Remove the decided item and move pushed/pulled clients to matched
      setResult((prev) => {
        if (!prev) return prev;
        const pushedClient = decision === "pushed"
          ? prev.only_local.find((c) => c.id === cloudId)
          : undefined;
        const pulledCloud = decision === "pulled"
          ? prev.only_cloud.find((c) => (c.cloud_id ?? "") === cloudId)
          : undefined;
        const newMatched = pushedClient
          ? [...prev.matched, { local: pushedClient, cloud: { first_name: pushedClient.first_name, last_name: pushedClient.last_name, dob: pushedClient.dob, mbi: pushedClient.mbi, phone: pushedClient.phone, email: pushedClient.email, address_line1: pushedClient.address_line1, city: pushedClient.city, state: pushedClient.state, zip: pushedClient.zip } }]
          : pulledCloud
          ? [...prev.matched, { local: { id: cloudId, first_name: pulledCloud.first_name ?? "", last_name: pulledCloud.last_name ?? "", dob: pulledCloud.dob, mbi: pulledCloud.mbi, phone: pulledCloud.phone, email: pulledCloud.email, address_line1: pulledCloud.address_line1, city: pulledCloud.city, state: pulledCloud.state, zip: pulledCloud.zip }, cloud: pulledCloud }]
          : prev.matched;
        return {
          ...prev,
          only_local: prev.only_local.filter((c) => c.id !== cloudId),
          only_cloud: prev.only_cloud.filter((c) => (c.cloud_id ?? "") !== cloudId),
          conflicts: prev.conflicts.filter((c) => (c.cloud.cloud_id ?? "") !== cloudId),
          matched: newMatched,
        };
      });
      await loadDecisions();
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to save decision");
    } finally {
      setDeciding(null);
    }
  };

  return (
    <div className="space-y-6 max-w-5xl">
      <Tabs value={activeTab} onValueChange={setActiveTab}>
        {/* Header: button + tabs inline */}
        <div className="flex items-center gap-4">
          <Button onClick={handleLoad} disabled={loading} className="shrink-0">
            {loading ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <RefreshCw className="mr-2 h-4 w-4" />
            )}
            {result ? "Refresh" : "Load Comparison"}
          </Button>

          {result && (
            <TabsList>
              <TabsTrigger value="matched">
                In Sync
                {result.matched.length > 0 && (
                  <Badge variant="secondary" className="ml-2">{result.matched.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="local">
                Local Only
                {result.only_local.length > 0 && (
                  <Badge variant="secondary" className="ml-2">{result.only_local.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="cloud">
                Cloud Only
                {result.only_cloud.length > 0 && (
                  <Badge variant="secondary" className="ml-2">{result.only_cloud.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="conflicts">
                Conflicts
                {result.conflicts.length > 0 && (
                  <Badge className="ml-2 bg-orange-500 hover:bg-orange-600">{result.conflicts.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="skipped">
                Skipped
                {skipped.length > 0 && (
                  <Badge variant="secondary" className="ml-2">{skipped.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="history">History</TabsTrigger>
            </TabsList>
          )}
        </div>

        {!result && !loading && (
          <p className="text-sm text-muted-foreground">
            Click "Load Comparison" to fetch the cloud database and compare it with your local data.
          </p>
        )}

        {result && (
          <>
            <TabsContent value="local" className="mt-4">
              <OnlyLocalTab clients={result.only_local} onDecision={handleDecision} deciding={deciding} />
            </TabsContent>
            <TabsContent value="cloud" className="mt-4">
              <OnlyCloudTab clients={result.only_cloud} onDecision={handleDecision} deciding={deciding} />
            </TabsContent>
            <TabsContent value="matched" className="mt-4">
              <MatchedTab pairs={result.matched} />
            </TabsContent>
            <TabsContent value="conflicts" className="mt-4">
              <ConflictsTab conflicts={result.conflicts} onDecision={handleDecision} deciding={deciding} />
            </TabsContent>
            <TabsContent value="skipped" className="mt-4">
              <SkippedTab entries={skipped} />
            </TabsContent>
            <TabsContent value="history" className="mt-4">
              <HistoryTab decisions={decisions} />
            </TabsContent>
          </>
        )}
      </Tabs>
    </div>
  );
}
