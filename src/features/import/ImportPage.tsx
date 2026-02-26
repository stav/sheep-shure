import { useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { tauriInvoke } from "@/lib/tauri";
import { toast } from "sonner";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { Upload, FileSpreadsheet, CheckCircle2, AlertCircle, Loader2, ArrowRight, ArrowLeft, Check, Plus, X, ChevronDown, ChevronRight, Minus } from "lucide-react";

type Step = "select" | "map" | "review" | "result";

interface ParseResult {
  headers: string[];
  sample_rows: string[][];
  total_rows: number;
  auto_mapping: Record<string, string>;
}

interface ImportRowDetail {
  label: string;
  detail: string;
}

interface ImportResultData {
  inserted: number;
  updated: number;
  skipped: number;
  errors: number;
  total: number;
  inserted_details: ImportRowDetail[];
  updated_details: ImportRowDetail[];
  skipped_details: ImportRowDetail[];
  errors_details: ImportRowDetail[];
}

interface ImportPreview {
  inserts: PreviewInsert[];
  updates: PreviewUpdate[];
  skipped: PreviewSkipped[];
  errors: { row_number: number; data: string[]; errors: string[] }[];
}

interface PreviewInsert {
  row_index: number;
  name: string;
}

interface PreviewUpdate {
  row_index: number;
  client_id: string;
  name: string;
  diffs: FieldDiff[];
}

interface FieldDiff {
  field: string;
  old_value: string;
  new_value: string;
}

interface PreviewSkipped {
  row_index: number;
  name: string;
  reason: string;
}

const TARGET_FIELDS = [
  "first_name", "last_name", "middle_name", "dob", "gender",
  "phone", "phone2", "email", "address_line1", "address_line2",
  "city", "state", "zip", "county", "mbi", "part_a_date", "part_b_date",
  "plan_name", "carrier_name", "plan_type_code", "effective_date",
  "termination_date", "premium", "contract_number", "pbp_number",
  "confirmation_number", "lead_source", "dual_status_code", "lis_level", "medicaid_id", "notes",
];

function fieldLabel(field: string): string {
  return field.replace(/_/g, " ");
}

export function ImportPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [step, setStep] = useState<Step>("select");
  const [filePath, setFilePath] = useState("");
  const [parseResult, setParseResult] = useState<ParseResult | null>(null);
  const [mapping, setMapping] = useState<Record<string, string>>({});
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [approvedInserts, setApprovedInserts] = useState<Set<number>>(new Set());
  const [approvedUpdates, setApprovedUpdates] = useState<Record<string, Set<string>>>({});
  const [expandedClients, setExpandedClients] = useState<Set<string>>(new Set());
  const [importResult, setImportResult] = useState<ImportResultData | null>(null);
  const [loading, setLoading] = useState(false);
  const [detailCategory, setDetailCategory] = useState<string | null>(null);
  const [constantMappings, setConstantMappings] = useState<{ value: string; field: string }[]>([]);

  const handleSelectFile = useCallback(async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: [{ name: "Spreadsheets", extensions: ["csv", "xlsx", "xls"] }],
      });
      if (selected && typeof selected === "string") {
        setFilePath(selected);
        setLoading(true);
        const result = await tauriInvoke<ParseResult>("parse_import_file", { filePath: selected });
        setParseResult(result);
        setMapping(result.auto_mapping);
        setStep("map");
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to parse file");
    } finally {
      setLoading(false);
    }
  }, []);

  const handlePreview = useCallback(async () => {
    if (!filePath || !mapping) return;
    setLoading(true);
    try {
      const constants: Record<string, string> = {};
      for (const cm of constantMappings) {
        if (cm.value && cm.field) constants[cm.field] = cm.value;
      }
      const result = await tauriInvoke<ImportPreview>("preview_import", {
        filePath,
        columnMapping: mapping,
        constantValues: Object.keys(constants).length > 0 ? constants : null,
      });
      setPreview(result);
      // Initialize all inserts as approved
      setApprovedInserts(new Set(result.inserts.map((ins) => ins.row_index)));
      // Initialize all diffs as approved
      const initial: Record<string, Set<string>> = {};
      for (const u of result.updates) {
        initial[u.client_id] = new Set(u.diffs.map((d) => d.field));
      }
      setApprovedUpdates(initial);
      setExpandedClients(new Set());
      setStep("review");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Preview failed");
    } finally {
      setLoading(false);
    }
  }, [filePath, mapping, constantMappings]);

  const handleImport = useCallback(async () => {
    if (!filePath || !mapping) return;
    setLoading(true);
    try {
      const constants: Record<string, string> = {};
      for (const cm of constantMappings) {
        if (cm.value && cm.field) constants[cm.field] = cm.value;
      }
      // Serialize approved updates: only include clients with at least one approved field
      const serializedApproved: Record<string, string[]> = {};
      for (const [clientId, fields] of Object.entries(approvedUpdates)) {
        if (fields.size > 0) {
          serializedApproved[clientId] = Array.from(fields);
        }
      }
      const result = await tauriInvoke<ImportResultData>("execute_import", {
        filePath,
        columnMapping: mapping,
        constantValues: Object.keys(constants).length > 0 ? constants : null,
        approvedUpdates: serializedApproved,
        approvedInserts: Array.from(approvedInserts),
      });
      setImportResult(result);
      setStep("result");
      queryClient.invalidateQueries({ queryKey: ["clients"] });
      queryClient.invalidateQueries({ queryKey: ["dashboard-stats"] });
      toast.success(`Imported ${result.inserted} new clients, updated ${result.updated}`);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Import failed");
    } finally {
      setLoading(false);
    }
  }, [filePath, mapping, constantMappings, approvedInserts, approvedUpdates, queryClient]);

  const toggleClientApproval = (clientId: string, allFields: string[]) => {
    setApprovedUpdates((prev) => {
      const current = prev[clientId];
      if (current && current.size > 0) {
        return { ...prev, [clientId]: new Set<string>() };
      }
      return { ...prev, [clientId]: new Set(allFields) };
    });
  };

  const toggleFieldApproval = (clientId: string, field: string) => {
    setApprovedUpdates((prev) => {
      const current = new Set(prev[clientId] || []);
      if (current.has(field)) {
        current.delete(field);
      } else {
        current.add(field);
      }
      return { ...prev, [clientId]: current };
    });
  };

  const toggleExpanded = (clientId: string) => {
    setExpandedClients((prev) => {
      const next = new Set(prev);
      if (next.has(clientId)) {
        next.delete(clientId);
      } else {
        next.add(clientId);
      }
      return next;
    });
  };

  const approvedInsertCount = approvedInserts.size;
  const approvedUpdateCount = preview
    ? preview.updates.filter((u) => (approvedUpdates[u.client_id]?.size ?? 0) > 0).length
    : 0;

  const updateMapping = (sourceCol: string, targetField: string) => {
    setMapping((prev) => {
      const next = { ...prev };
      if (targetField === "") {
        delete next[sourceCol];
      } else {
        next[sourceCol] = targetField;
      }
      return next;
    });
  };

  return (
    <div className="space-y-6 max-w-4xl">
      {/* Step indicator */}
      <div className="flex items-center gap-2 text-sm">
        {(["select", "map", "review", "result"] as Step[]).map((s, i) => (
          <div key={s} className="flex items-center gap-2">
            {i > 0 && <div className="h-px w-8 bg-border" />}
            <div className={`flex items-center gap-1.5 ${step === s ? "text-primary font-medium" : "text-muted-foreground"}`}>
              <div className={`h-6 w-6 rounded-full flex items-center justify-center text-xs ${
                step === s ? "bg-primary text-primary-foreground" :
                (["select", "map", "review", "result"].indexOf(step) > i ? "bg-primary/20 text-primary" : "bg-muted")
              }`}>
                {i + 1}
              </div>
              {["Select File", "Map Columns", "Review", "Results"][i]}
            </div>
          </div>
        ))}
      </div>

      {/* Step 1: Select File */}
      {step === "select" && (
        <Card>
          <CardHeader>
            <CardTitle>Select File</CardTitle>
            <CardDescription>Choose a CSV or XLSX file exported from a carrier portal</CardDescription>
          </CardHeader>
          <CardContent>
            <div
              onClick={handleSelectFile}
              className="border-2 border-dashed rounded-lg p-12 text-center cursor-pointer hover:border-primary hover:bg-primary/5 transition-colors"
            >
              {loading ? (
                <Loader2 className="mx-auto h-12 w-12 animate-spin text-muted-foreground" />
              ) : (
                <>
                  <Upload className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
                  <p className="text-lg font-medium">Click to select a file</p>
                  <p className="text-sm text-muted-foreground mt-1">Supports CSV, XLS, and XLSX formats</p>
                </>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Step 2: Map Columns */}
      {step === "map" && parseResult && (
        <Card>
          <CardHeader>
            <CardTitle>Map Columns</CardTitle>
            <CardDescription>
              {parseResult.total_rows} rows found. Match source columns to client fields.
              Green = auto-mapped.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              {parseResult.headers.map((header) => {
                const isMapped = !!mapping[header];
                return (
                  <div key={header} className="flex items-center gap-4">
                    <div className={`w-1/3 text-sm font-medium truncate ${isMapped ? "text-green-600" : "text-muted-foreground"}`}>
                      {isMapped && <Check className="inline h-3 w-3 mr-1" />}
                      {header}
                    </div>
                    <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                    <select
                      value={mapping[header] || ""}
                      onChange={(e) => updateMapping(header, e.target.value)}
                      className="flex h-9 w-1/3 rounded-md border border-input bg-background text-foreground px-3 py-1 text-sm"
                    >
                      <option value="">-- Skip --</option>
                      {TARGET_FIELDS.map((f) => (
                        <option key={f} value={f}>{f.replace(/_/g, " ")}</option>
                      ))}
                    </select>
                  </div>
                );
              })}
            </div>

            {/* Constant value mappings */}
            {constantMappings.map((cm, i) => (
              <div key={i} className="flex items-center gap-4">
                <input
                  type="text"
                  value={cm.value}
                  onChange={(e) => setConstantMappings((prev) => prev.map((m, j) => j === i ? { ...m, value: e.target.value } : m))}
                  placeholder="Value for all rows..."
                  className="flex h-9 w-1/3 rounded-md border border-input bg-background px-3 py-1 text-sm"
                />
                <ArrowRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                <select
                  value={cm.field}
                  onChange={(e) => setConstantMappings((prev) => prev.map((m, j) => j === i ? { ...m, field: e.target.value } : m))}
                  className="flex h-9 w-1/3 rounded-md border border-input bg-background text-foreground px-3 py-1 text-sm"
                >
                  <option value="">-- Select field --</option>
                  {TARGET_FIELDS.map((f) => (
                    <option key={f} value={f}>{f.replace(/_/g, " ")}</option>
                  ))}
                </select>
                <button
                  onClick={() => setConstantMappings((prev) => prev.filter((_, j) => j !== i))}
                  className="text-muted-foreground hover:text-destructive"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
            ))}
            <Button
              variant="outline"
              size="sm"
              onClick={() => setConstantMappings((prev) => [...prev, { value: "", field: "" }])}
            >
              <Plus className="mr-2 h-4 w-4" /> Add Constant Value
            </Button>

            {/* Sample data preview */}
            {parseResult.sample_rows.length > 0 && (
              <div className="mt-6">
                <h3 className="text-sm font-medium mb-2">Sample Data (first {parseResult.sample_rows.length} rows)</h3>
                <div className="overflow-x-auto rounded border">
                  <table className="text-xs w-full">
                    <thead>
                      <tr className="bg-muted/50">
                        {parseResult.headers.map((h) => (
                          <th key={h} className="px-2 py-1 text-left font-medium whitespace-nowrap">{h}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {parseResult.sample_rows.slice(0, 5).map((row, i) => (
                        <tr key={i} className="border-t">
                          {row.map((cell, j) => (
                            <td key={j} className="px-2 py-1 whitespace-nowrap">{cell || "--"}</td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            <div className="flex items-center gap-2 pt-4">
              <Button variant="outline" onClick={() => setStep("select")}>
                <ArrowLeft className="mr-2 h-4 w-4" /> Back
              </Button>
              <Button onClick={handlePreview} disabled={loading || Object.keys(mapping).length === 0}>
                {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                Review
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Step 3: Review */}
      {step === "review" && preview && (
        <Card>
          <CardHeader>
            <CardTitle>Review Import</CardTitle>
            <CardDescription>
              Review what will happen before importing. Uncheck fields or clients to skip them.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Summary bar */}
            <div className="flex flex-wrap gap-3">
              <div className="flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-md bg-green-50 text-green-700 dark:bg-green-950/30 dark:text-green-400">
                <Plus className="h-3.5 w-3.5" />
                <span className="font-medium">{preview.inserts.length}</span> new
              </div>
              <div className="flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-md bg-blue-50 text-blue-700 dark:bg-blue-950/30 dark:text-blue-400">
                <FileSpreadsheet className="h-3.5 w-3.5" />
                <span className="font-medium">{preview.updates.length}</span> to update
              </div>
              <div className="flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-md bg-gray-100 text-gray-600 dark:bg-gray-800/30 dark:text-gray-400">
                <Minus className="h-3.5 w-3.5" />
                <span className="font-medium">{preview.skipped.length}</span> unchanged
              </div>
              {preview.errors.length > 0 && (
                <div className="flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-md bg-red-50 text-red-700 dark:bg-red-950/30 dark:text-red-400">
                  <AlertCircle className="h-3.5 w-3.5" />
                  <span className="font-medium">{preview.errors.length}</span> errors
                </div>
              )}
            </div>

            {/* Errors section */}
            {preview.errors.length > 0 && (
              <CollapsibleSection title="Errors" count={preview.errors.length} defaultOpen={false}>
                <div className="max-h-48 overflow-y-auto rounded border">
                  {preview.errors.slice(0, 20).map((row) => (
                    <div key={row.row_number} className="px-3 py-2 border-b last:border-b-0 text-xs">
                      <span className="font-medium">Row {row.row_number}:</span>{" "}
                      {row.errors.join("; ")}
                    </div>
                  ))}
                  {preview.errors.length > 20 && (
                    <div className="px-3 py-2 text-xs text-muted-foreground">
                      ... and {preview.errors.length - 20} more errors
                    </div>
                  )}
                </div>
              </CollapsibleSection>
            )}

            {/* New clients section */}
            {preview.inserts.length > 0 && (
              <CollapsibleSection
                title="New clients"
                count={preview.inserts.length}
                defaultOpen={false}
                selectedCount={approvedInsertCount}
                onSelectAll={() => setApprovedInserts(new Set(preview.inserts.map((ins) => ins.row_index)))}
                onSelectNone={() => setApprovedInserts(new Set())}
              >
                <div className="max-h-64 overflow-y-auto rounded border divide-y">
                  {preview.inserts.map((ins) => (
                    <label key={ins.row_index} className="flex items-center gap-3 px-3 py-1.5 text-sm hover:bg-muted/50 cursor-pointer transition-colors">
                      <input
                        type="checkbox"
                        checked={approvedInserts.has(ins.row_index)}
                        onChange={() => {
                          setApprovedInserts((prev) => {
                            const next = new Set(prev);
                            if (next.has(ins.row_index)) next.delete(ins.row_index);
                            else next.add(ins.row_index);
                            return next;
                          });
                        }}
                        className="h-4 w-4 rounded border-gray-300"
                      />
                      {ins.name}
                    </label>
                  ))}
                </div>
              </CollapsibleSection>
            )}

            {/* Updates section */}
            {preview.updates.length > 0 && (
              <CollapsibleSection
                title="Updates"
                count={preview.updates.length}
                defaultOpen
                selectedCount={approvedUpdateCount}
                onSelectAll={() => {
                  const all: Record<string, Set<string>> = {};
                  for (const u of preview.updates) {
                    all[u.client_id] = new Set(u.diffs.map((d) => d.field));
                  }
                  setApprovedUpdates(all);
                }}
                onSelectNone={() => {
                  const none: Record<string, Set<string>> = {};
                  for (const u of preview.updates) {
                    none[u.client_id] = new Set<string>();
                  }
                  setApprovedUpdates(none);
                }}
              >
                <div className="rounded border divide-y">
                  {preview.updates.map((upd) => {
                    const approved = approvedUpdates[upd.client_id] ?? new Set<string>();
                    const allFields = upd.diffs.map((d) => d.field);
                    const allChecked = allFields.every((f) => approved.has(f));
                    const someChecked = allFields.some((f) => approved.has(f));
                    const isExpanded = expandedClients.has(upd.client_id);

                    return (
                      <div key={upd.client_id}>
                        <div
                          className="flex items-center gap-3 px-3 py-2 cursor-pointer hover:bg-muted/50 transition-colors"
                          onClick={() => toggleExpanded(upd.client_id)}
                        >
                          <input
                            type="checkbox"
                            checked={allChecked}
                            ref={(el) => { if (el) el.indeterminate = someChecked && !allChecked; }}
                            onChange={(e) => { e.stopPropagation(); toggleClientApproval(upd.client_id, allFields); }}
                            onClick={(e) => e.stopPropagation()}
                            className="h-4 w-4 rounded border-gray-300"
                          />
                          <span className="text-sm font-medium flex-1">{upd.name}</span>
                          <span className="text-xs text-muted-foreground px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/40">
                            {upd.diffs.length} {upd.diffs.length === 1 ? "change" : "changes"}
                          </span>
                          {isExpanded
                            ? <ChevronDown className="h-4 w-4 text-muted-foreground" />
                            : <ChevronRight className="h-4 w-4 text-muted-foreground" />
                          }
                        </div>
                        {isExpanded && (
                          <div className="px-3 pb-3">
                            <table className="w-full text-sm">
                              <thead>
                                <tr className="text-xs text-muted-foreground">
                                  <th className="w-8 py-1" />
                                  <th className="text-left py-1 font-medium">Field</th>
                                  <th className="text-left py-1 font-medium">Current</th>
                                  <th className="w-8 py-1" />
                                  <th className="text-left py-1 font-medium">New</th>
                                </tr>
                              </thead>
                              <tbody>
                                {upd.diffs.map((diff) => (
                                  <tr key={diff.field} className="border-t border-border/50">
                                    <td className="py-1.5 pr-2">
                                      <input
                                        type="checkbox"
                                        checked={approved.has(diff.field)}
                                        onChange={() => toggleFieldApproval(upd.client_id, diff.field)}
                                        className="h-3.5 w-3.5 rounded border-gray-300"
                                      />
                                    </td>
                                    <td className="py-1.5 pr-3 text-muted-foreground capitalize whitespace-nowrap">
                                      {fieldLabel(diff.field)}
                                    </td>
                                    <td className="py-1.5 pr-2">
                                      <span className="text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 px-1.5 py-0.5 rounded text-xs">
                                        {diff.old_value || "(empty)"}
                                      </span>
                                    </td>
                                    <td className="py-1.5 px-1">
                                      <ArrowRight className="h-3 w-3 text-muted-foreground" />
                                    </td>
                                    <td className="py-1.5">
                                      <span className="text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950/30 px-1.5 py-0.5 rounded text-xs">
                                        {diff.new_value || "(empty)"}
                                      </span>
                                    </td>
                                  </tr>
                                ))}
                              </tbody>
                            </table>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </CollapsibleSection>
            )}

            {/* Unchanged section */}
            {preview.skipped.length > 0 && (
              <CollapsibleSection title="Unchanged" count={preview.skipped.length} defaultOpen={false}>
                <div className="max-h-48 overflow-y-auto rounded border divide-y">
                  {preview.skipped.map((sk) => (
                    <div key={sk.row_index} className="px-3 py-1.5 text-sm text-muted-foreground">
                      {sk.name}
                    </div>
                  ))}
                </div>
              </CollapsibleSection>
            )}

            <div className="flex items-center gap-2 pt-4">
              <Button variant="outline" onClick={() => setStep("map")}>
                <ArrowLeft className="mr-2 h-4 w-4" /> Back
              </Button>
              <Button
                onClick={handleImport}
                disabled={loading || (approvedInsertCount === 0 && approvedUpdateCount === 0)}
              >
                {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <FileSpreadsheet className="mr-2 h-4 w-4" />}
                Import {approvedInsertCount} new + {approvedUpdateCount} updates
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Step 4: Results */}
      {step === "result" && importResult && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <CheckCircle2 className="h-5 w-5 text-green-500" />
              Import Complete
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {([
                { key: "inserted", label: "Inserted", count: importResult.inserted, bg: "bg-green-50 dark:bg-green-950/30", text: "text-green-600", hoverBg: "hover:bg-green-100 dark:hover:bg-green-950/50" },
                { key: "updated", label: "Updated", count: importResult.updated, bg: "bg-blue-50 dark:bg-blue-950/30", text: "text-blue-600", hoverBg: "hover:bg-blue-100 dark:hover:bg-blue-950/50" },
                { key: "skipped", label: "Skipped", count: importResult.skipped, bg: "bg-gray-50 dark:bg-gray-800/30", text: "text-gray-600", hoverBg: "hover:bg-gray-100 dark:hover:bg-gray-800/50" },
                { key: "errors", label: "Errors", count: importResult.errors, bg: "bg-red-50 dark:bg-red-950/30", text: "text-red-600", hoverBg: "hover:bg-red-100 dark:hover:bg-red-950/50" },
              ] as const).map(({ key, label, count, bg, text, hoverBg }) => (
                <div
                  key={key}
                  onClick={count > 0 ? () => setDetailCategory(key) : undefined}
                  className={`text-center p-4 rounded-lg ${bg} ${count > 0 ? `${hoverBg} cursor-pointer transition-colors` : "opacity-60"}`}
                >
                  <div className={`text-2xl font-bold ${text}`}>{count}</div>
                  <div className={`text-sm ${text}`}>{label}</div>
                </div>
              ))}
            </div>

            <p className="text-xs text-muted-foreground">Click a box to see per-row details</p>

            <div className="flex items-center gap-2 pt-4">
              <Button onClick={() => navigate("/clients")}>
                View Clients
              </Button>
              <Button variant="outline" onClick={() => { setStep("select"); setFilePath(""); setParseResult(null); setMapping({}); setPreview(null); setApprovedInserts(new Set()); setApprovedUpdates({}); setExpandedClients(new Set()); setImportResult(null); setConstantMappings([]); }}>
                Import Another File
              </Button>
            </div>
          </CardContent>

          <Dialog open={detailCategory !== null} onOpenChange={(open) => { if (!open) setDetailCategory(null); }}>
            <DialogContent className="max-w-lg max-h-[80vh] flex flex-col">
              <DialogHeader>
                <DialogTitle className="capitalize">{detailCategory} Details</DialogTitle>
                <DialogDescription>
                  {detailCategory && importResult[`${detailCategory}_details` as keyof ImportResultData] ?
                    `${(importResult[`${detailCategory}_details` as keyof ImportResultData] as ImportRowDetail[]).length} rows` : ""}
                </DialogDescription>
              </DialogHeader>
              <div className="overflow-y-auto min-h-0 -mx-6 px-6">
                {detailCategory && (() => {
                  const details = importResult[`${detailCategory}_details` as keyof ImportResultData] as ImportRowDetail[] | undefined;
                  if (!details || details.length === 0) return <p className="text-sm text-muted-foreground">No details available.</p>;
                  return (
                    <div className="rounded border divide-y">
                      {details.map((row, i) => (
                        <div key={i} className="px-3 py-2 text-sm">
                          <span className="font-medium">{row.label}</span>
                          {row.detail && <span className="text-muted-foreground ml-2">— {row.detail}</span>}
                        </div>
                      ))}
                    </div>
                  );
                })()}
              </div>
            </DialogContent>
          </Dialog>
        </Card>
      )}
    </div>
  );
}

function CollapsibleSection({ title, count, defaultOpen, selectedCount, onSelectAll, onSelectNone, children }: {
  title: string;
  count: number;
  defaultOpen: boolean;
  selectedCount?: number;
  onSelectAll?: () => void;
  onSelectNone?: () => void;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const hasSelection = selectedCount !== undefined && onSelectAll && onSelectNone;
  return (
    <div>
      <div className="flex items-center gap-2 py-1">
        <button
          onClick={() => setOpen(!open)}
          className="flex items-center gap-2 text-sm font-medium text-left hover:text-primary transition-colors"
        >
          {open ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
          {title}
          <span className="text-xs text-muted-foreground font-normal">
            ({hasSelection ? `${selectedCount}/${count}` : count})
          </span>
        </button>
        {open && hasSelection && (
          <span className="text-xs text-muted-foreground ml-auto">
            <button onClick={onSelectAll} className="hover:text-primary transition-colors">All</button>
            {" / "}
            <button onClick={onSelectNone} className="hover:text-primary transition-colors">None</button>
          </span>
        )}
      </div>
      {open && <div className="mt-1">{children}</div>}
    </div>
  );
}
