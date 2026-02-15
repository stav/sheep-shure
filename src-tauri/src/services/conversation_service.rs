use rusqlite::Connection;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{
    Conversation, ConversationEntry, ConversationListItem, CreateConversationEntryInput,
    CreateConversationInput, TimelineEntry, UpdateConversationEntryInput, UpdateConversationInput,
};
use crate::repositories::conversation_repo;

pub fn get_conversations(
    conn: &Connection,
    client_id: &str,
) -> Result<Vec<ConversationListItem>, AppError> {
    conversation_repo::get_conversations(conn, client_id)
}

pub fn get_conversation(conn: &Connection, id: &str) -> Result<Conversation, AppError> {
    conversation_repo::get_conversation(conn, id)
}

pub fn create_conversation(
    conn: &Connection,
    input: &CreateConversationInput,
) -> Result<Conversation, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::Validation(
            "Conversation title cannot be empty".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    conversation_repo::create_conversation(conn, &id, input)?;
    conversation_repo::get_conversation(conn, &id)
}

pub fn update_conversation(
    conn: &Connection,
    id: &str,
    input: &UpdateConversationInput,
) -> Result<Conversation, AppError> {
    if let Some(ref status) = input.status {
        let valid = ["OPEN", "CLOSED", "ARCHIVED"];
        if !valid.contains(&status.as_str()) {
            return Err(AppError::Validation(format!(
                "Invalid conversation status: {}",
                status
            )));
        }
    }

    conversation_repo::update_conversation(conn, id, input)?;
    conversation_repo::get_conversation(conn, id)
}

pub fn get_conversation_entries(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<ConversationEntry>, AppError> {
    conversation_repo::get_conversation_entries(conn, conversation_id)
}

pub fn create_conversation_entry(
    conn: &Connection,
    input: &CreateConversationEntryInput,
) -> Result<ConversationEntry, AppError> {
    let valid_types = ["CALL", "EMAIL", "MEETING", "SMS", "NOTE", "SYSTEM"];
    if !valid_types.contains(&input.entry_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid entry type: {}",
            input.entry_type
        )));
    }

    // CALL entries require call_direction
    if input.entry_type == "CALL" && input.call_direction.is_none() {
        return Err(AppError::Validation(
            "Call entries require a call direction (INBOUND or OUTBOUND)".to_string(),
        ));
    }

    // SYSTEM entries should not be created directly through this path
    if input.entry_type == "SYSTEM" {
        return Err(AppError::Validation(
            "System entries cannot be created directly; use system event integration".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    conversation_repo::create_conversation_entry(conn, &id, input)?;
    conversation_repo::get_conversation_entry(conn, &id)
}

pub fn update_conversation_entry(
    conn: &Connection,
    id: &str,
    input: &UpdateConversationEntryInput,
) -> Result<ConversationEntry, AppError> {
    conversation_repo::update_conversation_entry(conn, id, input)?;
    conversation_repo::get_conversation_entry(conn, id)
}

pub fn get_client_timeline(
    conn: &Connection,
    client_id: &str,
    entry_type_filter: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<TimelineEntry>, AppError> {
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    conversation_repo::get_client_timeline(conn, client_id, entry_type_filter, limit, offset)
}

pub fn get_pending_follow_ups(
    conn: &Connection,
    client_id: Option<&str>,
) -> Result<Vec<TimelineEntry>, AppError> {
    conversation_repo::get_pending_follow_ups(conn, client_id)
}

/// Create a system event entry. Finds or auto-creates a "System Activity" conversation.
pub fn create_system_event(
    conn: &Connection,
    client_id: &str,
    event_type: &str,
    event_data: Option<&str>,
) -> Result<(), AppError> {
    let conv_uuid = Uuid::new_v4().to_string();
    let conversation_id =
        conversation_repo::find_or_create_system_conversation(conn, &conv_uuid, client_id)?;

    let entry_id = Uuid::new_v4().to_string();
    conversation_repo::create_system_entry(
        conn,
        &entry_id,
        &conversation_id,
        client_id,
        event_type,
        event_data,
    )?;

    Ok(())
}
