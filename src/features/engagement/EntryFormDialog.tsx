import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
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
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useCreateConversationEntry, useUpdateConversationEntry } from "@/hooks/useConversations";
import type { ConversationEntry } from "@/types";

type UserEntryType = "CALL" | "EMAIL" | "MEETING" | "SMS" | "NOTE";

const ENTRY_TYPES: { value: UserEntryType; label: string }[] = [
  { value: "CALL", label: "Call" },
  { value: "EMAIL", label: "Email" },
  { value: "MEETING", label: "Meeting" },
  { value: "SMS", label: "SMS" },
  { value: "NOTE", label: "Note" },
];

const entrySchema = z.object({
  entry_type: z.enum(["CALL", "EMAIL", "MEETING", "SMS", "NOTE"]),
  subject: z.string().optional(),
  body: z.string().optional(),
  occurred_at: z.string().optional(),
  follow_up_date: z.string().optional(),
  follow_up_note: z.string().optional(),
  call_direction: z.enum(["INBOUND", "OUTBOUND"]).optional(),
  call_duration: z.coerce.number().int().min(0).optional(),
  call_outcome: z
    .enum([
      "ANSWERED",
      "NO_ANSWER",
      "VOICEMAIL",
      "BUSY",
      "CALLBACK_REQUESTED",
      "WRONG_NUMBER",
    ])
    .optional(),
  call_phone_number: z.string().optional(),
  meeting_location: z.string().optional(),
  meeting_type: z.enum(["IN_PERSON", "VIDEO", "PHONE"]).optional(),
  email_to: z.string().optional(),
  email_from: z.string().optional(),
});

type EntryFormValues = z.infer<typeof entrySchema>;

interface EntryFormDialogProps {
  conversationId: string;
  clientId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editEntry?: ConversationEntry;
}

export function EntryFormDialog({
  conversationId,
  clientId,
  open,
  onOpenChange,
  editEntry,
}: EntryFormDialogProps) {
  const isEditing = !!editEntry;
  const createEntry = useCreateConversationEntry();
  const updateEntry = useUpdateConversationEntry();

  const form = useForm<EntryFormValues>({
    resolver: zodResolver(entrySchema),
    defaultValues: {
      entry_type: "NOTE",
    },
  });

  const entryType = form.watch("entry_type");

  useEffect(() => {
    if (open) {
      if (editEntry) {
        // Convert SQLite datetime "2026-02-15 21:44:09" to datetime-local "2026-02-15T21:44"
        const toDatetimeLocal = (v?: string) => {
          if (!v) return "";
          return v.replace(" ", "T").slice(0, 16);
        };
        // Extract just the date portion for date inputs
        const toDateOnly = (v?: string) => {
          if (!v) return "";
          return v.slice(0, 10);
        };

        form.reset({
          entry_type: editEntry.entry_type as UserEntryType,
          subject: editEntry.subject ?? "",
          body: editEntry.body ?? "",
          occurred_at: toDatetimeLocal(editEntry.occurred_at),
          follow_up_date: toDateOnly(editEntry.follow_up_date),
          follow_up_note: editEntry.follow_up_note ?? "",
          call_direction: editEntry.call_direction ?? undefined,
          call_duration: editEntry.call_duration ?? undefined,
          call_outcome: editEntry.call_outcome ?? undefined,
          call_phone_number: editEntry.call_phone_number ?? "",
          meeting_location: editEntry.meeting_location ?? "",
          meeting_type: editEntry.meeting_type ?? undefined,
          email_to: editEntry.email_to ?? "",
          email_from: editEntry.email_from ?? "",
        });
      } else {
        form.reset({ entry_type: "NOTE" });
      }
    }
  }, [open, form, editEntry]);

  const isMutating = createEntry.isPending || updateEntry.isPending;

  const handleSubmit = (values: EntryFormValues) => {
    const onSuccess = () => {
      form.reset();
      onOpenChange(false);
    };

    if (isEditing) {
      updateEntry.mutate(
        {
          id: editEntry.id,
          input: {
            subject: values.subject || undefined,
            body: values.body || undefined,
            occurred_at: values.occurred_at || undefined,
            follow_up_date: values.follow_up_date || undefined,
            follow_up_note: values.follow_up_note || undefined,
            call_direction: values.call_direction || undefined,
            call_duration: values.call_duration || undefined,
            call_outcome: values.call_outcome || undefined,
            call_phone_number: values.call_phone_number || undefined,
            meeting_location: values.meeting_location || undefined,
            meeting_type: values.meeting_type || undefined,
            email_to: values.email_to || undefined,
            email_from: values.email_from || undefined,
          },
        },
        { onSuccess }
      );
    } else {
      createEntry.mutate(
        {
          conversation_id: conversationId,
          client_id: clientId,
          entry_type: values.entry_type,
          subject: values.subject || undefined,
          body: values.body || undefined,
          occurred_at: values.occurred_at || undefined,
          follow_up_date: values.follow_up_date || undefined,
          follow_up_note: values.follow_up_note || undefined,
          call_direction: values.call_direction || undefined,
          call_duration: values.call_duration || undefined,
          call_outcome: values.call_outcome || undefined,
          call_phone_number: values.call_phone_number || undefined,
          meeting_location: values.meeting_location || undefined,
          meeting_type: values.meeting_type || undefined,
          email_to: values.email_to || undefined,
          email_from: values.email_from || undefined,
        },
        { onSuccess }
      );
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{isEditing ? "Edit Entry" : "Add Entry"}</DialogTitle>
        </DialogHeader>
        <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
          {/* Entry Type */}
          <div className="space-y-2">
            <Label>Type</Label>
            <Select
              value={entryType}
              onValueChange={(v) =>
                form.setValue("entry_type", v as UserEntryType)
              }
              disabled={isEditing}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ENTRY_TYPES.map((t) => (
                  <SelectItem key={t.value} value={t.value}>
                    {t.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Shared fields */}
          <div className="space-y-2">
            <Label htmlFor="entry-subject">Subject</Label>
            <Input
              id="entry-subject"
              {...form.register("subject")}
              placeholder="Brief subject line"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="entry-body">Notes</Label>
            <Textarea
              id="entry-body"
              {...form.register("body")}
              placeholder="Details..."
              rows={3}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="entry-occurred">Date/Time</Label>
            <Input
              id="entry-occurred"
              type="datetime-local"
              {...form.register("occurred_at")}
            />
          </div>

          {/* CALL-specific fields */}
          {entryType === "CALL" && (
            <div className="space-y-4 rounded-md border p-3">
              <p className="text-xs font-medium uppercase text-muted-foreground">
                Call Details
              </p>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Direction</Label>
                  <Select
                    value={form.watch("call_direction") || ""}
                    onValueChange={(v) =>
                      form.setValue(
                        "call_direction",
                        v as "INBOUND" | "OUTBOUND"
                      )
                    }
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select..." />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="INBOUND">Inbound</SelectItem>
                      <SelectItem value="OUTBOUND">Outbound</SelectItem>
                    </SelectContent>
                  </Select>
                  {form.formState.errors.call_direction && (
                    <p className="text-xs text-destructive">Required for calls</p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label>Outcome</Label>
                  <Select
                    value={form.watch("call_outcome") || ""}
                    onValueChange={(v) =>
                      form.setValue("call_outcome", v as EntryFormValues["call_outcome"])
                    }
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select..." />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="ANSWERED">Answered</SelectItem>
                      <SelectItem value="NO_ANSWER">No Answer</SelectItem>
                      <SelectItem value="VOICEMAIL">Voicemail</SelectItem>
                      <SelectItem value="BUSY">Busy</SelectItem>
                      <SelectItem value="CALLBACK_REQUESTED">
                        Callback Requested
                      </SelectItem>
                      <SelectItem value="WRONG_NUMBER">Wrong Number</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="call-duration">Duration (seconds)</Label>
                  <Input
                    id="call-duration"
                    type="number"
                    min={0}
                    {...form.register("call_duration")}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="call-phone">Phone Number</Label>
                  <Input
                    id="call-phone"
                    {...form.register("call_phone_number")}
                    placeholder="(555) 123-4567"
                  />
                </div>
              </div>
            </div>
          )}

          {/* EMAIL-specific fields */}
          {entryType === "EMAIL" && (
            <div className="space-y-4 rounded-md border p-3">
              <p className="text-xs font-medium uppercase text-muted-foreground">
                Email Details
              </p>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="email-from">From</Label>
                  <Input
                    id="email-from"
                    {...form.register("email_from")}
                    placeholder="sender@example.com"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="email-to">To</Label>
                  <Input
                    id="email-to"
                    {...form.register("email_to")}
                    placeholder="recipient@example.com"
                  />
                </div>
              </div>
            </div>
          )}

          {/* MEETING-specific fields */}
          {entryType === "MEETING" && (
            <div className="space-y-4 rounded-md border p-3">
              <p className="text-xs font-medium uppercase text-muted-foreground">
                Meeting Details
              </p>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label>Type</Label>
                  <Select
                    value={form.watch("meeting_type") || ""}
                    onValueChange={(v) =>
                      form.setValue(
                        "meeting_type",
                        v as "IN_PERSON" | "VIDEO" | "PHONE"
                      )
                    }
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select..." />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="IN_PERSON">In Person</SelectItem>
                      <SelectItem value="VIDEO">Video</SelectItem>
                      <SelectItem value="PHONE">Phone</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="meeting-loc">Location</Label>
                  <Input
                    id="meeting-loc"
                    {...form.register("meeting_location")}
                    placeholder="Office, Zoom link, etc."
                  />
                </div>
              </div>
            </div>
          )}

          {/* Follow-up */}
          <div className="space-y-4 rounded-md border p-3">
            <p className="text-xs font-medium uppercase text-muted-foreground">
              Follow-up (optional)
            </p>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="follow-up-date">Date</Label>
                <Input
                  id="follow-up-date"
                  type="date"
                  {...form.register("follow_up_date")}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="follow-up-note">Note</Label>
                <Input
                  id="follow-up-note"
                  {...form.register("follow_up_note")}
                  placeholder="Remind to..."
                />
              </div>
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
            <Button type="submit" disabled={isMutating}>
              {isMutating ? "Saving..." : isEditing ? "Save Changes" : "Add Entry"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
