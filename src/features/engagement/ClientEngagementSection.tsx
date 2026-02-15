import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useConversations, useClientTimeline } from "@/hooks/useConversations";
import { ConversationList } from "./ConversationList";
import { ConversationDetail } from "./ConversationDetail";
import { NewConversationDialog } from "./NewConversationDialog";
import { TimelineCard } from "./TimelineCard";
import type { EntryType } from "@/types";

interface ClientEngagementSectionProps {
  clientId: string;
}

const ENTRY_TYPE_OPTIONS: { value: string; label: string }[] = [
  { value: "ALL", label: "All Types" },
  { value: "CALL", label: "Calls" },
  { value: "EMAIL", label: "Emails" },
  { value: "MEETING", label: "Meetings" },
  { value: "SMS", label: "SMS" },
  { value: "NOTE", label: "Notes" },
  { value: "SYSTEM", label: "System" },
];

export function ClientEngagementSection({
  clientId,
}: ClientEngagementSectionProps) {
  const [activeTab, setActiveTab] = useState("threads");
  const [selectedConvId, setSelectedConvId] = useState<string>();
  const [newConvOpen, setNewConvOpen] = useState(false);
  const [timelineFilter, setTimelineFilter] = useState<string>("ALL");

  const { data: conversations } = useConversations(clientId);
  const { data: timeline } = useClientTimeline(
    clientId,
    activeTab === "timeline"
      ? timelineFilter === "ALL"
        ? null
        : (timelineFilter as EntryType)
      : null
  );

  const totalEntries =
    conversations?.reduce((sum, c) => sum + c.entry_count, 0) ?? 0;

  return (
    <div>
      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <h2 className="text-lg font-semibold">Engagement</h2>
            {totalEntries > 0 && (
              <Badge variant="secondary" className="text-xs">
                {totalEntries} {totalEntries === 1 ? "entry" : "entries"}
              </Badge>
            )}
          </div>
          <TabsList>
            <TabsTrigger value="threads">Threads</TabsTrigger>
            <TabsTrigger value="timeline">Timeline</TabsTrigger>
          </TabsList>
        </div>

        {/* Threads View */}
        <TabsContent value="threads" className="mt-0">
          <div className="flex min-h-[400px] rounded-lg border">
            {/* Left panel: conversation list */}
            <div className="w-64 shrink-0 border-r">
              <ConversationList
                conversations={conversations ?? []}
                selectedId={selectedConvId}
                onSelect={setSelectedConvId}
                onNew={() => setNewConvOpen(true)}
              />
            </div>

            {/* Right panel: conversation detail */}
            <div className="flex-1">
              {selectedConvId ? (
                <ConversationDetail
                  conversationId={selectedConvId}
                  clientId={clientId}
                  onDeleted={() => setSelectedConvId(undefined)}
                />
              ) : (
                <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                  {conversations && conversations.length > 0
                    ? "Select a conversation"
                    : "Create a conversation to get started"}
                </div>
              )}
            </div>
          </div>
        </TabsContent>

        {/* Timeline View */}
        <TabsContent value="timeline" className="mt-0">
          <div className="mb-3">
            <Select value={timelineFilter} onValueChange={setTimelineFilter}>
              <SelectTrigger className="w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ENTRY_TYPE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {timeline && timeline.length > 0 ? (
            <div className="space-y-3">
              {timeline.map((entry) => (
                <TimelineCard
                  key={entry.id}
                  entry={entry}
                  showConversationTitle
                />
              ))}
            </div>
          ) : (
            <p className="py-8 text-center text-sm text-muted-foreground">
              No entries found.
            </p>
          )}
        </TabsContent>
      </Tabs>

      <NewConversationDialog
        clientId={clientId}
        open={newConvOpen}
        onOpenChange={setNewConvOpen}
        onCreated={(id) => setSelectedConvId(id)}
      />
    </div>
  );
}
