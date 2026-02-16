import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Plus, Pin, MessageSquare } from "lucide-react";
import type { ConversationListItem } from "@/types";

interface ConversationListProps {
  conversations: ConversationListItem[];
  selectedId?: string;
  onSelect: (id: string) => void;
  onNew: () => void;
}

const STATUS_BADGE: Record<string, { label: string; className: string }> = {
  OPEN: { label: "Open", className: "bg-green-100 text-green-700 hover:bg-green-100 dark:bg-green-900/30 dark:text-green-400 dark:hover:bg-green-900/30" },
  CLOSED: { label: "Closed", className: "bg-gray-100 text-gray-600 hover:bg-gray-100 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-gray-800" },
  ARCHIVED: { label: "Archived", className: "bg-yellow-100 text-yellow-700 hover:bg-yellow-100 dark:bg-yellow-900/30 dark:text-yellow-400 dark:hover:bg-yellow-900/30" },
};

function formatRelative(dateStr?: string): string {
  if (!dateStr) return "";
  const d = new Date(dateStr.replace(" ", "T"));
  if (isNaN(d.getTime())) return "";
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays}d ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

export function ConversationList({
  conversations,
  selectedId,
  onSelect,
  onNew,
}: ConversationListProps) {
  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b px-3 py-2">
        <h3 className="text-sm font-semibold">Threads</h3>
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onNew}>
          <Plus className="h-4 w-4" />
        </Button>
      </div>
      <ScrollArea className="flex-1">
        {conversations.length === 0 ? (
          <p className="p-4 text-center text-sm text-muted-foreground">
            No conversations yet
          </p>
        ) : (
          <div className="space-y-1 p-2">
            {conversations.map((conv) => {
              const statusInfo = STATUS_BADGE[conv.status] || STATUS_BADGE.OPEN;
              return (
                <button
                  key={conv.id}
                  onClick={() => onSelect(conv.id)}
                  className={`w-full rounded-md p-2 text-left transition-colors hover:bg-accent ${
                    selectedId === conv.id ? "bg-accent" : ""
                  }`}
                >
                  <div className="flex items-center gap-1">
                    {!!conv.is_pinned && (
                      <Pin className="h-3 w-3 shrink-0 text-muted-foreground" />
                    )}
                    <span className="truncate text-sm font-medium">
                      {conv.title}
                    </span>
                  </div>
                  <div className="mt-1 flex items-center gap-2">
                    <Badge
                      variant="outline"
                      className={`text-[10px] px-1.5 py-0 ${statusInfo.className}`}
                    >
                      {statusInfo.label}
                    </Badge>
                    <span className="flex items-center gap-1 text-xs text-muted-foreground">
                      <MessageSquare className="h-3 w-3" />
                      {conv.entry_count}
                    </span>
                    <span className="ml-auto text-xs text-muted-foreground">
                      {formatRelative(conv.last_entry_at || conv.created_at)}
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}
