import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { tauriInvoke } from "@/lib/tauri";
import { toast } from "sonner";
import {
  Users,
  FileDown,
  FileSpreadsheet,
  Loader2,
  BarChart3,
  Heart,
  MapPin,
  CalendarPlus,
  AlertTriangle,
} from "lucide-react";
import type { ClientFilters } from "@/types";

interface ReportDef {
  name: string;
  description: string;
  icon: React.ElementType;
  filters: ClientFilters;
  columns: string[];
}

const PRESET_REPORTS: ReportDef[] = [
  {
    name: "All Active Clients",
    description: "Complete list of all active clients",
    icon: Users,
    filters: { is_active: true },
    columns: [
      "first_name",
      "last_name",
      "dob",
      "phone",
      "email",
      "city",
      "state",
      "zip",
      "mbi",
    ],
  },
  {
    name: "D-SNP Eligible",
    description: "Dual-eligible special needs plan candidates",
    icon: Heart,
    filters: { is_active: true, is_dual_eligible: true },
    columns: [
      "first_name",
      "last_name",
      "phone",
      "mbi",
      "city",
      "state",
      "zip",
    ],
  },
  {
    name: "By State",
    description: "Clients grouped by state",
    icon: MapPin,
    filters: { is_active: true },
    columns: [
      "first_name",
      "last_name",
      "state",
      "city",
      "zip",
      "phone",
      "mbi",
    ],
  },
  {
    name: "New Clients (30 Days)",
    description: "Clients added in the last 30 days",
    icon: CalendarPlus,
    filters: { is_active: true },
    columns: [
      "first_name",
      "last_name",
      "phone",
      "email",
      "city",
      "state",
      "created_at",
    ],
  },
  {
    name: "Missing Information",
    description: "Clients with incomplete data (no phone, email, or MBI)",
    icon: AlertTriangle,
    filters: { is_active: true },
    columns: [
      "first_name",
      "last_name",
      "phone",
      "email",
      "mbi",
      "city",
      "state",
    ],
  },
];

interface ReportData {
  columns: string[];
  data: Record<string, string>[];
  total: number;
  report_name: string;
}

export function ReportsPage() {
  const [selectedReport, setSelectedReport] = useState<ReportDef | null>(null);

  const { data: reportData, isLoading } = useQuery({
    queryKey: ["report", selectedReport?.name],
    queryFn: () => {
      if (!selectedReport) return null;
      return tauriInvoke<ReportData>("get_report", {
        definition: {
          name: selectedReport.name,
          filters: selectedReport.filters,
          columns: selectedReport.columns,
          sort_by: "last_name",
          sort_dir: "ASC",
          group_by: null,
        },
      });
    },
    enabled: !!selectedReport,
  });

  const handleExportPdf = async () => {
    if (!selectedReport) return;
    try {
      const path = await tauriInvoke<string>("export_report_pdf", {
        definition: {
          name: selectedReport.name,
          filters: selectedReport.filters,
          columns: selectedReport.columns,
          sort_by: "last_name",
          sort_dir: "ASC",
          group_by: null,
        },
      });
      toast.success(`PDF saved to ${path}`);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to export PDF");
    }
  };

  const handleExportExcel = async () => {
    if (!reportData?.data) return;
    try {
      const XLSX = await import("xlsx");
      const ws = XLSX.utils.json_to_sheet(reportData.data);
      const wb = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(wb, ws, "Report");

      // Auto-width columns
      const colWidths = reportData.columns.map((col) => ({
        wch:
          Math.max(
            col.length,
            ...reportData.data.map((row) => (row[col] || "").length)
          ) + 2,
      }));
      ws["!cols"] = colWidths;

      const { save } = await import("@tauri-apps/plugin-dialog");
      const filePath = await save({
        filters: [{ name: "Excel", extensions: ["xlsx"] }],
        defaultPath: `${selectedReport?.name?.replace(/\s+/g, "_")}.xlsx`,
      });

      if (filePath) {
        const buffer = XLSX.write(wb, { type: "buffer", bookType: "xlsx" });
        const { writeFile } = await import("@tauri-apps/plugin-fs");
        await writeFile(filePath, new Uint8Array(buffer));
        toast.success(`Excel saved to ${filePath}`);
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to export Excel");
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Reports</h1>
        <p className="text-sm text-muted-foreground">
          Generate and export reports from your Book of Business
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Report Presets Sidebar */}
        <div className="space-y-2">
          <h2 className="text-sm font-medium text-muted-foreground mb-3">
            Preset Reports
          </h2>
          {PRESET_REPORTS.map((report) => {
            const Icon = report.icon;
            const isSelected = selectedReport?.name === report.name;
            return (
              <button
                key={report.name}
                onClick={() => setSelectedReport(report)}
                className={`w-full text-left p-3 rounded-lg border transition-colors ${
                  isSelected
                    ? "border-primary bg-primary/5"
                    : "border-transparent hover:bg-muted"
                }`}
              >
                <div className="flex items-center gap-2">
                  <Icon
                    className={`h-4 w-4 ${isSelected ? "text-primary" : "text-muted-foreground"}`}
                  />
                  <span className="text-sm font-medium">{report.name}</span>
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {report.description}
                </p>
              </button>
            );
          })}
        </div>

        {/* Report Content */}
        <div className="lg:col-span-3">
          {!selectedReport ? (
            <Card>
              <CardContent className="py-12 text-center">
                <BarChart3 className="mx-auto h-12 w-12 text-muted-foreground mb-4" />
                <p className="text-muted-foreground">
                  Select a report from the sidebar to get started
                </p>
              </CardContent>
            </Card>
          ) : (
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle>{selectedReport.name}</CardTitle>
                    <CardDescription>
                      {reportData ? `${reportData.total} records` : "Loading..."}
                    </CardDescription>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleExportPdf}
                    >
                      <FileDown className="mr-2 h-4 w-4" />
                      PDF
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleExportExcel}
                      disabled={!reportData?.data?.length}
                    >
                      <FileSpreadsheet className="mr-2 h-4 w-4" />
                      Excel
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                {isLoading ? (
                  <div className="flex items-center justify-center py-12">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                ) : reportData && reportData.data.length > 0 ? (
                  <div className="rounded-md border overflow-x-auto">
                    <table className="w-full text-sm">
                      <thead>
                        <tr className="border-b bg-muted/50">
                          {reportData.columns.map((col) => (
                            <th
                              key={col}
                              className="h-10 px-4 text-left font-medium text-muted-foreground whitespace-nowrap"
                            >
                              {col.replace(/_/g, " ")}
                            </th>
                          ))}
                        </tr>
                      </thead>
                      <tbody>
                        {reportData.data.slice(0, 100).map((row, i) => (
                          <tr key={i} className="border-b">
                            {reportData.columns.map((col) => (
                              <td
                                key={col}
                                className="px-4 py-2 whitespace-nowrap"
                              >
                                {row[col] || "\u2014"}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                    {reportData.data.length > 100 && (
                      <div className="px-4 py-2 text-xs text-muted-foreground bg-muted/30">
                        Showing first 100 of {reportData.total} records. Export
                        to see all.
                      </div>
                    )}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground text-center py-12">
                    No data matches the report criteria
                  </p>
                )}
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}
