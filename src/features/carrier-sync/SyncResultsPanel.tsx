import { useState, useEffect, useRef } from "react";
import { CheckCircle2, Loader2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { isPortalMemberActive } from "./utils";
import { DisenrollmentSection } from "./DisenrollmentSection";
import { NewInPortalSection } from "./NewInPortalSection";
import { useImportPortalMembers } from "@/hooks/useCarrierSync";
import type { Carrier, SyncResult, PortalMember, ImportPortalResult } from "@/types";

type StatView = "portal" | "active" | "inactive" | "matched" | "disenrolled" | null;

function MatchTierBadge({ tier }: { tier: string }) {
  switch (tier) {
    case "exact":
      return <Badge variant="outline" className="text-xs text-green-700">Exact Match</Badge>;
    case "fuzzy":
      return <Badge variant="outline" className="text-xs text-amber-600">Fuzzy Match</Badge>;
    case "mbi":
      return <Badge variant="outline" className="text-xs text-blue-600">MBI Match</Badge>;
    case "existing_client":
      return <Badge variant="outline" className="text-xs text-purple-600">Existing Client</Badge>;
    default:
      return <Badge variant="outline" className="text-xs text-green-700">Matched</Badge>;
  }
}

export function SyncResultsPanel({
  result,
  carrierId,
  carrier,
  onUpdateExpected,
  onImported,
  onDisenrolled,
}: {
  result: SyncResult;
  carrierId: string;
  carrier?: Carrier;
  onUpdateExpected: (count: number) => void;
  onImported: (result: ImportPortalResult, members: PortalMember[]) => void;
  onDisenrolled: (confirmedIds: string[]) => void;
}) {
  // All portal members = matched + new
  const allPortalMembers: PortalMember[] = [
    ...result.matched_members.map((m) => m.portal_member),
    ...result.new_in_portal,
  ];
  const activeMembers = allPortalMembers.filter((m) => isPortalMemberActive(m));
  const inactiveMembers = allPortalMembers.filter((m) => !isPortalMemberActive(m));

  const expectedActive = carrier?.expected_active ?? 0;
  const hasExpected = expectedActive > 0;
  const matchesExpected = hasExpected && activeMembers.length === expectedActive;

  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(String(expectedActive));
  const inputRef = useRef<HTMLInputElement>(null);
  const [expandedStat, setExpandedStat] = useState<StatView>(null);

  // Split matched members into enrollment matches vs existing-client matches
  const enrollmentMatches = result.matched_members.filter(
    (m) => m.match_tier !== "existing_client",
  );
  const clientOnlyMatches = result.matched_members.filter(
    (m) => m.match_tier === "existing_client",
  );

  // Import state for existing-client matches
  const [selectedClientMatches, setSelectedClientMatches] = useState<Set<number>>(new Set());
  const [clientMatchImportResult, setClientMatchImportResult] = useState<ImportPortalResult | null>(null);
  const importExistingClients = useImportPortalMembers();

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

  const toggleClientMatch = (index: number) => {
    setSelectedClientMatches((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const toggleAllClientMatches = () => {
    if (selectedClientMatches.size === clientOnlyMatches.length) {
      setSelectedClientMatches(new Set());
    } else {
      setSelectedClientMatches(new Set(clientOnlyMatches.map((_, i) => i)));
    }
  };

  const handleImportClientMatches = () => {
    const selected = clientOnlyMatches.filter((_, i) => selectedClientMatches.has(i));
    if (selected.length === 0) return;

    const members = selected.map((m) => m.portal_member);
    setClientMatchImportResult(null);
    importExistingClients.mutate(
      { carrierId, membersJson: JSON.stringify(members) },
      {
        onSuccess: (res) => {
          setClientMatchImportResult(res);
          setSelectedClientMatches(new Set());
          onImported(res, members);
        },
        onError: (err) => {
          setClientMatchImportResult({ imported: 0, imported_names: [], errors: [String(err)] });
        },
      },
    );
  };

  const toggleStat = (stat: StatView) =>
    setExpandedStat((prev) => (prev === stat ? null : stat));

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
      <Badge variant={isPortalMemberActive(m) ? "secondary" : "destructive"} className="text-xs">
        {isPortalMemberActive(m) ? "Active" : "Inactive"}
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
          {statBox("To Disenroll", result.disenrolled.length, "disenrolled", "text-red-600")}
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
          <div className="space-y-3">
            {/* Enrollment matches — read-only */}
            {enrollmentMatches.length > 0 && (
              <ScrollArea className="h-48">
                <div className="space-y-1">
                  {enrollmentMatches.map((m, i) => (
                    <div
                      key={i}
                      className="flex items-center justify-between rounded-md border border-green-200 bg-green-50 p-2 text-sm dark:border-green-900 dark:bg-green-950"
                    >
                      <span className="font-medium">{m.client_name}</span>
                      <span className="text-muted-foreground">
                        {m.portal_member.plan_name ?? "—"}
                      </span>
                      <MatchTierBadge tier={m.match_tier} />
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}

            {/* Existing-client matches — importable */}
            {clientOnlyMatches.length > 0 && (
              <div>
                <div className="mb-2 flex items-center justify-between">
                  <h4 className="text-sm font-medium text-purple-700 dark:text-purple-400">
                    Existing Clients ({clientOnlyMatches.length})
                  </h4>
                  <div className="flex items-center gap-2">
                    <Button size="sm" variant="outline" onClick={toggleAllClientMatches}>
                      {selectedClientMatches.size === clientOnlyMatches.length ? "Deselect All" : "Select All"}
                    </Button>
                    <Button
                      size="sm"
                      disabled={selectedClientMatches.size === 0 || importExistingClients.isPending}
                      onClick={handleImportClientMatches}
                    >
                      {importExistingClients.isPending ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : null}
                      Import Selected ({selectedClientMatches.size})
                    </Button>
                  </div>
                </div>

                {clientMatchImportResult && (
                  <div
                    className={`mb-2 rounded-md border p-3 text-sm ${
                      clientMatchImportResult.errors.length > 0
                        ? "border-yellow-300 bg-yellow-50 dark:border-yellow-800 dark:bg-yellow-950"
                        : "border-green-300 bg-green-50 dark:border-green-800 dark:bg-green-950"
                    }`}
                  >
                    <p className="font-medium">
                      Imported {clientMatchImportResult.imported} enrollment{clientMatchImportResult.imported !== 1 ? "s" : ""} successfully.
                      {clientMatchImportResult.imported_names.length > 0 && (
                        <span className="font-normal"> ({clientMatchImportResult.imported_names.join(", ")})</span>
                      )}
                    </p>
                    {clientMatchImportResult.errors.map((err, i) => (
                      <p key={i} className="mt-1 text-destructive">{err}</p>
                    ))}
                  </div>
                )}

                <ScrollArea className="h-48">
                  <div className="space-y-1">
                    {clientOnlyMatches.map((m, i) => (
                      <div
                        key={i}
                        className="flex items-center gap-3 rounded-md border border-purple-200 bg-purple-50 p-2 text-sm dark:border-purple-900 dark:bg-purple-950"
                      >
                        <Checkbox
                          checked={selectedClientMatches.has(i)}
                          onCheckedChange={() => toggleClientMatch(i)}
                        />
                        <span className="min-w-[140px] font-medium">{m.client_name}</span>
                        <span className="flex-1 text-muted-foreground">
                          {m.portal_member.plan_name ?? "—"}
                        </span>
                        <Badge variant={isPortalMemberActive(m.portal_member) ? "secondary" : "destructive"} className="text-xs">
                          {isPortalMemberActive(m.portal_member) ? "Active" : "Inactive"}
                        </Badge>
                        <MatchTierBadge tier={m.match_tier} />
                      </div>
                    ))}
                  </div>
                </ScrollArea>
              </div>
            )}

            {enrollmentMatches.length === 0 && clientOnlyMatches.length === 0 && (
              <p className="py-4 text-center text-sm text-muted-foreground">
                No matched members.
              </p>
            )}
          </div>
        )}

        {expandedStat === "disenrolled" && (
          <DisenrollmentSection
            disenrolled={result.disenrolled}
            onDisenrolled={onDisenrolled}
          />
        )}

        {/* New in portal list with import */}
        <NewInPortalSection
          members={result.new_in_portal}
          carrierId={carrierId}
          onImported={onImported}
        />

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
