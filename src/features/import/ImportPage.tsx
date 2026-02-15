import { useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { tauriInvoke } from "@/lib/tauri";
import { toast } from "sonner";
import { Upload, FileSpreadsheet, CheckCircle2, AlertCircle, Loader2, ArrowRight, ArrowLeft, Check } from "lucide-react";

type Step = "select" | "map" | "validate" | "result";

interface ParseResult {
  headers: string[];
  sample_rows: string[][];
  total_rows: number;
  auto_mapping: Record<string, string>;
}

interface ValidationResult {
  valid_rows: string[][];
  error_rows: { row_number: number; data: string[]; errors: string[] }[];
  total: number;
}

interface ImportResultData {
  inserted: number;
  updated: number;
  skipped: number;
  errors: number;
  total: number;
}

const TARGET_FIELDS = [
  "first_name", "last_name", "middle_name", "dob", "gender",
  "phone", "phone2", "email", "address_line1", "address_line2",
  "city", "state", "zip", "county", "mbi", "part_a_date", "part_b_date",
  "plan_name", "carrier_name", "plan_type_code", "effective_date",
  "termination_date", "premium", "contract_number", "pbp_number",
  "confirmation_number", "lead_source", "dual_status_code", "lis_level", "medicaid_id",
];

export function ImportPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [step, setStep] = useState<Step>("select");
  const [filePath, setFilePath] = useState("");
  const [parseResult, setParseResult] = useState<ParseResult | null>(null);
  const [mapping, setMapping] = useState<Record<string, string>>({});
  const [validation, setValidation] = useState<ValidationResult | null>(null);
  const [importResult, setImportResult] = useState<ImportResultData | null>(null);
  const [loading, setLoading] = useState(false);

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

  const handleValidate = useCallback(async () => {
    if (!filePath || !mapping) return;
    setLoading(true);
    try {
      const result = await tauriInvoke<ValidationResult>("validate_import", {
        filePath,
        columnMapping: mapping,
      });
      setValidation(result);
      setStep("validate");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Validation failed");
    } finally {
      setLoading(false);
    }
  }, [filePath, mapping]);

  const handleImport = useCallback(async () => {
    if (!filePath || !mapping) return;
    setLoading(true);
    try {
      const result = await tauriInvoke<ImportResultData>("execute_import", {
        filePath,
        columnMapping: mapping,
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
  }, [filePath, mapping, queryClient]);

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
      <div>
        <h1 className="text-2xl font-bold">Import Data</h1>
        <p className="text-sm text-muted-foreground">
          Import clients from carrier CSV or XLSX files
        </p>
      </div>

      {/* Step indicator */}
      <div className="flex items-center gap-2 text-sm">
        {(["select", "map", "validate", "result"] as Step[]).map((s, i) => (
          <div key={s} className="flex items-center gap-2">
            {i > 0 && <div className="h-px w-8 bg-border" />}
            <div className={`flex items-center gap-1.5 ${step === s ? "text-primary font-medium" : "text-muted-foreground"}`}>
              <div className={`h-6 w-6 rounded-full flex items-center justify-center text-xs ${
                step === s ? "bg-primary text-primary-foreground" :
                (["select", "map", "validate", "result"].indexOf(step) > i ? "bg-primary/20 text-primary" : "bg-muted")
              }`}>
                {i + 1}
              </div>
              {["Select File", "Map Columns", "Validate", "Results"][i]}
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
                      className="flex h-9 w-1/3 rounded-md border border-input bg-background px-3 py-1 text-sm"
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
              <Button onClick={handleValidate} disabled={loading || Object.keys(mapping).length === 0}>
                {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                Validate
                <ArrowRight className="ml-2 h-4 w-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Step 3: Validate */}
      {step === "validate" && validation && (
        <Card>
          <CardHeader>
            <CardTitle>Validation Results</CardTitle>
            <CardDescription>
              {validation.valid_rows.length} valid rows, {validation.error_rows.length} errors
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex gap-4">
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle2 className="h-4 w-4 text-green-500" />
                <span className="font-medium">{validation.valid_rows.length}</span> ready to import
              </div>
              {validation.error_rows.length > 0 && (
                <div className="flex items-center gap-2 text-sm">
                  <AlertCircle className="h-4 w-4 text-red-500" />
                  <span className="font-medium">{validation.error_rows.length}</span> will be skipped
                </div>
              )}
            </div>

            {validation.error_rows.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-sm font-medium">Errors</h3>
                <div className="max-h-48 overflow-y-auto rounded border">
                  {validation.error_rows.slice(0, 20).map((row) => (
                    <div key={row.row_number} className="px-3 py-2 border-b text-xs">
                      <span className="font-medium">Row {row.row_number}:</span>{" "}
                      {row.errors.join("; ")}
                    </div>
                  ))}
                  {validation.error_rows.length > 20 && (
                    <div className="px-3 py-2 text-xs text-muted-foreground">
                      ... and {validation.error_rows.length - 20} more errors
                    </div>
                  )}
                </div>
              </div>
            )}

            <div className="flex items-center gap-2 pt-4">
              <Button variant="outline" onClick={() => setStep("map")}>
                <ArrowLeft className="mr-2 h-4 w-4" /> Back
              </Button>
              <Button
                onClick={handleImport}
                disabled={loading || validation.valid_rows.length === 0}
              >
                {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <FileSpreadsheet className="mr-2 h-4 w-4" />}
                Import {validation.valid_rows.length} Rows
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
              <div className="text-center p-4 rounded-lg bg-green-50 dark:bg-green-950/30">
                <div className="text-2xl font-bold text-green-600">{importResult.inserted}</div>
                <div className="text-sm text-green-600">Inserted</div>
              </div>
              <div className="text-center p-4 rounded-lg bg-blue-50 dark:bg-blue-950/30">
                <div className="text-2xl font-bold text-blue-600">{importResult.updated}</div>
                <div className="text-sm text-blue-600">Updated</div>
              </div>
              <div className="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800/30">
                <div className="text-2xl font-bold text-gray-600">{importResult.skipped}</div>
                <div className="text-sm text-gray-600">Skipped</div>
              </div>
              <div className="text-center p-4 rounded-lg bg-red-50 dark:bg-red-950/30">
                <div className="text-2xl font-bold text-red-600">{importResult.errors}</div>
                <div className="text-sm text-red-600">Errors</div>
              </div>
            </div>

            <div className="flex items-center gap-2 pt-4">
              <Button onClick={() => navigate("/clients")}>
                View Clients
              </Button>
              <Button variant="outline" onClick={() => { setStep("select"); setFilePath(""); setParseResult(null); setMapping({}); setValidation(null); setImportResult(null); }}>
                Import Another File
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
