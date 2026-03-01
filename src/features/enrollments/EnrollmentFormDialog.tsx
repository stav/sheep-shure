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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useCarriers } from "@/hooks";
import type { Enrollment } from "@/types";

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
  { code: "FIDE", label: "FIDE" },
  { code: "Supplement", label: "Supplement" },
];

const STATUSES = [
  { code: "ACTIVE", label: "Active" },
  { code: "PENDING", label: "Pending" },
  { code: "CANCELLED", label: "Cancelled" },
  { code: "DISENROLLED_VOLUNTARY", label: "Disenrolled (Voluntary)" },
  { code: "DISENROLLED_INVOLUNTARY", label: "Disenrolled (Involuntary)" },
  { code: "REINSTATED", label: "Reinstated" },
  { code: "REJECTED", label: "Rejected" },
];

interface EnrollmentFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  enrollment?: Enrollment;
  clientId: string;
  onSubmit: (input: Partial<Enrollment>) => void;
  isPending: boolean;
}

export function EnrollmentFormDialog({
  open,
  onOpenChange,
  enrollment,
  clientId,
  onSubmit,
  isPending,
}: EnrollmentFormDialogProps) {
  const { data: carriers } = useCarriers();
  const [carrierId, setCarrierId] = useState("");
  const [planName, setPlanName] = useState("");
  const [planTypeCode, setPlanTypeCode] = useState("");
  const [statusCode, setStatusCode] = useState("ACTIVE");
  const [effectiveDate, setEffectiveDate] = useState("");
  const [terminationDate, setTerminationDate] = useState("");
  const [applicationDate, setApplicationDate] = useState("");
  const [premium, setPremium] = useState("");
  const [confirmationNumber, setConfirmationNumber] = useState("");

  useEffect(() => {
    if (enrollment) {
      setCarrierId(enrollment.carrier_id ?? "");
      setPlanName(enrollment.plan_name ?? "");
      setPlanTypeCode(enrollment.plan_type_code ?? "");
      setStatusCode(enrollment.status_code ?? "ACTIVE");
      setEffectiveDate(enrollment.effective_date ?? "");
      setTerminationDate(enrollment.termination_date ?? "");
      setApplicationDate(enrollment.application_date ?? "");
      setPremium(enrollment.premium != null ? String(enrollment.premium) : "");
      setConfirmationNumber(enrollment.confirmation_number ?? "");
    } else {
      setCarrierId("");
      setPlanName("");
      setPlanTypeCode("");
      setStatusCode("ACTIVE");
      setEffectiveDate("");
      setTerminationDate("");
      setApplicationDate("");
      setPremium("");
      setConfirmationNumber("");
    }
  }, [enrollment, open]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit({
      client_id: clientId,
      carrier_id: carrierId || undefined,
      plan_name: planName || undefined,
      plan_type_code: planTypeCode || undefined,
      status_code: statusCode,
      effective_date: effectiveDate || undefined,
      termination_date: terminationDate || undefined,
      application_date: applicationDate || undefined,
      premium: premium ? Number(premium) : undefined,
      confirmation_number: confirmationNumber || undefined,
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {enrollment ? "Edit Enrollment" : "Add Enrollment"}
          </DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
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
              <Label>Plan Name</Label>
              <Input
                value={planName}
                onChange={(e) => setPlanName(e.target.value)}
                placeholder="Plan name"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Plan Type</Label>
              <Select value={planTypeCode} onValueChange={setPlanTypeCode}>
                <SelectTrigger>
                  <SelectValue placeholder="Select type" />
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
              <Label>Status</Label>
              <Select value={statusCode} onValueChange={setStatusCode}>
                <SelectTrigger>
                  <SelectValue placeholder="Select status" />
                </SelectTrigger>
                <SelectContent>
                  {STATUSES.map((s) => (
                    <SelectItem key={s.code} value={s.code}>
                      {s.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label>Effective Date</Label>
              <Input
                type="date"
                value={effectiveDate}
                onChange={(e) => setEffectiveDate(e.target.value)}
              />
            </div>

            <div className="space-y-2">
              <Label>Termination Date</Label>
              <Input
                type="date"
                value={terminationDate}
                onChange={(e) => setTerminationDate(e.target.value)}
              />
            </div>

            <div className="space-y-2">
              <Label>Application Date</Label>
              <Input
                type="date"
                value={applicationDate}
                onChange={(e) => setApplicationDate(e.target.value)}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>Premium ($/mo)</Label>
              <Input
                type="number"
                step="0.01"
                value={premium}
                onChange={(e) => setPremium(e.target.value)}
                placeholder="0.00"
              />
            </div>

            <div className="space-y-2">
              <Label>Confirmation #</Label>
              <Input
                value={confirmationNumber}
                onChange={(e) => setConfirmationNumber(e.target.value)}
                placeholder="Confirmation number"
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={!statusCode || isPending}>
              {isPending
                ? "Saving..."
                : enrollment
                  ? "Update"
                  : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
