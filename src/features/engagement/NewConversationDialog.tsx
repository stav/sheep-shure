import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCreateConversation } from "@/hooks/useConversations";

interface NewConversationDialogProps {
  clientId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (id: string) => void;
}

export function NewConversationDialog({
  clientId,
  open,
  onOpenChange,
  onCreated,
}: NewConversationDialogProps) {
  const [title, setTitle] = useState("");
  const createConversation = useCreateConversation();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;

    createConversation.mutate(
      { client_id: clientId, title: title.trim() },
      {
        onSuccess: (data) => {
          setTitle("");
          onOpenChange(false);
          onCreated?.(data.id);
        },
      }
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New Conversation</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="conv-title">Title</Label>
              <Input
                id="conv-title"
                placeholder="e.g., AEP 2026 enrollment"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                autoFocus
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={!title.trim() || createConversation.isPending}
            >
              {createConversation.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
