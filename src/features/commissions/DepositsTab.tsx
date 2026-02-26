import { useState } from "react";
import { Plus, Pencil, Trash2, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useCommissionDeposits,
  useCreateCommissionDeposit,
  useUpdateCommissionDeposit,
  useDeleteCommissionDeposit,
  useCarriers,
} from "@/hooks";
import { DepositFormDialog } from "./components/DepositFormDialog";
import type {
  CommissionDepositListItem,
  CreateCommissionDepositInput,
} from "@/types";

export function DepositsTab() {
  const [filterCarrier, setFilterCarrier] = useState<string | undefined>();
  const [filterMonth, setFilterMonth] = useState<string | undefined>();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingDeposit, setEditingDeposit] = useState<
    CommissionDepositListItem | undefined
  >();
  const [duplicateDefaults, setDuplicateDefaults] = useState<
    CreateCommissionDepositInput | undefined
  >();

  const { data: carriers } = useCarriers();
  const { data: deposits, isLoading } = useCommissionDeposits(
    filterCarrier,
    filterMonth
  );
  const createDeposit = useCreateCommissionDeposit();
  const updateDeposit = useUpdateCommissionDeposit();
  const deleteDeposit = useDeleteCommissionDeposit();

  const handleAdd = () => {
    setEditingDeposit(undefined);
    setDuplicateDefaults(undefined);
    setDialogOpen(true);
  };

  const handleEdit = (deposit: CommissionDepositListItem) => {
    setEditingDeposit(deposit);
    setDuplicateDefaults(undefined);
    setDialogOpen(true);
  };

  const handleDuplicate = (deposit: CommissionDepositListItem) => {
    setEditingDeposit(undefined);
    setDuplicateDefaults({
      carrier_id: deposit.carrier_id,
      deposit_month: deposit.deposit_month,
      deposit_amount: deposit.deposit_amount,
      deposit_date: deposit.deposit_date ?? undefined,
      reference: deposit.reference ?? undefined,
      notes: deposit.notes ?? undefined,
    });
    setDialogOpen(true);
  };

  const handleDelete = (id: string) => {
    deleteDeposit.mutate(id);
  };

  const [submitError, setSubmitError] = useState<string | null>(null);

  const handleSubmit = (input: CreateCommissionDepositInput) => {
    setSubmitError(null);
    const callbacks = {
      onSuccess: () => {
        setSubmitError(null);
        setDialogOpen(false);
      },
      onError: (err: Error) => {
        setSubmitError(err.message || String(err));
      },
    };
    if (editingDeposit) {
      updateDeposit.mutate({ id: editingDeposit.id, input }, callbacks);
    } else {
      createDeposit.mutate(input, callbacks);
    }
  };

  const fmt = (v: number) => `$${v.toFixed(2)}`;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Select
            value={filterCarrier ?? "all"}
            onValueChange={(v) =>
              setFilterCarrier(v === "all" ? undefined : v)
            }
          >
            <SelectTrigger className="w-48">
              <SelectValue placeholder="All Carriers" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Carriers</SelectItem>
              {carriers?.map((c) => (
                <SelectItem key={c.id} value={c.id}>
                  {c.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Input
            type="month"
            value={filterMonth ?? ""}
            onChange={(e) => setFilterMonth(e.target.value || undefined)}
            className="w-44"
          />
        </div>

        <Button onClick={handleAdd} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          Record Deposit
        </Button>
      </div>

      <Card>
        <CardContent className="p-0">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="px-4 py-3 text-left font-medium">Carrier</th>
                  <th className="px-4 py-3 text-left font-medium">Month</th>
                  <th className="px-4 py-3 text-right font-medium">
                    Deposit Amount
                  </th>
                  <th className="px-4 py-3 text-left font-medium">Date</th>
                  <th className="px-4 py-3 text-left font-medium">Reference</th>
                  <th className="px-4 py-3 text-right font-medium">
                    Statement Total
                  </th>
                  <th className="px-4 py-3 text-right font-medium">
                    Difference
                  </th>
                  <th className="px-4 py-3 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {isLoading ? (
                  <tr>
                    <td
                      colSpan={8}
                      className="px-4 py-8 text-center text-muted-foreground"
                    >
                      Loading...
                    </td>
                  </tr>
                ) : !deposits?.length ? (
                  <tr>
                    <td
                      colSpan={8}
                      className="px-4 py-8 text-center text-muted-foreground"
                    >
                      No deposits recorded. Click "Record Deposit" to add one.
                    </td>
                  </tr>
                ) : (
                  deposits.map((d) => (
                    <tr
                      key={d.id}
                      className="border-b last:border-b-0 hover:bg-muted/25"
                    >
                      <td className="px-4 py-3">{d.carrier_name}</td>
                      <td className="px-4 py-3">{d.deposit_month}</td>
                      <td className="px-4 py-3 text-right font-mono">
                        {fmt(d.deposit_amount)}
                      </td>
                      <td className="px-4 py-3">{d.deposit_date ?? "—"}</td>
                      <td className="px-4 py-3">{d.reference ?? "—"}</td>
                      <td className="px-4 py-3 text-right font-mono">
                        {fmt(d.statement_total)}
                      </td>
                      <td className="px-4 py-3 text-right font-mono">
                        <span
                          className={
                            d.difference < -0.01
                              ? "text-red-600"
                              : d.difference > 0.01
                                ? "text-orange-600"
                                : "text-green-600"
                          }
                        >
                          {d.difference >= 0 ? "+" : ""}
                          ${d.difference.toFixed(2)}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-right">
                        <div className="flex items-center justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => handleDuplicate(d)}
                            title="Duplicate"
                          >
                            <Copy className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => handleEdit(d)}
                            title="Edit"
                          >
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8 text-destructive hover:text-destructive"
                            onClick={() => handleDelete(d.id)}
                            title="Delete"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>

      <DepositFormDialog
        open={dialogOpen}
        onOpenChange={(v) => { setDialogOpen(v); if (!v) setSubmitError(null); }}
        deposit={editingDeposit}
        defaultValues={duplicateDefaults}
        onSubmit={handleSubmit}
        isPending={createDeposit.isPending || updateDeposit.isPending}
        error={submitError}
      />
    </div>
  );
}
