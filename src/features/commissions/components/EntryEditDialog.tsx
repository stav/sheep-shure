import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ReconciliationRow, UpdateCommissionEntryInput } from "@/types";

const PLAN_TYPES = [
  { code: "MA", label: "MA" },
  { code: "MAPD", label: "MAPD" },
  { code: "DSNP", label: "DSNP" },
  { code: "CSNP", label: "CSNP" },
  { code: "ISNP", label: "ISNP" },
  { code: "MMP", label: "MMP" },
  { code: "PACE", label: "PACE" },
  { code: "MSA", label: "MSA" },
  { code: "PFFS", label: "PFFS" },
  { code: "COST", label: "COST" },
  { code: "PDP", label: "PDP" },
  { code: "MedSupA", label: "MedSup A" },
  { code: "MedSupB", label: "MedSup B" },
  { code: "MedSupC", label: "MedSup C" },
  { code: "MedSupD", label: "MedSup D" },
  { code: "MedSupF", label: "MedSup F" },
  { code: "MedSupG", label: "MedSup G" },
  { code: "MedSupK", label: "MedSup K" },
  { code: "MedSupL", label: "MedSup L" },
  { code: "MedSupM", label: "MedSup M" },
  { code: "MedSupN", label: "MedSup N" },
];

const STATUSES = [
  "PENDING",
  "OK",
  "UNDERPAID",
  "OVERPAID",
  "MISSING",
  "ZERO_RATE",
  "UNMATCHED",
];

interface EntryEditDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  entry?: ReconciliationRow;
  onSubmit: (id: string, input: UpdateCommissionEntryInput) => void;
  isPending: boolean;
  error?: string | null;
}

export function EntryEditDialog({
  open,
  onOpenChange,
  entry,
  onSubmit,
  isPending,
  error,
}: EntryEditDialogProps) {
  const [memberName, setMemberName] = useState("");
  const [planTypeCode, setPlanTypeCode] = useState("");
  const [statementAmount, setStatementAmount] = useState(0);
  const [paidAmount, setPaidAmount] = useState(0);
  const [isInitial, setIsInitial] = useState("unknown");
  const [status, setStatus] = useState("PENDING");
  const [notes, setNotes] = useState("");

  useEffect(() => {
    if (entry) {
      setMemberName(entry.member_name ?? "");
      setPlanTypeCode(entry.plan_type_code ?? "");
      setStatementAmount(entry.statement_amount ?? 0);
      setPaidAmount(entry.paid_amount ?? 0);
      setIsInitial(
        entry.is_initial != null
          ? entry.is_initial === 1
            ? "initial"
            : "renewal"
          : "unknown"
      );
      setStatus(entry.status ?? "PENDING");
      setNotes("");
    }
  }, [entry, open]);

  const handleClick = () => {
    if (!entry) return;
    const input: UpdateCommissionEntryInput = {
      member_name: memberName || undefined,
      plan_type_code: planTypeCode || undefined,
      statement_amount: statementAmount,
      paid_amount: paidAmount,
      is_initial: isInitial === "unknown" ? undefined : isInitial === "initial" ? 1 : 0,
      status: status || undefined,
      notes: notes || undefined,
    };
    onSubmit(entry.id, input);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Edit Commission Entry</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-2">
            <Label>Member Name</Label>
            <Input
              value={memberName}
              onChange={(e) => setMemberName(e.target.value)}
              placeholder="Member name"
            />
          </div>

          <div className="space-y-2">
            <Label>Plan Type</Label>
            <Select value={planTypeCode} onValueChange={setPlanTypeCode}>
              <SelectTrigger>
                <SelectValue placeholder="Select plan type" />
              </SelectTrigger>
              <SelectContent>
                {PLAN_TYPES.map((pt) => (
                  <SelectItem key={pt.code} value={pt.code}>
                    {pt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Statement Amount</Label>
              <Input
                type="number"
                step="0.01"
                value={statementAmount}
                onChange={(e) => setStatementAmount(Number(e.target.value))}
              />
            </div>
            <div className="space-y-2">
              <Label>Paid Amount</Label>
              <Input
                type="number"
                step="0.01"
                value={paidAmount}
                onChange={(e) => setPaidAmount(Number(e.target.value))}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Initial/Renewal</Label>
              <Select value={isInitial} onValueChange={setIsInitial}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="initial">Initial</SelectItem>
                  <SelectItem value="renewal">Renewal</SelectItem>
                  <SelectItem value="unknown">Unknown</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>Status</Label>
              <Select value={status} onValueChange={setStatus}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {STATUSES.map((s) => (
                    <SelectItem key={s} value={s}>
                      {s}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label>Notes</Label>
            <Textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Optional notes"
              rows={2}
            />
          </div>

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleClick}
              disabled={isPending}
            >
              {isPending ? "Saving..." : "Update"}
            </Button>
          </DialogFooter>
        </div>
      </DialogContent>
    </Dialog>
  );
}
