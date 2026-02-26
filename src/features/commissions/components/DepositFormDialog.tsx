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
import { useCarriers } from "@/hooks";
import type {
  CommissionDepositListItem,
  CreateCommissionDepositInput,
} from "@/types";

interface DepositFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  deposit?: CommissionDepositListItem;
  onSubmit: (input: CreateCommissionDepositInput) => void;
  isPending: boolean;
}

export function DepositFormDialog({
  open,
  onOpenChange,
  deposit,
  onSubmit,
  isPending,
}: DepositFormDialogProps) {
  const { data: carriers } = useCarriers();
  const [carrierId, setCarrierId] = useState("");
  const [depositMonth, setDepositMonth] = useState("");
  const [depositAmount, setDepositAmount] = useState(0);
  const [depositDate, setDepositDate] = useState("");
  const [reference, setReference] = useState("");
  const [notes, setNotes] = useState("");

  useEffect(() => {
    if (deposit) {
      setCarrierId(deposit.carrier_id);
      setDepositMonth(deposit.deposit_month);
      setDepositAmount(deposit.deposit_amount);
      setDepositDate(deposit.deposit_date ?? "");
      setReference(deposit.reference ?? "");
      setNotes(deposit.notes ?? "");
    } else {
      setCarrierId("");
      setDepositMonth("");
      setDepositAmount(0);
      setDepositDate("");
      setReference("");
      setNotes("");
    }
  }, [deposit, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      carrier_id: carrierId,
      deposit_month: depositMonth,
      deposit_amount: depositAmount,
      deposit_date: depositDate || undefined,
      reference: reference || undefined,
      notes: notes || undefined,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{deposit ? "Edit Deposit" : "Record Deposit"}</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
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
            <Label>Month</Label>
            <Input
              type="month"
              value={depositMonth}
              onChange={(e) => setDepositMonth(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label>Deposit Amount</Label>
            <Input
              type="number"
              step="0.01"
              value={depositAmount}
              onChange={(e) => setDepositAmount(Number(e.target.value))}
            />
          </div>

          <div className="space-y-2">
            <Label>Deposit Date</Label>
            <Input
              type="date"
              value={depositDate}
              onChange={(e) => setDepositDate(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label>Reference (check#/ACH)</Label>
            <Input
              value={reference}
              onChange={(e) => setReference(e.target.value)}
              placeholder="Check # or ACH reference"
            />
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

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={!carrierId || !depositMonth || isPending}
            >
              {isPending ? "Saving..." : deposit ? "Update" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
