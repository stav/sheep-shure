use serde::{Deserialize, Serialize};

// ── Conversation (thread container) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub client_id: String,
    pub title: String,
    pub status: String,
    pub is_pinned: i32,
    pub is_active: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationInput {
    pub client_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConversationInput {
    pub title: Option<String>,
    pub status: Option<String>,
    pub is_pinned: Option<i32>,
    pub is_active: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationListItem {
    pub id: String,
    pub client_id: String,
    pub title: String,
    pub status: String,
    pub is_pinned: i32,
    pub entry_count: i64,
    pub last_entry_at: Option<String>,
    pub created_at: Option<String>,
}

// ── Conversation Entry (message within a thread) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub id: String,
    pub conversation_id: String,
    pub client_id: String,
    pub entry_type: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub occurred_at: Option<String>,
    pub follow_up_date: Option<String>,
    pub follow_up_note: Option<String>,
    pub call_direction: Option<String>,
    pub call_duration: Option<i64>,
    pub call_outcome: Option<String>,
    pub call_phone_number: Option<String>,
    pub meeting_location: Option<String>,
    pub meeting_type: Option<String>,
    pub email_to: Option<String>,
    pub email_from: Option<String>,
    pub system_event_type: Option<String>,
    pub system_event_data: Option<String>,
    pub is_active: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationEntryInput {
    pub conversation_id: String,
    pub client_id: String,
    pub entry_type: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub occurred_at: Option<String>,
    pub follow_up_date: Option<String>,
    pub follow_up_note: Option<String>,
    pub call_direction: Option<String>,
    pub call_duration: Option<i64>,
    pub call_outcome: Option<String>,
    pub call_phone_number: Option<String>,
    pub meeting_location: Option<String>,
    pub meeting_type: Option<String>,
    pub email_to: Option<String>,
    pub email_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConversationEntryInput {
    pub subject: Option<String>,
    pub body: Option<String>,
    pub occurred_at: Option<String>,
    pub follow_up_date: Option<String>,
    pub follow_up_note: Option<String>,
    pub call_direction: Option<String>,
    pub call_duration: Option<i64>,
    pub call_outcome: Option<String>,
    pub call_phone_number: Option<String>,
    pub meeting_location: Option<String>,
    pub meeting_type: Option<String>,
    pub email_to: Option<String>,
    pub email_from: Option<String>,
    pub is_active: Option<i32>,
}

// ── Timeline Entry (cross-thread view) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: String,
    pub conversation_id: String,
    pub conversation_title: String,
    pub client_id: String,
    pub entry_type: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub occurred_at: Option<String>,
    pub follow_up_date: Option<String>,
    pub follow_up_note: Option<String>,
    pub call_direction: Option<String>,
    pub call_duration: Option<i64>,
    pub call_outcome: Option<String>,
    pub call_phone_number: Option<String>,
    pub meeting_location: Option<String>,
    pub meeting_type: Option<String>,
    pub email_to: Option<String>,
    pub email_from: Option<String>,
    pub system_event_type: Option<String>,
    pub system_event_data: Option<String>,
    pub created_at: Option<String>,
}
