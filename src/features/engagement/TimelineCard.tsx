import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Phone,
  PhoneIncoming,
  PhoneOutgoing,
  Mail,
  Calendar,
  MessageSquare,
  StickyNote,
  Cog,
  Pencil,
  Trash2,
} from "lucide-react";
import { FollowUpBadge } from "./FollowUpBadge";
import type { ConversationEntry, TimelineEntry } from "@/types";

type Entry = ConversationEntry | TimelineEntry;

interface TimelineCardProps {
  entry: Entry;
  showConversationTitle?: boolean;
  onDelete?: () => void;
  onEdit?: () => void;
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatDate(dateStr?: string): string {
  if (!dateStr) return "";
  const d = new Date(dateStr.replace(" ", "T"));
  if (isNaN(d.getTime())) return "";
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

const OUTCOME_LABELS: Record<string, string> = {
  ANSWERED: "Answered",
  NO_ANSWER: "No Answer",
  VOICEMAIL: "Voicemail",
  BUSY: "Busy",
  CALLBACK_REQUESTED: "Callback Requested",
  WRONG_NUMBER: "Wrong Number",
};

const MEETING_TYPE_LABELS: Record<string, string> = {
  IN_PERSON: "In Person",
  VIDEO: "Video",
  PHONE: "Phone",
};

function EntryIcon({ entry }: { entry: Entry }) {
  switch (entry.entry_type) {
    case "CALL":
      if (entry.call_direction === "INBOUND")
        return <PhoneIncoming className="h-4 w-4 text-green-600" />;
      if (entry.call_direction === "OUTBOUND")
        return <PhoneOutgoing className="h-4 w-4 text-blue-600" />;
      return <Phone className="h-4 w-4 text-blue-600" />;
    case "EMAIL":
      return <Mail className="h-4 w-4 text-purple-600" />;
    case "MEETING":
      return <Calendar className="h-4 w-4 text-orange-600" />;
    case "SMS":
      return <MessageSquare className="h-4 w-4 text-teal-600" />;
    case "NOTE":
      return <StickyNote className="h-4 w-4 text-yellow-600" />;
    case "SYSTEM":
      return <Cog className="h-4 w-4 text-muted-foreground" />;
  }
}

function CallDetails({ entry }: { entry: Entry }) {
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs">
      {entry.call_direction && (
        <Badge variant="outline" className="text-xs">
          {entry.call_direction === "INBOUND" ? "Inbound" : "Outbound"}
        </Badge>
      )}
      {entry.call_outcome && (
        <Badge variant="secondary" className="text-xs">
          {OUTCOME_LABELS[entry.call_outcome] || entry.call_outcome}
        </Badge>
      )}
      {entry.call_duration != null && entry.call_duration > 0 && (
        <span className="text-muted-foreground">
          {formatDuration(entry.call_duration)}
        </span>
      )}
      {entry.call_phone_number && (
        <span className="text-muted-foreground">
          {entry.call_phone_number}
        </span>
      )}
    </div>
  );
}

function EmailDetails({ entry }: { entry: Entry }) {
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
      {entry.email_from && <span>From: {entry.email_from}</span>}
      {entry.email_to && <span>To: {entry.email_to}</span>}
    </div>
  );
}

function MeetingDetails({ entry }: { entry: Entry }) {
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs">
      {entry.meeting_type && (
        <Badge variant="outline" className="text-xs">
          {MEETING_TYPE_LABELS[entry.meeting_type] || entry.meeting_type}
        </Badge>
      )}
      {entry.meeting_location && (
        <span className="text-muted-foreground">
          {entry.meeting_location}
        </span>
      )}
    </div>
  );
}

function SystemDetails({ entry }: { entry: Entry }) {
  const eventType = entry.system_event_type?.replace(/_/g, " ").toLowerCase();
  return (
    <span className="text-xs text-muted-foreground italic">
      {eventType || "System event"}
    </span>
  );
}

export function TimelineCard({
  entry,
  showConversationTitle,
  onDelete,
  onEdit,
}: TimelineCardProps) {
  const isSystem = entry.entry_type === "SYSTEM";
  const convTitle =
    showConversationTitle && "conversation_title" in entry
      ? (entry as TimelineEntry).conversation_title
      : null;

  return (
    <Card className={isSystem ? "border-dashed opacity-75" : ""}>
      <CardContent className="p-4">
        <div className="flex items-start gap-3">
          <div className="mt-0.5 shrink-0">
            <EntryIcon entry={entry} />
          </div>
          <div className="min-w-0 flex-1 space-y-1">
            {/* Header row */}
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <span className="text-xs font-medium uppercase text-muted-foreground">
                  {entry.entry_type}
                </span>
                {convTitle && convTitle !== "System Activity" && (
                  <span className="text-xs text-muted-foreground">
                    in {convTitle}
                  </span>
                )}
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <span className="text-xs text-muted-foreground">
                  {formatDate(entry.occurred_at)}
                </span>
                {(onEdit || onDelete) && !isSystem && (
                  <div className="flex items-center gap-0.5 ml-1">
                    {onEdit && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-muted-foreground hover:text-foreground"
                        onClick={onEdit}
                      >
                        <Pencil className="h-3 w-3" />
                      </Button>
                    )}
                    {onDelete && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 text-muted-foreground hover:text-destructive"
                        onClick={onDelete}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    )}
                  </div>
                )}
              </div>
            </div>

            {/* Subject */}
            {entry.subject && (
              <p className="text-sm font-medium">{entry.subject}</p>
            )}

            {/* Type-specific details */}
            {entry.entry_type === "CALL" && <CallDetails entry={entry} />}
            {entry.entry_type === "EMAIL" && <EmailDetails entry={entry} />}
            {entry.entry_type === "MEETING" && (
              <MeetingDetails entry={entry} />
            )}
            {entry.entry_type === "SYSTEM" && <SystemDetails entry={entry} />}

            {/* Body */}
            {entry.body && (
              <p className="whitespace-pre-wrap text-sm text-muted-foreground">
                {entry.body}
              </p>
            )}

            {/* Follow-up badge */}
            {entry.follow_up_date && (
              <div className="pt-1">
                <FollowUpBadge
                  date={entry.follow_up_date}
                  note={entry.follow_up_note ?? undefined}
                />
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
