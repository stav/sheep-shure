import { useState } from "react";
import { Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Checkbox } from "@/components/ui/checkbox";
import { useConfirmDisenrollments } from "@/hooks/useCarrierSync";
import type { SyncDisenrollment, ConfirmDisenrollmentResult } from "@/types";

export function DisenrollmentSection({
  disenrolled,
  onDisenrolled,
}: {
  disenrolled: SyncDisenrollment[];
  onDisenrolled: (confirmedIds: string[]) => void;
}) {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [result, setResult] = useState<ConfirmDisenrollmentResult | null>(null);
  const confirmDisenrollments = useConfirmDisenrollments();

  if (disenrolled.length === 0) {
    return (
      <p className="py-4 text-center text-sm text-muted-foreground">
        No disenrollment candidates.
      </p>
    );
  }

  const toggleAll = () => {
    if (selectedIds.size === disenrolled.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(disenrolled.map((d) => d.enrollment_id)));
    }
  };

  const toggleOne = (enrollmentId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(enrollmentId)) next.delete(enrollmentId);
      else next.add(enrollmentId);
      return next;
    });
  };

  const handleConfirm = () => {
    const ids = Array.from(selectedIds);
    setResult(null);
    confirmDisenrollments.mutate(ids, {
      onSuccess: (res) => {
        setResult(res);
        setSelectedIds(new Set());
        onDisenrolled(ids);
      },
      onError: (err) => {
        setResult({ disenrolled: 0, errors: [String(err)] });
      },
    });
  };

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <h4 className="text-sm font-medium text-red-600">
          Disenrollment Candidates ({disenrolled.length})
        </h4>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="outline" onClick={toggleAll}>
            {selectedIds.size === disenrolled.length ? "Deselect All" : "Select All"}
          </Button>
          <Button
            size="sm"
            variant="destructive"
            disabled={selectedIds.size === 0 || confirmDisenrollments.isPending}
            onClick={handleConfirm}
          >
            {confirmDisenrollments.isPending ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : null}
            Confirm Disenrollments ({selectedIds.size})
          </Button>
        </div>
      </div>

      {result && (
        <div
          className={`mb-2 rounded-md border p-3 text-sm ${
            result.errors.length > 0
              ? "border-yellow-300 bg-yellow-50 dark:border-yellow-800 dark:bg-yellow-950"
              : "border-green-300 bg-green-50 dark:border-green-800 dark:bg-green-950"
          }`}
        >
          <p className="font-medium">
            Disenrolled {result.disenrolled} member{result.disenrolled !== 1 ? "s" : ""} successfully.
          </p>
          {result.errors.map((err, i) => (
            <p key={i} className="mt-1 text-destructive">{err}</p>
          ))}
        </div>
      )}

      <ScrollArea className="h-48">
        <div className="space-y-1">
          {disenrolled.map((d) => (
            <div
              key={d.enrollment_id}
              className="flex items-center gap-3 rounded-md border border-red-200 bg-red-50 p-2 text-sm dark:border-red-900 dark:bg-red-950"
            >
              <Checkbox
                checked={selectedIds.has(d.enrollment_id)}
                onCheckedChange={() => toggleOne(d.enrollment_id)}
              />
              <span className="min-w-[140px] font-medium">{d.client_name}</span>
              <span className="flex-1 text-muted-foreground">
                {d.plan_name ?? "—"}
              </span>
              <Badge variant="destructive" className="text-xs">
                Not in Portal
              </Badge>
            </div>
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
