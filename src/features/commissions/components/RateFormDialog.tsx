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
import type { CommissionRateListItem, CreateCommissionRateInput } from "@/types";

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

interface RateFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  rate?: CommissionRateListItem;
  onSubmit: (input: CreateCommissionRateInput) => void;
  isPending: boolean;
}

export function RateFormDialog({
  open,
  onOpenChange,
  rate,
  onSubmit,
  isPending,
}: RateFormDialogProps) {
  const { data: carriers } = useCarriers();
  const [carrierId, setCarrierId] = useState("");
  const [planTypeCode, setPlanTypeCode] = useState("");
  const [planYear, setPlanYear] = useState(new Date().getFullYear());
  const [initialRate, setInitialRate] = useState(0);
  const [renewalRate, setRenewalRate] = useState(0);
  const [notes, setNotes] = useState("");

  useEffect(() => {
    if (rate) {
      setCarrierId(rate.carrier_id);
      setPlanTypeCode(rate.plan_type_code);
      setPlanYear(rate.plan_year);
      setInitialRate(rate.initial_rate);
      setRenewalRate(rate.renewal_rate);
      setNotes(rate.notes ?? "");
    } else {
      setCarrierId("");
      setPlanTypeCode("");
      setPlanYear(new Date().getFullYear());
      setInitialRate(0);
      setRenewalRate(0);
      setNotes("");
    }
  }, [rate, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      carrier_id: carrierId,
      plan_type_code: planTypeCode,
      plan_year: planYear,
      initial_rate: initialRate,
      renewal_rate: renewalRate,
      notes: notes || undefined,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{rate ? "Edit Rate" : "Add Rate"}</DialogTitle>
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

          <div className="space-y-2">
            <Label>Plan Year</Label>
            <Input
              type="number"
              value={planYear}
              onChange={(e) => setPlanYear(Number(e.target.value))}
              min={2020}
              max={2035}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Initial Rate ($/mo)</Label>
              <Input
                type="number"
                step="0.01"
                value={initialRate}
                onChange={(e) => setInitialRate(Number(e.target.value))}
              />
            </div>
            <div className="space-y-2">
              <Label>Renewal Rate ($/mo)</Label>
              <Input
                type="number"
                step="0.01"
                value={renewalRate}
                onChange={(e) => setRenewalRate(Number(e.target.value))}
              />
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
              disabled={!carrierId || !planTypeCode || isPending}
            >
              {isPending ? "Saving..." : rate ? "Update" : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
