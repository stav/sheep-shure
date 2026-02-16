import { useState, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import {
  useReactTable,
  getCoreRowModel,
  flexRender,
  createColumnHelper,
} from "@tanstack/react-table";
import { useClients } from "@/hooks/useClients";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { ClientListItem, ClientFilters } from "@/types";
import { Plus, Search, ChevronLeft, ChevronRight, Loader2 } from "lucide-react";

const columnHelper = createColumnHelper<ClientListItem>();

export function ClientsPage() {
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [page, setPage] = useState(1);
  const perPage = 25;

  // Simple debounce
  const [timer, setTimer] = useState<ReturnType<typeof setTimeout> | null>(null);
  const handleSearch = (value: string) => {
    setSearch(value);
    if (timer) clearTimeout(timer);
    const t = setTimeout(() => {
      setDebouncedSearch(value);
      setPage(1);
    }, 300);
    setTimer(t);
  };

  const filters: ClientFilters = useMemo(() => ({
    search: debouncedSearch || undefined,
    is_active: true,
  }), [debouncedSearch]);

  const { data, isLoading } = useClients(filters, page, perPage);

  const columns = useMemo(() => [
    columnHelper.accessor((row) => `${row.last_name}, ${row.first_name}`, {
      id: "name",
      header: "Name",
      cell: (info) => <span className="font-medium">{info.getValue()}</span>,
    }),
    columnHelper.accessor("dob", {
      header: "DOB",
      cell: (info) => info.getValue() || "\u2014",
    }),
    columnHelper.accessor("phone", {
      header: "Phone",
      cell: (info) => info.getValue() || "\u2014",
    }),
    columnHelper.accessor("email", {
      header: "Email",
      cell: (info) => info.getValue() || "\u2014",
    }),
    columnHelper.accessor((row) => [row.city, row.state, row.zip].filter(Boolean).join(", "), {
      id: "location",
      header: "Location",
      cell: (info) => info.getValue() || "\u2014",
    }),
    columnHelper.accessor("mbi", {
      header: "MBI",
      cell: (info) => (
        <span className="font-mono text-xs">{info.getValue() || "\u2014"}</span>
      ),
    }),
    columnHelper.accessor("is_dual_eligible", {
      header: "Dual",
      cell: (info) =>
        info.getValue() ? (
          <span className="inline-flex items-center rounded-full bg-purple-100 px-2 py-0.5 text-xs font-medium text-purple-700 dark:bg-purple-900/30 dark:text-purple-400">
            Dual
          </span>
        ) : null,
    }),
  ], []);

  const table = useReactTable({
    data: data?.items ?? [],
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const totalPages = data ? Math.ceil(data.total / perPage) : 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Clients</h1>
          <p className="text-sm text-muted-foreground">
            {data ? `${data.total} total clients` : "Loading..."}
          </p>
        </div>
        <Button onClick={() => navigate("/clients/new")}>
          <Plus className="mr-2 h-4 w-4" />
          New Client
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search by name, MBI, phone, email..."
            value={search}
            onChange={(e) => handleSearch(e.target.value)}
            className="pl-9"
          />
        </div>
      </div>

      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="border-b bg-muted/50">
                {headerGroup.headers.map((header) => (
                  <th
                    key={header.id}
                    className="h-10 px-4 text-left font-medium text-muted-foreground"
                  >
                    {flexRender(header.column.columnDef.header, header.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {isLoading ? (
              <tr>
                <td colSpan={columns.length} className="h-32 text-center">
                  <Loader2 className="mx-auto h-6 w-6 animate-spin text-muted-foreground" />
                </td>
              </tr>
            ) : table.getRowModel().rows.length === 0 ? (
              <tr>
                <td colSpan={columns.length} className="h-32 text-center text-muted-foreground">
                  No clients found.
                </td>
              </tr>
            ) : (
              table.getRowModel().rows.map((row) => (
                <tr
                  key={row.id}
                  className="border-b cursor-pointer hover:bg-muted/50 transition-colors"
                  onClick={() => navigate(`/clients/${row.original.id}`)}
                >
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

      {totalPages > 1 && (
        <div className="flex items-center justify-between">
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
  );
}
