import { useState } from "react";
import { Users, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { useImportPortalMembers } from "@/hooks/useCarrierSync";
import { isPortalMemberActive } from "./utils";
import type { PortalMember, ImportPortalResult } from "@/types";

export function NewInPortalSection({
  members,
  carrierId,
  onImported,
}: {
  members: PortalMember[];
  carrierId: string;
  onImported: (result: ImportPortalResult, importedMembers: PortalMember[]) => void;
}) {
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(new Set());
  const [importResult, setImportResult] = useState<ImportPortalResult | null>(null);
  const importMembers = useImportPortalMembers();

  if (members.length === 0) return null;

  const toggleSelect = (index: number) => {
    setSelectedIndices((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  };

  const toggleAll = () => {
    if (selectedIndices.size === members.length) {
      setSelectedIndices(new Set());
    } else {
      setSelectedIndices(new Set(members.map((_, i) => i)));
    }
  };

  const handleImport = () => {
    const selected = members.filter((_, i) => selectedIndices.has(i));
    if (selected.length === 0) return;

    setImportResult(null);
    importMembers.mutate(
      { carrierId, membersJson: JSON.stringify(selected) },
      {
        onSuccess: (res) => {
          setImportResult(res);
          setSelectedIndices(new Set());
          onImported(res, selected);
        },
        onError: (err) => {
          setImportResult({ imported: 0, imported_names: [], errors: [String(err)] });
        },
      }
    );
  };

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <h4 className="flex items-center gap-2 text-sm font-medium">
          <Users className="h-4 w-4 text-blue-500" />
          New in Portal ({members.length})
        </h4>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="outline" onClick={toggleAll}>
            {selectedIndices.size === members.length ? "Deselect All" : "Select All"}
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
            {importResult.imported_names.length > 0 && (
              <span className="font-normal"> ({importResult.imported_names.join(", ")})</span>
            )}
          </p>
          {importResult.errors.map((err, i) => (
            <p key={i} className="mt-1 text-destructive">{err}</p>
          ))}
        </div>
      )}

      <ScrollArea className="h-40">
        <div className="space-y-1">
          {members.map((m, i) => (
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
              <Badge variant={isPortalMemberActive(m) ? "secondary" : "destructive"} className="text-xs">
                {isPortalMemberActive(m) ? "Active" : "Inactive"}
              </Badge>
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
