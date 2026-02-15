import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Plus, MoreHorizontal, Pin, PinOff, Archive, XCircle, CheckCircle, Trash2 } from "lucide-react";
import {
  useConversation,
  useConversationEntries,
  useUpdateConversation,
  useUpdateConversationEntry,
} from "@/hooks/useConversations";
import { TimelineCard } from "./TimelineCard";
import { EntryFormDialog } from "./EntryFormDialog";
import type { ConversationEntry } from "@/types";

interface ConversationDetailProps {
  conversationId: string;
  clientId: string;
  onDeleted?: () => void;
}

export function ConversationDetail({
  conversationId,
  clientId,
  onDeleted,
}: ConversationDetailProps) {
  const { data: conversation } = useConversation(conversationId);
  const { data: entries, isLoading } = useConversationEntries(conversationId);
  const updateConversation = useUpdateConversation();
  const updateEntry = useUpdateConversationEntry();
  const [entryFormOpen, setEntryFormOpen] = useState(false);
  const [editingEntry, setEditingEntry] = useState<ConversationEntry | null>(null);
  const [confirmDeleteThread, setConfirmDeleteThread] = useState(false);
  const [confirmDeleteEntryId, setConfirmDeleteEntryId] = useState<string | null>(null);

  if (!conversation) return null;

  const isPinned = !!conversation.is_pinned;

  const togglePin = () => {
    updateConversation.mutate({
      id: conversationId,
      input: { is_pinned: isPinned ? 0 : 1 },
    });
  };

  const setStatus = (status: "OPEN" | "CLOSED" | "ARCHIVED") => {
    updateConversation.mutate({
      id: conversationId,
      input: { status },
    });
  };

  const executeDeleteThread = () => {
    updateConversation.mutate(
      { id: conversationId, input: { is_active: 0 } },
      { onSuccess: () => onDeleted?.() }
    );
    setConfirmDeleteThread(false);
  };

  const executeDeleteEntry = () => {
    if (!confirmDeleteEntryId) return;
    updateEntry.mutate({ id: confirmDeleteEntryId, input: { is_active: 0 } });
    setConfirmDeleteEntryId(null);
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b px-4 py-3">
        <div className="min-w-0">
          <h3 className="truncate text-sm font-semibold">
            {conversation.title}
          </h3>
          <div className="flex items-center gap-2 mt-0.5">
            <Badge
              variant="outline"
              className={`text-[10px] px-1.5 py-0 ${
                conversation.status === "OPEN"
                  ? "bg-green-100 text-green-700"
                  : conversation.status === "CLOSED"
                  ? "bg-gray-100 text-gray-600"
                  : "bg-yellow-100 text-yellow-700"
              }`}
            >
              {conversation.status}
            </Badge>
            {isPinned && (
              <Pin className="h-3 w-3 text-muted-foreground" />
            )}
          </div>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setEntryFormOpen(true)}
          >
            <Plus className="mr-1 h-3.5 w-3.5" />
            Add Entry
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={togglePin}>
                {isPinned ? (
                  <>
                    <PinOff className="mr-2 h-4 w-4" /> Unpin
                  </>
                ) : (
                  <>
                    <Pin className="mr-2 h-4 w-4" /> Pin
                  </>
                )}
              </DropdownMenuItem>
              {conversation.status !== "OPEN" && (
                <DropdownMenuItem onClick={() => setStatus("OPEN")}>
                  <CheckCircle className="mr-2 h-4 w-4" /> Reopen
                </DropdownMenuItem>
              )}
              {conversation.status !== "CLOSED" && (
                <DropdownMenuItem onClick={() => setStatus("CLOSED")}>
                  <XCircle className="mr-2 h-4 w-4" /> Close
                </DropdownMenuItem>
              )}
              {conversation.status !== "ARCHIVED" && (
                <DropdownMenuItem onClick={() => setStatus("ARCHIVED")}>
                  <Archive className="mr-2 h-4 w-4" /> Archive
                </DropdownMenuItem>
              )}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => setConfirmDeleteThread(true)}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" /> Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* Entries */}
      <div className="flex-1 overflow-y-auto p-4">
        {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading...</p>
        ) : entries && entries.length > 0 ? (
          <div className="space-y-3">
            {entries.map((entry) => (
              <TimelineCard
                key={entry.id}
                entry={entry}
                onDelete={() => setConfirmDeleteEntryId(entry.id)}
                onEdit={() => setEditingEntry(entry)}
              />
            ))}
          </div>
        ) : (
          <p className="py-8 text-center text-sm text-muted-foreground">
            No entries yet. Click "Add Entry" to get started.
          </p>
        )}
      </div>

      {/* New Entry Dialog */}
      <EntryFormDialog
        conversationId={conversationId}
        clientId={clientId}
        open={entryFormOpen}
        onOpenChange={setEntryFormOpen}
      />

      {/* Edit Entry Dialog */}
      {editingEntry && (
        <EntryFormDialog
          conversationId={conversationId}
          clientId={clientId}
          open={!!editingEntry}
          onOpenChange={(open) => { if (!open) setEditingEntry(null); }}
          editEntry={editingEntry}
        />
      )}

      {/* Confirm Delete Thread */}
      <Dialog open={confirmDeleteThread} onOpenChange={setConfirmDeleteThread}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Thread</DialogTitle>
            <DialogDescription>
              Delete this conversation thread and all its entries? This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmDeleteThread(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={executeDeleteThread}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Confirm Delete Entry */}
      <Dialog open={!!confirmDeleteEntryId} onOpenChange={(open) => { if (!open) setConfirmDeleteEntryId(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Entry</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete this entry?
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirmDeleteEntryId(null)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={executeDeleteEntry}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
