use rusqlite::{params, Connection};

use crate::error::AppError;
use crate::models::{
    Conversation, ConversationEntry, ConversationListItem, CreateConversationEntryInput,
    CreateConversationInput, TimelineEntry, UpdateConversationEntryInput, UpdateConversationInput,
};

// ── Conversations ────────────────────────────────────────────────────────────

pub fn get_conversations(
    conn: &Connection,
    client_id: &str,
) -> Result<Vec<ConversationListItem>, AppError> {
    let sql = "SELECT c.id, c.client_id, c.title, c.status, c.is_pinned,
                      COALESCE(cnt.entry_count, 0),
                      cnt.last_entry_at,
                      c.created_at
               FROM conversations c
               LEFT JOIN (
                   SELECT conversation_id,
                          COUNT(*) AS entry_count,
                          MAX(occurred_at) AS last_entry_at
                   FROM conversation_entries
                   WHERE is_active = 1
                   GROUP BY conversation_id
               ) cnt ON cnt.conversation_id = c.id
               WHERE c.client_id = ?1 AND c.is_active = 1
               ORDER BY c.is_pinned DESC, COALESCE(cnt.last_entry_at, c.created_at) DESC";

    let mut stmt = conn.prepare(sql)?;
    let items = stmt
        .query_map(params![client_id], |row| {
            Ok(ConversationListItem {
                id: row.get(0)?,
                client_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                is_pinned: row.get(4)?,
                entry_count: row.get(5)?,
                last_entry_at: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_conversation(conn: &Connection, id: &str) -> Result<Conversation, AppError> {
    let sql = "SELECT id, client_id, title, status, is_pinned, is_active, created_at, updated_at
               FROM conversations WHERE id = ?1";

    conn.query_row(sql, params![id], |row| {
        Ok(Conversation {
            id: row.get(0)?,
            client_id: row.get(1)?,
            title: row.get(2)?,
            status: row.get(3)?,
            is_pinned: row.get(4)?,
            is_active: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Conversation {} not found", id))
        }
        _ => AppError::Database(e.to_string()),
    })
}

pub fn create_conversation(
    conn: &Connection,
    id: &str,
    input: &CreateConversationInput,
) -> Result<(), AppError> {
    let sql = "INSERT INTO conversations (id, client_id, title) VALUES (?1, ?2, ?3)";
    conn.execute(sql, params![id, input.client_id, input.title])?;
    Ok(())
}

pub fn update_conversation(
    conn: &Connection,
    id: &str,
    input: &UpdateConversationInput,
) -> Result<(), AppError> {
    let sql = "UPDATE conversations SET
               title = COALESCE(?2, title),
               status = COALESCE(?3, status),
               is_pinned = COALESCE(?4, is_pinned),
               is_active = COALESCE(?5, is_active)
               WHERE id = ?1";

    let rows = conn.execute(
        sql,
        params![id, input.title, input.status, input.is_pinned, input.is_active],
    )?;

    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Conversation {} not found",
            id
        )));
    }
    Ok(())
}

// ── Conversation Entries ─────────────────────────────────────────────────────

pub fn get_conversation_entries(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<ConversationEntry>, AppError> {
    let sql = "SELECT id, conversation_id, client_id, entry_type, subject, body,
                      occurred_at, follow_up_date, follow_up_note,
                      call_direction, call_duration, call_outcome, call_phone_number,
                      meeting_location, meeting_type, email_to, email_from,
                      system_event_type, system_event_data,
                      is_active, created_at, updated_at
               FROM conversation_entries
               WHERE conversation_id = ?1 AND is_active = 1
               ORDER BY occurred_at DESC";

    let mut stmt = conn.prepare(sql)?;
    let items = stmt
        .query_map(params![conversation_id], |row| {
            Ok(ConversationEntry {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                client_id: row.get(2)?,
                entry_type: row.get(3)?,
                subject: row.get(4)?,
                body: row.get(5)?,
                occurred_at: row.get(6)?,
                follow_up_date: row.get(7)?,
                follow_up_note: row.get(8)?,
                call_direction: row.get(9)?,
                call_duration: row.get(10)?,
                call_outcome: row.get(11)?,
                call_phone_number: row.get(12)?,
                meeting_location: row.get(13)?,
                meeting_type: row.get(14)?,
                email_to: row.get(15)?,
                email_from: row.get(16)?,
                system_event_type: row.get(17)?,
                system_event_data: row.get(18)?,
                is_active: row.get(19)?,
                created_at: row.get(20)?,
                updated_at: row.get(21)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_conversation_entry(
    conn: &Connection,
    id: &str,
) -> Result<ConversationEntry, AppError> {
    let sql = "SELECT id, conversation_id, client_id, entry_type, subject, body,
                      occurred_at, follow_up_date, follow_up_note,
                      call_direction, call_duration, call_outcome, call_phone_number,
                      meeting_location, meeting_type, email_to, email_from,
                      system_event_type, system_event_data,
                      is_active, created_at, updated_at
               FROM conversation_entries WHERE id = ?1";

    conn.query_row(sql, params![id], |row| {
        Ok(ConversationEntry {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            client_id: row.get(2)?,
            entry_type: row.get(3)?,
            subject: row.get(4)?,
            body: row.get(5)?,
            occurred_at: row.get(6)?,
            follow_up_date: row.get(7)?,
            follow_up_note: row.get(8)?,
            call_direction: row.get(9)?,
            call_duration: row.get(10)?,
            call_outcome: row.get(11)?,
            call_phone_number: row.get(12)?,
            meeting_location: row.get(13)?,
            meeting_type: row.get(14)?,
            email_to: row.get(15)?,
            email_from: row.get(16)?,
            system_event_type: row.get(17)?,
            system_event_data: row.get(18)?,
            is_active: row.get(19)?,
            created_at: row.get(20)?,
            updated_at: row.get(21)?,
        })
    })
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Conversation entry {} not found", id))
        }
        _ => AppError::Database(e.to_string()),
    })
}

pub fn create_conversation_entry(
    conn: &Connection,
    id: &str,
    input: &CreateConversationEntryInput,
) -> Result<(), AppError> {
    let sql = "INSERT INTO conversation_entries
               (id, conversation_id, client_id, entry_type, subject, body, occurred_at,
                follow_up_date, follow_up_note,
                call_direction, call_duration, call_outcome, call_phone_number,
                meeting_location, meeting_type, email_to, email_from)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE(?7, datetime('now')),
                        ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)";

    conn.execute(
        sql,
        params![
            id,
            input.conversation_id,
            input.client_id,
            input.entry_type,
            input.subject,
            input.body,
            input.occurred_at,
            input.follow_up_date,
            input.follow_up_note,
            input.call_direction,
            input.call_duration,
            input.call_outcome,
            input.call_phone_number,
            input.meeting_location,
            input.meeting_type,
            input.email_to,
            input.email_from,
        ],
    )?;

    Ok(())
}

pub fn update_conversation_entry(
    conn: &Connection,
    id: &str,
    input: &UpdateConversationEntryInput,
) -> Result<(), AppError> {
    let sql = "UPDATE conversation_entries SET
               subject = COALESCE(?2, subject),
               body = COALESCE(?3, body),
               occurred_at = COALESCE(?4, occurred_at),
               follow_up_date = COALESCE(?5, follow_up_date),
               follow_up_note = COALESCE(?6, follow_up_note),
               call_direction = COALESCE(?7, call_direction),
               call_duration = COALESCE(?8, call_duration),
               call_outcome = COALESCE(?9, call_outcome),
               call_phone_number = COALESCE(?10, call_phone_number),
               meeting_location = COALESCE(?11, meeting_location),
               meeting_type = COALESCE(?12, meeting_type),
               email_to = COALESCE(?13, email_to),
               email_from = COALESCE(?14, email_from),
               is_active = COALESCE(?15, is_active)
               WHERE id = ?1";

    let rows = conn.execute(
        sql,
        params![
            id,
            input.subject,
            input.body,
            input.occurred_at,
            input.follow_up_date,
            input.follow_up_note,
            input.call_direction,
            input.call_duration,
            input.call_outcome,
            input.call_phone_number,
            input.meeting_location,
            input.meeting_type,
            input.email_to,
            input.email_from,
            input.is_active,
        ],
    )?;

    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Conversation entry {} not found",
            id
        )));
    }
    Ok(())
}

// ── Timeline (cross-thread) ─────────────────────────────────────────────────

pub fn get_client_timeline(
    conn: &Connection,
    client_id: &str,
    entry_type_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TimelineEntry>, AppError> {
    let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(et) = entry_type_filter {
            (
                "SELECT ce.id, ce.conversation_id, c.title, ce.client_id, ce.entry_type,
                        ce.subject, ce.body, ce.occurred_at,
                        ce.follow_up_date, ce.follow_up_note,
                        ce.call_direction, ce.call_duration, ce.call_outcome, ce.call_phone_number,
                        ce.meeting_location, ce.meeting_type, ce.email_to, ce.email_from,
                        ce.system_event_type, ce.system_event_data, ce.created_at
                 FROM conversation_entries ce
                 JOIN conversations c ON c.id = ce.conversation_id
                 WHERE ce.client_id = ?1 AND ce.entry_type = ?2 AND ce.is_active = 1 AND c.is_active = 1
                 ORDER BY ce.occurred_at DESC
                 LIMIT ?3 OFFSET ?4"
                    .to_string(),
                vec![
                    Box::new(client_id.to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(et.to_string()),
                    Box::new(limit),
                    Box::new(offset),
                ],
            )
        } else {
            (
                "SELECT ce.id, ce.conversation_id, c.title, ce.client_id, ce.entry_type,
                        ce.subject, ce.body, ce.occurred_at,
                        ce.follow_up_date, ce.follow_up_note,
                        ce.call_direction, ce.call_duration, ce.call_outcome, ce.call_phone_number,
                        ce.meeting_location, ce.meeting_type, ce.email_to, ce.email_from,
                        ce.system_event_type, ce.system_event_data, ce.created_at
                 FROM conversation_entries ce
                 JOIN conversations c ON c.id = ce.conversation_id
                 WHERE ce.client_id = ?1 AND ce.is_active = 1 AND c.is_active = 1
                 ORDER BY ce.occurred_at DESC
                 LIMIT ?2 OFFSET ?3"
                    .to_string(),
                vec![
                    Box::new(client_id.to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit),
                    Box::new(offset),
                ],
            )
        };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(TimelineEntry {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                conversation_title: row.get(2)?,
                client_id: row.get(3)?,
                entry_type: row.get(4)?,
                subject: row.get(5)?,
                body: row.get(6)?,
                occurred_at: row.get(7)?,
                follow_up_date: row.get(8)?,
                follow_up_note: row.get(9)?,
                call_direction: row.get(10)?,
                call_duration: row.get(11)?,
                call_outcome: row.get(12)?,
                call_phone_number: row.get(13)?,
                meeting_location: row.get(14)?,
                meeting_type: row.get(15)?,
                email_to: row.get(16)?,
                email_from: row.get(17)?,
                system_event_type: row.get(18)?,
                system_event_data: row.get(19)?,
                created_at: row.get(20)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn get_pending_follow_ups(
    conn: &Connection,
    client_id: Option<&str>,
) -> Result<Vec<TimelineEntry>, AppError> {
    let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
        if let Some(cid) = client_id {
            (
                "SELECT ce.id, ce.conversation_id, c.title, ce.client_id, ce.entry_type,
                        ce.subject, ce.body, ce.occurred_at,
                        ce.follow_up_date, ce.follow_up_note,
                        ce.call_direction, ce.call_duration, ce.call_outcome, ce.call_phone_number,
                        ce.meeting_location, ce.meeting_type, ce.email_to, ce.email_from,
                        ce.system_event_type, ce.system_event_data, ce.created_at
                 FROM conversation_entries ce
                 JOIN conversations c ON c.id = ce.conversation_id
                 WHERE ce.client_id = ?1 AND ce.follow_up_date IS NOT NULL
                       AND ce.follow_up_date >= date('now') AND ce.is_active = 1 AND c.is_active = 1
                 ORDER BY ce.follow_up_date ASC"
                    .to_string(),
                vec![Box::new(cid.to_string()) as Box<dyn rusqlite::types::ToSql>],
            )
        } else {
            (
                "SELECT ce.id, ce.conversation_id, c.title, ce.client_id, ce.entry_type,
                        ce.subject, ce.body, ce.occurred_at,
                        ce.follow_up_date, ce.follow_up_note,
                        ce.call_direction, ce.call_duration, ce.call_outcome, ce.call_phone_number,
                        ce.meeting_location, ce.meeting_type, ce.email_to, ce.email_from,
                        ce.system_event_type, ce.system_event_data, ce.created_at
                 FROM conversation_entries ce
                 JOIN conversations c ON c.id = ce.conversation_id
                 WHERE ce.follow_up_date IS NOT NULL
                       AND ce.follow_up_date >= date('now') AND ce.is_active = 1 AND c.is_active = 1
                 ORDER BY ce.follow_up_date ASC"
                    .to_string(),
                vec![],
            )
        };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(TimelineEntry {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                conversation_title: row.get(2)?,
                client_id: row.get(3)?,
                entry_type: row.get(4)?,
                subject: row.get(5)?,
                body: row.get(6)?,
                occurred_at: row.get(7)?,
                follow_up_date: row.get(8)?,
                follow_up_note: row.get(9)?,
                call_direction: row.get(10)?,
                call_duration: row.get(11)?,
                call_outcome: row.get(12)?,
                call_phone_number: row.get(13)?,
                meeting_location: row.get(14)?,
                meeting_type: row.get(15)?,
                email_to: row.get(16)?,
                email_from: row.get(17)?,
                system_event_type: row.get(18)?,
                system_event_data: row.get(19)?,
                created_at: row.get(20)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Find or create the "System Activity" conversation for a client.
pub fn find_or_create_system_conversation(
    conn: &Connection,
    id: &str,
    client_id: &str,
) -> Result<String, AppError> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM conversations WHERE client_id = ?1 AND title = 'System Activity' AND is_active = 1",
            params![client_id],
            |row| row.get(0),
        )
        .ok();

    if let Some(conv_id) = existing {
        return Ok(conv_id);
    }

    conn.execute(
        "INSERT INTO conversations (id, client_id, title) VALUES (?1, ?2, 'System Activity')",
        params![id, client_id],
    )?;
    Ok(id.to_string())
}

/// Insert a SYSTEM entry into a conversation.
pub fn create_system_entry(
    conn: &Connection,
    id: &str,
    conversation_id: &str,
    client_id: &str,
    event_type: &str,
    event_data: Option<&str>,
) -> Result<(), AppError> {
    let sql = "INSERT INTO conversation_entries
               (id, conversation_id, client_id, entry_type, system_event_type, system_event_data)
               VALUES (?1, ?2, ?3, 'SYSTEM', ?4, ?5)";
    conn.execute(sql, params![id, conversation_id, client_id, event_type, event_data])?;
    Ok(())
}
