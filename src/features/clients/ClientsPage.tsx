import { useState, useMemo, useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { useAppStore } from "@/stores/appStore";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
  type RowSelectionState,
} from "@tanstack/react-table";
import { useClients } from "@/hooks/useClients";
import { tauriInvoke } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ClientListItem, ClientFilters } from "@/types";
import { Plus, Search, X, ChevronLeft, ChevronRight, Loader2, ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react";
import { toast } from "sonner";

const columnHelper = createColumnHelper<ClientListItem>();

export function ClientsPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const setPageSubtitle = useAppStore((s) => s.setPageSubtitle);

  // Read URL params directly (useSearchParams causes infinite re-renders in React Router v6)
  const urlParams = new URLSearchParams(window.location.search);
  const initialSearch = urlParams.get("q") ?? "";
  const initialPerPage = urlParams.get("perPage") ?? "25";
  const initialPage = Number(urlParams.get("page") ?? "1") || 1;

  const [search, setSearch] = useState(initialSearch);
  const [debouncedSearch, setDebouncedSearch] = useState(initialSearch);
  const [page, setPage] = useState(initialPage);
  const [perPageOption, setPerPageOption] = useState(initialPerPage);
  const [showInactive, setShowInactive] = useState(false);

  // Bulk selection
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const [bulkAction, setBulkAction] = useState<"deactivate" | "delete" | null>(null);
  const [bulkPending, setBulkPending] = useState(false);
  const lastClickedIndex = useRef<number | null>(null);

  // Clear selection on page/filter/search changes
  useEffect(() => {
    setRowSelection({});
    lastClickedIndex.current = null;
  }, [page, debouncedSearch, showInactive, perPageOption]);

  // Simple debounce
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const handleSearch = (value: string) => {
    setSearch(value);
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setDebouncedSearch(value);
      setPage(1);
    }, 300);
  };

  const filters: ClientFilters = useMemo(() => ({
    search: debouncedSearch || undefined,
    is_active: showInactive ? undefined : true,
  }), [debouncedSearch, showInactive]);

  const [sorting, setSorting] = useState<SortingState>([]);
  const perPage = perPageOption === "all" ? 9999 : Number(perPageOption);
  const { data, isLoading } = useClients(filters, page, perPage);

  // Stable empty array to avoid TanStack Table getting a new [] reference each render
  const emptyItems = useRef<ClientListItem[]>([]);
  const tableData = data?.items ?? emptyItems.current;

  // Set page subtitle in nav header
  const total = data?.total;
  useEffect(() => {
    setPageSubtitle(total != null ? `${total} total clients` : null);
    return () => setPageSubtitle(null);
  }, [total, setPageSubtitle]);

  const columns = useMemo(() => [
    columnHelper.accessor("first_name", {
      header: "First Name",
      cell: (info) => <span className="font-medium">{info.getValue()}</span>,
    }),
    columnHelper.accessor("last_name", {
      header: "Last Name",
      cell: (info) => <span className="font-medium">{info.getValue()}</span>,
    }),
    columnHelper.accessor("dob", {
      header: "Age",
      cell: (info) => {
        const dob = info.getValue();
        if (!dob) return "\u2014";
        const birth = new Date(dob);
        const today = new Date();
        let age = today.getFullYear() - birth.getFullYear();
        const m = today.getMonth() - birth.getMonth();
        if (m < 0 || (m === 0 && today.getDate() < birth.getDate())) age--;
        return age;
      },
    }),
    columnHelper.accessor("carrier_name", {
      header: "Carrier",
      cell: (info) => info.getValue() || "\u2014",
    }),
    columnHelper.accessor("plan_name", {
      header: "Plan",
      cell: (info) => info.getValue() || "\u2014",
    }),
  ], []);

  const table = useReactTable({
    data: tableData,
    columns,
    state: { sorting, rowSelection },
    onSortingChange: setSorting,
    onRowSelectionChange: setRowSelection,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    enableRowSelection: true,
    getRowId: (row) => row.id,
  });

  const totalPages = data ? Math.ceil(data.total / perPage) : 0;
  const selectedIds = Object.keys(rowSelection).filter((id) => rowSelection[id]);
  const selectedCount = selectedIds.length;

  // Shift-click range selection
  const handleRowCheckbox = useCallback(
    (rowIndex: number, rowId: string, e: React.MouseEvent) => {
      e.stopPropagation(); // prevent row navigation
      const rows = table.getRowModel().rows;

      if (e.shiftKey && lastClickedIndex.current != null) {
        const start = Math.min(lastClickedIndex.current, rowIndex);
        const end = Math.max(lastClickedIndex.current, rowIndex);
        // Determine if we're selecting or deselecting based on the clicked row's current state
        const willSelect = !rowSelection[rowId];
        setRowSelection((prev) => {
          const next = { ...prev };
          for (let i = start; i <= end; i++) {
            next[rows[i].id] = willSelect;
          }
          return next;
        });
      } else {
        setRowSelection((prev) => ({
          ...prev,
          [rowId]: !prev[rowId],
        }));
      }
      lastClickedIndex.current = rowIndex;
    },
    [table, rowSelection],
  );

  // Bulk actions
  const handleBulkDeactivate = useCallback(async () => {
    setBulkPending(true);
    let count = 0;
    for (const id of selectedIds) {
      try {
        await tauriInvoke("update_client", { id, input: { is_active: false } });
        await tauriInvoke("create_system_event", {
          clientId: id,
          eventType: "CLIENT_DEACTIVATED",
          eventData: null,
        });
        count++;
      } catch (err) {
        console.error(`Failed to deactivate ${id}:`, err);
      }
    }
    queryClient.invalidateQueries({ queryKey: ["clients"] });
    setRowSelection({});
    setBulkAction(null);
    setBulkPending(false);
    toast.success(`Deactivated ${count} client${count !== 1 ? "s" : ""}`);
  }, [selectedIds, queryClient]);

  const handleBulkDelete = useCallback(async () => {
    setBulkPending(true);
    let count = 0;
    for (const id of selectedIds) {
      try {
        await tauriInvoke("hard_delete_client", { id });
        count++;
      } catch (err) {
        console.error(`Failed to delete ${id}:`, err);
      }
    }
    queryClient.invalidateQueries({ queryKey: ["clients"] });
    setRowSelection({});
    setBulkAction(null);
    setBulkPending(false);
    toast.success(`Deleted ${count} client${count !== 1 ? "s" : ""}`);
  }, [selectedIds, queryClient]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4 flex-wrap">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search by name, MBI, phone, email..."
            value={search}
            onChange={(e) => handleSearch(e.target.value)}
            className="pl-9 pr-8"
          />
          {search && (
            <button
              onClick={() => handleSearch("")}
              className="absolute right-2 top-1/2 -translate-y-1/2 rounded-sm p-0.5 text-muted-foreground hover:text-foreground"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
        {selectedCount > 0 ? (
          <div className="flex items-center gap-3 text-sm">
            <span className="text-muted-foreground font-medium">{selectedCount} selected</span>
            {bulkAction === "deactivate" ? (
              <div className="flex items-center gap-1.5">
                <Button variant="outline" size="sm" onClick={() => setBulkAction(null)} disabled={bulkPending}>
                  Cancel
                </Button>
                <Button variant="destructive" size="sm" onClick={handleBulkDeactivate} disabled={bulkPending}>
                  {bulkPending && <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />}
                  Confirm Deactivate
                </Button>
              </div>
            ) : bulkAction === "delete" ? (
              <div className="flex items-center gap-1.5">
                <Button variant="outline" size="sm" onClick={() => setBulkAction(null)} disabled={bulkPending}>
                  Cancel
                </Button>
                <Button variant="destructive" size="sm" onClick={handleBulkDelete} disabled={bulkPending}>
                  {bulkPending && <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />}
                  Confirm Delete
                </Button>
              </div>
            ) : (
              <div className="flex items-center gap-1.5">
                <Button variant="outline" size="sm" onClick={() => setBulkAction("deactivate")}>
                  Deactivate
                </Button>
                <Button variant="outline" size="sm" className="text-red-600 hover:text-red-700" onClick={() => setBulkAction("delete")}>
                  Delete
                </Button>
              </div>
            )}
          </div>
        ) : (
          <label className="flex items-center gap-2 text-sm text-muted-foreground cursor-pointer select-none">
            <Checkbox
              id="show-inactive"
              checked={showInactive}
              onCheckedChange={(checked) => { setShowInactive(!!checked); setPage(1); }}
            />
            Show inactive
          </label>
        )}
        <Button onClick={() => navigate("/clients/new")} className="ml-auto">
          <Plus className="mr-2 h-4 w-4" />
          New Client
        </Button>
      </div>

      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="border-b bg-muted/50">
                <th className="h-10 w-10 px-3">
                  <Checkbox
                    checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && "indeterminate")}
                    onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
                  />
                </th>
                {headerGroup.headers.map((header) => (
                  <th
                    key={header.id}
                    className="h-10 px-4 text-left font-medium text-muted-foreground select-none cursor-pointer hover:text-foreground"
                    onClick={header.column.getToggleSortingHandler()}
                  >
                    <span className="inline-flex items-center gap-1">
                      {flexRender(header.column.columnDef.header, header.getContext())}
                      {{
                        asc: <ArrowUp className="h-3.5 w-3.5" />,
                        desc: <ArrowDown className="h-3.5 w-3.5" />,
                      }[header.column.getIsSorted() as string] ?? (
                        <ArrowUpDown className="h-3.5 w-3.5 opacity-30" />
                      )}
                    </span>
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody className="select-none">
            {isLoading ? (
              <tr>
                <td colSpan={columns.length + 1} className="h-32 text-center">
                  <Loader2 className="mx-auto h-6 w-6 animate-spin text-muted-foreground" />
                </td>
              </tr>
            ) : table.getRowModel().rows.length === 0 ? (
              <tr>
                <td colSpan={columns.length + 1} className="h-32 text-center text-muted-foreground">
                  No clients found.
                </td>
              </tr>
            ) : (
              table.getRowModel().rows.map((row, rowIndex) => (
                <tr
                  key={row.id}
                  className={`border-b cursor-pointer hover:bg-muted/50 transition-colors ${!row.original.is_active ? "opacity-50 border-l-2 border-l-red-400" : ""} ${row.getIsSelected() ? "bg-muted/30" : ""}`}
                  onClick={() => navigate(`/clients/${row.original.id}`)}
                >
                  <td className="w-10 px-3 py-3">
                    <Checkbox
                      checked={row.getIsSelected()}
                      onClick={(e) => handleRowCheckbox(rowIndex, row.id, e)}
                      onCheckedChange={() => {}} // controlled — actual logic in onClick for shift-click support
                    />
                  </td>
                  {row.getVisibleCells().map((cell) => (
                    <td key={cell.id} className="px-4 py-3">
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <span>Rows per page</span>
          <Select
            value={perPageOption}
            onValueChange={(val) => {
              setPerPageOption(val);
              setPage(1);
            }}
          >
            <SelectTrigger className="h-8 w-[70px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {[5, 10, 25, 50, 100].map((n) => (
                <SelectItem key={n} value={String(n)}>{n}</SelectItem>
              ))}
              <SelectItem value="all">All</SelectItem>
            </SelectContent>
          </Select>
        </div>
        {totalPages > 1 && (
          <div className="flex items-center gap-4">
            <p className="text-sm text-muted-foreground">
              Page {page} of {totalPages}
            </p>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={page <= 1}
                onClick={() => setPage((p) => p - 1)}
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={page >= totalPages}
                onClick={() => setPage((p) => p + 1)}
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
