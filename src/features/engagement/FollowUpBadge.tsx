import { Badge } from "@/components/ui/badge";
import { Clock } from "lucide-react";

interface FollowUpBadgeProps {
  date: string;
  note?: string;
}

export function FollowUpBadge({ date, note }: FollowUpBadgeProps) {
  const today = new Date().toISOString().split("T")[0];
  const isOverdue = date < today;
  const isToday = date === today;

  let className: string;
  let label: string;

  if (isOverdue) {
    className = "bg-red-100 text-red-700 hover:bg-red-100";
    label = `Overdue: ${date}`;
  } else if (isToday) {
    className = "bg-amber-100 text-amber-700 hover:bg-amber-100";
    label = "Follow-up today";
  } else {
    className = "bg-blue-100 text-blue-700 hover:bg-blue-100";
    label = `Follow-up: ${date}`;
  }

  return (
    <Badge variant="outline" className={className} title={note}>
      <Clock className="mr-1 h-3 w-3" />
      {label}
    </Badge>
  );
}
