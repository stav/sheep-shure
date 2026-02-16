import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type {
  Conversation,
  ConversationListItem,
  ConversationEntry,
  TimelineEntry,
  CreateConversationInput,
  UpdateConversationInput,
  CreateConversationEntryInput,
  UpdateConversationEntryInput,
  EntryType,
} from "@/types";

// ── Conversations ────────────────────────────────────────────────────────────

export function useConversations(clientId?: string) {
  return useQuery({
    queryKey: ["conversations", clientId],
    queryFn: () =>
      tauriInvoke<ConversationListItem[]>("get_conversations", {
        clientId: clientId!,
      }),
    enabled: !!clientId,
  });
}

export function useConversation(id?: string) {
  return useQuery({
    queryKey: ["conversation", id],
    queryFn: () => tauriInvoke<Conversation>("get_conversation", { id: id! }),
    enabled: !!id,
  });
}

export function useCreateConversation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateConversationInput) =>
      tauriInvoke<Conversation>("create_conversation", { input }),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: ["conversations", variables.client_id],
      });
    },
  });
}

export function useUpdateConversation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: string;
      input: UpdateConversationInput;
    }) => tauriInvoke<Conversation>("update_conversation", { id, input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      queryClient.invalidateQueries({ queryKey: ["conversation"] });
      queryClient.invalidateQueries({ queryKey: ["client_timeline"] });
      queryClient.invalidateQueries({ queryKey: ["pending_follow_ups"] });
    },
  });
}

// ── Conversation Entries ─────────────────────────────────────────────────────

export function useConversationEntries(conversationId?: string) {
  return useQuery({
    queryKey: ["conversation_entries", conversationId],
    queryFn: () =>
      tauriInvoke<ConversationEntry[]>("get_conversation_entries", {
        conversationId: conversationId!,
      }),
    enabled: !!conversationId,
  });
}

export function useCreateConversationEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateConversationEntryInput) =>
      tauriInvoke<ConversationEntry>("create_conversation_entry", { input }),
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: ["conversation_entries", variables.conversation_id],
      });
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      queryClient.invalidateQueries({ queryKey: ["client_timeline"] });
    },
  });
}

export function useUpdateConversationEntry() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      id,
      input,
    }: {
      id: string;
      input: UpdateConversationEntryInput;
    }) =>
      tauriInvoke<ConversationEntry>("update_conversation_entry", {
        id,
        input,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["conversation_entries"] });
      queryClient.invalidateQueries({ queryKey: ["conversations"] });
      queryClient.invalidateQueries({ queryKey: ["client_timeline"] });
    },
  });
}

// ── Timeline ─────────────────────────────────────────────────────────────────

export function useClientTimeline(
  clientId?: string,
  entryType?: EntryType | null
) {
  return useQuery({
    queryKey: ["client_timeline", clientId, entryType],
    queryFn: () =>
      tauriInvoke<TimelineEntry[]>("get_client_timeline", {
        clientId: clientId!,
        entryTypeFilter: entryType ?? null,
        limit: 100,
        offset: 0,
      }),
    enabled: !!clientId,
  });
}

export function usePendingFollowUps(clientId?: string) {
  return useQuery({
    queryKey: ["pending_follow_ups", clientId],
    queryFn: () =>
      tauriInvoke<TimelineEntry[]>("get_pending_follow_ups", {
        clientId: clientId ?? null,
      }),
    enabled: clientId !== undefined,
  });
}
