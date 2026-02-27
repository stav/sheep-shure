import { useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { ImportLogEntry } from "@/types";

const phaseColors: Record<string, string> = {
  portal: "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
  download: "bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300",
  parse: "bg-cyan-100 text-cyan-700 dark:bg-cyan-900 dark:text-cyan-300",
  match: "bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300",
  import: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300",
};

const levelColors: Record<string, string> = {
  success: "text-green-600 dark:text-green-400",
  warn: "text-yellow-600 dark:text-yellow-400",
  error: "text-red-600 dark:text-red-400",
  info: "",
};

function LogEntry({ entry }: { entry: ImportLogEntry }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="flex items-start gap-2 py-0.5 text-xs leading-relaxed">
      <span className="font-mono text-muted-foreground shrink-0">
        {entry.timestamp}
      </span>
      <span
        className={`inline-block rounded px-1.5 py-0.5 text-[10px] font-medium leading-none shrink-0 w-16 text-center ${phaseColors[entry.phase] ?? "bg-muted"}`}
      >
        {entry.phase}
      </span>
      <span className={`min-w-0 ${levelColors[entry.level] ?? ""}`}>
        {entry.detail ? (
          <button
            type="button"
            className="inline-flex items-center gap-0.5 hover:underline text-left"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? (
              <ChevronDown className="h-3 w-3 shrink-0" />
            ) : (
              <ChevronRight className="h-3 w-3 shrink-0" />
            )}
            {entry.message}
          </button>
        ) : (
          entry.message
        )}
        {expanded && entry.detail && (
          <pre className="mt-1 rounded bg-muted p-2 text-[10px] leading-tight overflow-x-auto whitespace-pre-wrap">
            {entry.detail}
          </pre>
        )}
      </span>
    </div>
  );
}

export function ActivityLog({ entries }: { entries: ImportLogEntry[] }) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries.length]);

  if (entries.length === 0) {
    return (
      <div className="h-40 flex items-center justify-center text-xs text-muted-foreground">
        Waiting for activity...
      </div>
    );
  }

  return (
    <ScrollArea className="h-80 rounded border bg-background p-3">
      {entries.map((entry, i) => (
        <LogEntry key={i} entry={entry} />
      ))}
      <div ref={bottomRef} />
    </ScrollArea>
  );
}
