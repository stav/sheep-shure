import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Upload, Trash2, Download, Loader2, ScrollText, ChevronDown, ChevronRight } from "lucide-react";
import type { OpenDialogOptions } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useCarriers,
  useCommissionEntries,
  useImportCommissionStatement,
  useDeleteCommissionBatch,
  useTriggerCommissionFetch,
  useImportCommissionCsv,
} from "@/hooks";
import { useOpenCarrierLogin } from "@/hooks/useCarrierSync";
import { ActivityLog } from "./ActivityLog";
import { RawDataDialog } from "./components/RawDataDialog";
import type {
  StatementImportResult,
  CommissionCsvPayload,
  CommissionEntryListItem,
  ImportLogEntry,
} from "@/types";

type FetchPhase = "idle" | "login" | "fetching" | "importing";

function BatchEntryList({ batchId }: { batchId: string }) {
  const [expanded, setExpanded] = useState(false);
  const [rawDialogOpen, setRawDialogOpen] = useState(false);
  const [selectedEntry, setSelectedEntry] = useState<CommissionEntryListItem | undefined>();
  const { data: entries } = useCommissionEntries({
    import_batch_id: batchId,
  });

  if (!entries?.length) return null;

  return (
    <div className="mt-2">
      <button
        type="button"
        className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        onClick={() => setExpanded(!expanded)}
      >
        {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        {entries.length} imported entries
      </button>
      {expanded && (
        <div className="mt-1 max-h-64 overflow-auto rounded border">
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b bg-muted/50">
                <th className="px-2 py-1 text-left font-medium">Name</th>
                <th className="px-2 py-1 text-right font-medium">Amount</th>
                <th className="px-2 py-1 text-left font-medium">Status</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => (
                <tr
                  key={e.id}
                  className="border-b last:border-b-0 hover:bg-muted/25 cursor-pointer"
                  onClick={() => {
                    setSelectedEntry(e);
                    setRawDialogOpen(true);
                  }}
                >
                  <td className="px-2 py-1">{e.client_name ?? e.member_name ?? "—"}</td>
                  <td className="px-2 py-1 text-right font-mono">
                    {e.statement_amount != null ? `$${e.statement_amount.toFixed(2)}` : "—"}
                  </td>
                  <td className="px-2 py-1">{e.status ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <RawDataDialog
        open={rawDialogOpen}
        onOpenChange={setRawDialogOpen}
        entry={selectedEntry}
      />
    </div>
  );
}

export function StatementImportTab() {
  const { data: carriers } = useCarriers();
  const [carrierId, setCarrierId] = useState("");
  const [commissionMonth, setCommissionMonth] = useState("");
  const [filePath, setFilePath] = useState("");
  const [result, setResult] = useState<StatementImportResult | null>(null);

  // Humana fetch state
  const [fetchFromMonth, setFetchFromMonth] = useState("");
  const [fetchThruMonth, setFetchThruMonth] = useState("");
  const [fetchPhase, setFetchPhase] = useState<FetchPhase>("idle");
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [fetchResults, setFetchResults] = useState<StatementImportResult[]>([]);

  // Activity log state
  const [logEntries, setLogEntries] = useState<ImportLogEntry[]>([]);
  const [showLog, setShowLog] = useState(false);
  const logEntriesRef = useRef(logEntries);
  logEntriesRef.current = logEntries;

  const importStatement = useImportCommissionStatement();
  const deleteBatch = useDeleteCommissionBatch();
  const openLogin = useOpenCarrierLogin();
  const triggerFetch = useTriggerCommissionFetch();
  const importCsv = useImportCommissionCsv();

  // Find the Humana carrier ID from the carriers list
  const humanaCarrier = carriers?.find(
    (c) => c.short_name?.toLowerCase() === "humana" || c.name.toLowerCase().includes("humana")
  );

  // Handle incoming commission CSV data from the webview
  const handleCommissionData = useCallback(
    (statementsJson: string) => {
      if (!humanaCarrier) return;
      const carrierId_ = humanaCarrier.id;
      setFetchPhase("importing");
      setFetchError(null);

      let statements: CommissionCsvPayload[];
      try {
        statements = JSON.parse(statementsJson);
      } catch {
        setFetchError("Failed to parse commission data from portal.");
        setFetchPhase("idle");
        return;
      }

      // Import each CSV sequentially, accumulating results across events
      let idx = 0;

      function importNext() {
        if (idx >= statements.length) {
          setFetchPhase("idle");
          return;
        }

        const stmt = statements[idx];
        // Month comes from the JS-scraped date or the backend's CommRunDt detection
        const month = stmt.month || "";

        importCsv.mutate(
          {
            carrierId: carrierId_,
            commissionMonth: month,
            csvContent: stmt.csv,
          },
          {
            onSuccess: (data) => {
              setFetchResults((prev) => [...prev, data]);
              idx++;
              importNext();
            },
            onError: (err) => {
              setFetchError(`Import failed: ${err}`);
              setFetchPhase("idle");
            },
          }
        );
      }

      importNext();
    },
    [humanaCarrier, importCsv]
  );

  // Listen for events from the carrier webview
  useEffect(() => {
    const unlistenData = listen<string>("carrier-commission-data", (event) => {
      handleCommissionData(event.payload);
    });

    const unlistenError = listen<string>("carrier-sync-error", (event) => {
      if (fetchPhase === "fetching") {
        setFetchError(event.payload);
        setFetchPhase("idle");
      }
    });

    const unlistenLog = listen<ImportLogEntry>("commission-import-log", (event) => {
      setLogEntries((prev) => [...prev, event.payload]);
    });

    return () => {
      unlistenData.then((fn) => fn());
      unlistenError.then((fn) => fn());
      unlistenLog.then((fn) => fn());
    };
  }, [handleCommissionData, fetchPhase]);

  const handleOpenPortal = () => {
    setFetchError(null);
    setFetchResults([]);
    setFetchPhase("login");
    openLogin.mutate("carrier-humana", {
      onError: (err) => {
        setFetchError(String(err));
        setFetchPhase("idle");
      },
    });
  };

  /** Convert YYYY-MM pair to MM/DD/YYYY range (first of from-month to last of thru-month) */
  const monthToDateRange = (from: string, thru: string) => {
    const [fy, fm] = from.split("-").map(Number);
    const [ty, tm] = thru.split("-").map(Number);
    const fromDate = `${String(fm).padStart(2, "0")}/01/${fy}`;
    // Last day of thru month
    const lastDay = new Date(ty, tm, 0).getDate();
    const thruDate = `${String(tm).padStart(2, "0")}/${lastDay}/${ty}`;
    return { fromDate, thruDate };
  };

  const handleFetchStatements = () => {
    if (!fetchFromMonth || !fetchThruMonth) return;
    setFetchError(null);
    setFetchResults([]);
    setLogEntries([]);
    setShowLog(true);
    setFetchPhase("fetching");
    const { fromDate, thruDate } = monthToDateRange(fetchFromMonth, fetchThruMonth);
    triggerFetch.mutate({ fromDate, thruDate }, {
      onError: (err) => {
        setFetchError(String(err));
        setFetchPhase("idle");
      },
    });
  };

  const handlePickFile = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Spreadsheet", extensions: ["csv", "xlsx", "xls", "txt"] },
      ],
    } as OpenDialogOptions);
    if (selected) {
      setFilePath(selected as string);
    }
  };

  const handleImport = () => {
    if (!filePath || !carrierId || !commissionMonth) return;
    importStatement.mutate(
      {
        filePath,
        carrierId,
        commissionMonth,
        columnMapping: {},
      },
      {
        onSuccess: (data) => setResult(data),
      }
    );
  };

  return (
    <div className="space-y-6">
      {/* Humana Auto-Fetch Section */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Fetch from Humana</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Open the Humana portal, log in, navigate to the
            Compensation Statements page, then click Fetch Statements.
          </p>

          <div className="flex items-center gap-3 flex-wrap">
            {(fetchPhase === "idle" || fetchPhase === "login") && (
              <>
                <Button variant="outline" onClick={handleOpenPortal}>
                  <Download className="mr-2 h-4 w-4" />
                  Open Humana Portal
                </Button>
                <div className="flex items-center gap-2">
                  <Label className="text-xs text-muted-foreground whitespace-nowrap">From:</Label>
                  <Input
                    type="month"
                    value={fetchFromMonth}
                    onChange={(e) => setFetchFromMonth(e.target.value)}
                    className="w-40 h-8"
                  />
                </div>
                <div className="flex items-center gap-2">
                  <Label className="text-xs text-muted-foreground whitespace-nowrap">To:</Label>
                  <Input
                    type="month"
                    value={fetchThruMonth}
                    onChange={(e) => setFetchThruMonth(e.target.value)}
                    className="w-40 h-8"
                  />
                </div>
                <div className="flex items-center gap-1">
                  {[
                    { label: "3mo", months: 3 },
                    { label: "6mo", months: 6 },
                    { label: "12mo", months: 12 },
                  ].map(({ label, months }) => (
                    <Button
                      key={label}
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-xs"
                      onClick={() => {
                        const now = new Date();
                        const thru = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
                        const from = new Date(now.getFullYear(), now.getMonth() - months + 1, 1);
                        const fromStr = `${from.getFullYear()}-${String(from.getMonth() + 1).padStart(2, "0")}`;
                        setFetchFromMonth(fromStr);
                        setFetchThruMonth(thru);
                      }}
                    >
                      {label}
                    </Button>
                  ))}
                </div>
                <Button
                  onClick={handleFetchStatements}
                  disabled={!fetchFromMonth || !fetchThruMonth}
                >
                  Fetch Statements
                </Button>
              </>
            )}

            {fetchPhase === "fetching" && (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                <span className="text-sm text-muted-foreground">
                  Fetching commission statements...
                </span>
              </>
            )}

            {fetchPhase === "importing" && (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                <span className="text-sm text-muted-foreground">
                  Importing statements...
                </span>
              </>
            )}
          </div>

          {logEntries.length > 0 && (
            <div className="space-y-2">
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs gap-1.5"
                onClick={() => setShowLog(!showLog)}
              >
                <ScrollText className="h-3.5 w-3.5" />
                {showLog ? "Hide" : "Show"} Activity Log ({logEntries.length})
              </Button>
              {showLog && <ActivityLog entries={logEntries} />}
            </div>
          )}

          {fetchError && (
            <p className="text-sm text-red-600">{fetchError}</p>
          )}

          {fetchResults.length > 0 && (
            <div className="space-y-2">
              {fetchResults.map((r, i) => (
                <div key={i} className="grid grid-cols-5 gap-4 text-sm rounded border p-3">
                  <div className="flex items-center gap-1.5">
                    <span className="text-muted-foreground">Total:</span>{" "}
                    <span className="font-medium">{r.total}</span>
                    <button
                      onClick={() => deleteBatch.mutate(r.batch_id)}
                      disabled={deleteBatch.isPending}
                      className="ml-1 text-red-400 hover:text-red-600 disabled:opacity-50"
                      title="Undo import"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Matched:</span>{" "}
                    <span className="font-medium text-green-600">{r.matched}</span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Unmatched:</span>{" "}
                    <span className="font-medium text-yellow-600">{r.unmatched}</span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Skipped:</span>{" "}
                    <span className="font-medium text-muted-foreground">{r.skipped}</span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Errors:</span>{" "}
                    <span className="font-medium text-red-600">{r.errors}</span>
                  </div>
                  {r.unmatched_names.length > 0 && (
                    <div className="col-span-5">
                      <p className="text-xs font-medium mb-1">Unmatched:</p>
                      <ul className="text-xs text-muted-foreground list-disc list-inside">
                        {r.unmatched_names.map((name, j) => (
                          <li key={j}>{name}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                  <div className="col-span-5">
                    <BatchEntryList batchId={r.batch_id} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Manual File Import Section */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Import Carrier Statement</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Carrier</Label>
              <Select value={carrierId} onValueChange={setCarrierId}>
                <SelectTrigger>
                  <SelectValue placeholder="Select carrier" />
                </SelectTrigger>
                <SelectContent>
                  {carriers?.map((c) => (
                    <SelectItem key={c.id} value={c.id}>
                      {c.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label>Commission Month</Label>
              <Input
                type="month"
                value={commissionMonth}
                onChange={(e) => setCommissionMonth(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label>Statement File</Label>
            <div className="flex items-center gap-3">
              <Button variant="outline" onClick={handlePickFile}>
                <Upload className="mr-2 h-4 w-4" />
                Choose File
              </Button>
              {filePath && (
                <span className="text-sm text-muted-foreground truncate max-w-96">
                  {filePath.split("/").pop()}
                </span>
              )}
            </div>
          </div>

          <Button
            onClick={handleImport}
            disabled={!filePath || !carrierId || !commissionMonth || importStatement.isPending}
          >
            {importStatement.isPending ? "Importing..." : "Import Statement"}
          </Button>
        </CardContent>
      </Card>

      {result && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Import Results</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="grid grid-cols-4 gap-4 text-sm">
              <div className="flex items-center gap-1.5">
                <span className="text-muted-foreground">Total Rows:</span>{" "}
                <span className="font-medium">{result.total}</span>
                <button
                  onClick={() => deleteBatch.mutate(result.batch_id)}
                  disabled={deleteBatch.isPending}
                  className="ml-1 text-red-400 hover:text-red-600 disabled:opacity-50"
                  title="Undo import"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
              <div>
                <span className="text-muted-foreground">Matched:</span>{" "}
                <span className="font-medium text-green-600">{result.matched}</span>
              </div>
              <div>
                <span className="text-muted-foreground">Unmatched:</span>{" "}
                <span className="font-medium text-yellow-600">{result.unmatched}</span>
              </div>
              <div>
                <span className="text-muted-foreground">Errors:</span>{" "}
                <span className="font-medium text-red-600">{result.errors}</span>
              </div>
            </div>

            {result.unmatched_names.length > 0 && (
              <div>
                <p className="text-sm font-medium mb-1">Unmatched Members:</p>
                <ul className="text-sm text-muted-foreground list-disc list-inside">
                  {result.unmatched_names.map((name, i) => (
                    <li key={i}>{name}</li>
                  ))}
                </ul>
              </div>
            )}

            {result.error_messages.length > 0 && (
              <div>
                <p className="text-sm font-medium mb-1 text-red-600">Errors:</p>
                <ul className="text-sm text-red-600 list-disc list-inside">
                  {result.error_messages.map((msg, i) => (
                    <li key={i}>{msg}</li>
                  ))}
                </ul>
              </div>
            )}

            <BatchEntryList batchId={result.batch_id} />
          </CardContent>
        </Card>
      )}
    </div>
  );
}
