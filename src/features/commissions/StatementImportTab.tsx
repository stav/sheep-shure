import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Upload, Trash2, Download, Loader2, ScrollText } from "lucide-react";
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
  useImportCommissionStatement,
  useDeleteCommissionBatch,
  useTriggerCommissionFetch,
  useImportCommissionCsv,
} from "@/hooks";
import { useOpenCarrierLogin } from "@/hooks/useCarrierSync";
import { ActivityLog } from "./ActivityLog";
import type {
  StatementImportResult,
  CommissionCsvPayload,
  ImportLogEntry,
} from "@/types";

type FetchPhase = "idle" | "login" | "fetching" | "importing";

export function StatementImportTab() {
  const { data: carriers } = useCarriers();
  const [carrierId, setCarrierId] = useState("");
  const [commissionMonth, setCommissionMonth] = useState("");
  const [filePath, setFilePath] = useState("");
  const [result, setResult] = useState<StatementImportResult | null>(null);

  // Humana fetch state
  const [fetchMonth, setFetchMonth] = useState("");
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

      // Import each CSV sequentially
      const results: StatementImportResult[] = [];
      const skipped: string[] = [];
      let idx = 0;

      function importNext() {
        if (idx >= statements.length) {
          setFetchResults(results);
          if (skipped.length > 0) {
            setFetchError(`Skipped ${skipped.length} statement(s) with no month detected: ${skipped.join(", ")}. Set a fallback month and retry.`);
          }
          setFetchPhase("idle");
          return;
        }

        const stmt = statements[idx];
        const month = stmt.month || fetchMonth || "";
        if (!month) {
          skipped.push(`Statement ${idx + 1}`);
          idx++;
          importNext();
          return;
        }

        importCsv.mutate(
          {
            carrierId: carrierId_,
            commissionMonth: month,
            csvContent: stmt.csv,
          },
          {
            onSuccess: (data) => {
              results.push(data);
              idx++;
              importNext();
            },
            onError: (err) => {
              setFetchError(`Statement ${idx + 1} import failed: ${err}`);
              setFetchResults(results);
              setFetchPhase("idle");
            },
          }
        );
      }

      importNext();
    },
    [humanaCarrier, fetchMonth, importCsv]
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

  const handleFetchStatements = () => {
    setFetchError(null);
    setLogEntries([]);
    setShowLog(true);
    setFetchPhase("fetching");
    triggerFetch.mutate(undefined, {
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
                  <Label className="text-xs text-muted-foreground whitespace-nowrap">Fallback month:</Label>
                  <Input
                    type="month"
                    value={fetchMonth}
                    onChange={(e) => setFetchMonth(e.target.value)}
                    className="w-40 h-8"
                  />
                </div>
                <Button onClick={handleFetchStatements}>
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
                  <div>
                    <span className="text-muted-foreground">Total:</span>{" "}
                    <span className="font-medium">{r.total}</span>
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
                    <div className="col-span-4">
                      <p className="text-xs font-medium mb-1">Unmatched:</p>
                      <ul className="text-xs text-muted-foreground list-disc list-inside">
                        {r.unmatched_names.map((name, j) => (
                          <li key={j}>{name}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                  <div className="col-span-4">
                    <Button
                      variant="destructive"
                      size="sm"
                      onClick={() => deleteBatch.mutate(r.batch_id)}
                      disabled={deleteBatch.isPending}
                    >
                      <Trash2 className="mr-2 h-3 w-3" />
                      Undo
                    </Button>
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
              <div>
                <span className="text-muted-foreground">Total Rows:</span>{" "}
                <span className="font-medium">{result.total}</span>
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

            <Button
              variant="destructive"
              size="sm"
              onClick={() => deleteBatch.mutate(result.batch_id)}
              disabled={deleteBatch.isPending}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Undo Import
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
