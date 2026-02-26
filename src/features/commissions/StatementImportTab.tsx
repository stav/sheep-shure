import { useState } from "react";
import { Upload, Trash2 } from "lucide-react";
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
} from "@/hooks";
import type { StatementImportResult } from "@/types";

export function StatementImportTab() {
  const { data: carriers } = useCarriers();
  const [carrierId, setCarrierId] = useState("");
  const [commissionMonth, setCommissionMonth] = useState("");
  const [filePath, setFilePath] = useState("");
  const [result, setResult] = useState<StatementImportResult | null>(null);

  const importStatement = useImportCommissionStatement();
  const deleteBatch = useDeleteCommissionBatch();

  const handlePickFile = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      multiple: false,
      filters: [
        { name: "Spreadsheet", extensions: ["csv", "xlsx", "xls"] },
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
