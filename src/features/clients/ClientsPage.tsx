import { useState, useMemo, useEffect } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  createColumnHelper,
  type SortingState,
} from "@tanstack/react-table";
import { useClients } from "@/hooks/useClients";
import { Button } from "@/components/ui/button";
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

const columnHelper = createColumnHelper<ClientListItem>();

export function ClientsPage() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  // Initialize state from URL search params
  const initialSearch = searchParams.get("q") ?? "";
  const initialPerPage = searchParams.get("perPage") ?? "25";
  const initialPage = Number(searchParams.get("page") ?? "1") || 1;

  const [search, setSearch] = useState(initialSearch);
  const [debouncedSearch, setDebouncedSearch] = useState(initialSearch);
  const [page, setPage] = useState(initialPage);
  const [perPageOption, setPerPageOption] = useState(initialPerPage);

  // Sync state changes to URL (replace, not push)
  useEffect(() => {
    const params: Record<string, string> = {};
    if (debouncedSearch) params.q = debouncedSearch;
    if (perPageOption !== "25") params.perPage = perPageOption;
    if (page > 1) params.page = String(page);
    setSearchParams(params, { replace: true });
  }, [debouncedSearch, perPageOption, page, setSearchParams]);

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

  const [sorting, setSorting] = useState<SortingState>([]);
  const perPage = perPageOption === "all" ? 9999 : Number(perPageOption);
  const { data, isLoading } = useClients(filters, page, perPage);

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
    data: data?.items ?? [],
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
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
      </div>

      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className="border-b bg-muted/50">
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
