import { useState } from "react";
import { Plus, Copy, Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useCommissionRates,
  useCreateCommissionRate,
  useUpdateCommissionRate,
  useDeleteCommissionRate,
  useCarriers,
} from "@/hooks";
import { RateFormDialog } from "./components/RateFormDialog";
import type { CommissionRateListItem, CreateCommissionRateInput } from "@/types";


export function RatesTab() {
  const [filterCarrier, setFilterCarrier] = useState<string | undefined>();
  const [filterYear, setFilterYear] = useState<number | undefined>();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRate, setEditingRate] = useState<CommissionRateListItem | undefined>();
  const [duplicateDefaults, setDuplicateDefaults] = useState<CreateCommissionRateInput | undefined>();

  const { data: carriers } = useCarriers();
  const { data: rates, isLoading } = useCommissionRates(filterCarrier, filterYear);
  const createRate = useCreateCommissionRate();
  const updateRate = useUpdateCommissionRate();
  const deleteRate = useDeleteCommissionRate();

  const handleAdd = () => {
    setEditingRate(undefined);
    setDuplicateDefaults(undefined);
    setDialogOpen(true);
  };

  const handleEdit = (rate: CommissionRateListItem) => {
    setEditingRate(rate);
    setDuplicateDefaults(undefined);
    setDialogOpen(true);
  };

  const handleDuplicate = (rate: CommissionRateListItem) => {
    setEditingRate(undefined);
    setDuplicateDefaults({
      carrier_id: rate.carrier_id,
      plan_type_code: rate.plan_type_code,
      plan_year: rate.plan_year,
      initial_rate: rate.initial_rate,
      renewal_rate: rate.renewal_rate,
      notes: rate.notes ?? undefined,
    });
    setDialogOpen(true);
  };

  const handleDelete = (id: string) => {
    deleteRate.mutate(id);
  };

  const handleSubmit = (input: CreateCommissionRateInput) => {
    if (editingRate) {
      updateRate.mutate(
        { id: editingRate.id, input },
        { onSuccess: () => setDialogOpen(false) }
      );
    } else {
      createRate.mutate(input, { onSuccess: () => setDialogOpen(false) });
    }
  };

  const currentYear = new Date().getFullYear();
  const years = Array.from({ length: 6 }, (_, i) => currentYear - 2 + i);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Select
            value={filterCarrier ?? "all"}
            onValueChange={(v) => setFilterCarrier(v === "all" ? undefined : v)}
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

          <Select
            value={filterYear?.toString() ?? "all"}
            onValueChange={(v) =>
              setFilterYear(v === "all" ? undefined : Number(v))
            }
          >
            <SelectTrigger className="w-32">
              <SelectValue placeholder="All Years" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Years</SelectItem>
              {years.map((y) => (
                <SelectItem key={y} value={y.toString()}>
                  {y}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <Button onClick={handleAdd} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          Add Rate
        </Button>
      </div>

      <Card>
        <CardContent className="p-0">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b bg-muted/50">
                <th className="px-4 py-3 text-left font-medium">Carrier</th>
                <th className="px-4 py-3 text-left font-medium">Plan Type</th>
                <th className="px-4 py-3 text-left font-medium">Year</th>
                <th className="px-4 py-3 text-right font-medium">Initial Rate</th>
                <th className="px-4 py-3 text-right font-medium">Renewal Rate</th>
                <th className="px-4 py-3 text-left font-medium">Notes</th>
                <th className="px-4 py-3 text-right font-medium">Actions</th>
              </tr>
            </thead>
            <tbody>
              {isLoading ? (
                <tr>
                  <td colSpan={7} className="px-4 py-8 text-center text-muted-foreground">
                    Loading rates...
                  </td>
                </tr>
              ) : !rates?.length ? (
                <tr>
                  <td colSpan={7} className="px-4 py-8 text-center text-muted-foreground">
                    No commission rates configured. Click "Add Rate" to get started.
                  </td>
                </tr>
              ) : (
                rates.map((rate) => (
                  <tr key={rate.id} className="border-b last:border-b-0 hover:bg-muted/25">
                    <td className="px-4 py-3">{rate.carrier_name}</td>
                    <td className="px-4 py-3">{rate.plan_type_code}</td>
                    <td className="px-4 py-3">{rate.plan_year}</td>
                    <td className="px-4 py-3 text-right font-mono">
                      ${rate.initial_rate.toFixed(2)}
                    </td>
                    <td className="px-4 py-3 text-right font-mono">
                      ${rate.renewal_rate.toFixed(2)}
                    </td>
                    <td className="px-4 py-3 text-muted-foreground truncate max-w-48">
                      {rate.notes}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => handleDuplicate(rate)}
                        >
                          <Copy className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => handleEdit(rate)}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 text-destructive hover:text-destructive"
                          onClick={() => handleDelete(rate.id)}
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
        </CardContent>
      </Card>

      <RateFormDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        rate={editingRate}
        defaultValues={duplicateDefaults}
        onSubmit={handleSubmit}
        isPending={createRate.isPending || updateRate.isPending}
      />
    </div>
  );
}
