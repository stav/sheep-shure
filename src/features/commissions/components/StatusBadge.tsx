import { Badge } from "@/components/ui/badge";
import type { CommissionStatus } from "@/types";

const statusConfig: Record<
  CommissionStatus,
  { label: string; className: string }
> = {
  OK: {
    label: "OK",
    className: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
  },
  UNDERPAID: {
    label: "Underpaid",
    className: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200",
  },
  OVERPAID: {
    label: "Overpaid",
    className: "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200",
  },
  MISSING: {
    label: "Missing",
    className: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
  },
  ZERO_RATE: {
    label: "No Rate",
    className: "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200",
  },
  UNMATCHED: {
    label: "Unmatched",
    className: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
  },
  PENDING: {
    label: "Pending",
    className: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
  },
};

export function StatusBadge({ status }: { status?: CommissionStatus | string }) {
  if (!status) return null;
  const config = statusConfig[status as CommissionStatus] ?? {
    label: status,
    className: "bg-gray-100 text-gray-800",
  };
  return (
    <Badge variant="outline" className={config.className}>
      {config.label}
    </Badge>
  );
}
