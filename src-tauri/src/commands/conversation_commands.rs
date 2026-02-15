use tauri::State;

use crate::db::DbState;
use crate::models::{
    Conversation, ConversationEntry, ConversationListItem, CreateConversationEntryInput,
    CreateConversationInput, TimelineEntry, UpdateConversationEntryInput, UpdateConversationInput,
};
use crate::services::conversation_service;

#[tauri::command]
pub fn get_conversations(
    client_id: String,
    state: State<'_, DbState>,
) -> Result<Vec<ConversationListItem>, String> {
    state
        .with_conn(|conn| conversation_service::get_conversations(conn, &client_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_conversation(id: String, state: State<'_, DbState>) -> Result<Conversation, String> {
    state
        .with_conn(|conn| conversation_service::get_conversation(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_conversation(
    input: CreateConversationInput,
    state: State<'_, DbState>,
) -> Result<Conversation, String> {
    state
        .with_conn(|conn| conversation_service::create_conversation(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_conversation(
    id: String,
    input: UpdateConversationInput,
    state: State<'_, DbState>,
) -> Result<Conversation, String> {
    state
        .with_conn(|conn| conversation_service::update_conversation(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_conversation_entries(
    conversation_id: String,
    state: State<'_, DbState>,
) -> Result<Vec<ConversationEntry>, String> {
    state
        .with_conn(|conn| conversation_service::get_conversation_entries(conn, &conversation_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_conversation_entry(
    input: CreateConversationEntryInput,
    state: State<'_, DbState>,
) -> Result<ConversationEntry, String> {
    state
        .with_conn(|conn| conversation_service::create_conversation_entry(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_conversation_entry(
    id: String,
    input: UpdateConversationEntryInput,
    state: State<'_, DbState>,
) -> Result<ConversationEntry, String> {
    state
        .with_conn(|conn| conversation_service::update_conversation_entry(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_client_timeline(
    client_id: String,
    entry_type_filter: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, DbState>,
) -> Result<Vec<TimelineEntry>, String> {
    state
        .with_conn(|conn| {
            conversation_service::get_client_timeline(
                conn,
                &client_id,
                entry_type_filter.as_deref(),
                limit,
                offset,
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pending_follow_ups(
    client_id: Option<String>,
    state: State<'_, DbState>,
) -> Result<Vec<TimelineEntry>, String> {
    state
        .with_conn(|conn| conversation_service::get_pending_follow_ups(conn, client_id.as_deref()))
        .map_err(|e| e.to_string())
}
