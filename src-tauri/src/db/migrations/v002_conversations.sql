-- V002: Threaded client engagement / conversation system
-- Replaces unused `notes` table with conversations + conversation_entries

-- ============================================================================
-- Drop unused notes table and its trigger
-- ============================================================================

DROP TRIGGER IF EXISTS notes_updated_at;
DROP TABLE IF EXISTS notes;

-- ============================================================================
-- Conversation Threads
-- ============================================================================

CREATE TABLE IF NOT EXISTS conversations (
    id         TEXT PRIMARY KEY,
    client_id  TEXT NOT NULL REFERENCES clients(id),
    title      TEXT NOT NULL,
    status     TEXT DEFAULT 'OPEN' CHECK (status IN ('OPEN', 'CLOSED', 'ARCHIVED')),
    is_pinned  INTEGER DEFAULT 0,
    is_active  INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- Conversation Entries (messages within a thread)
-- ============================================================================

CREATE TABLE IF NOT EXISTS conversation_entries (
    id                TEXT PRIMARY KEY,
    conversation_id   TEXT NOT NULL REFERENCES conversations(id),
    client_id         TEXT NOT NULL REFERENCES clients(id),
    entry_type        TEXT NOT NULL CHECK (entry_type IN ('CALL', 'EMAIL', 'MEETING', 'SMS', 'NOTE', 'SYSTEM')),
    subject           TEXT,
    body              TEXT,
    occurred_at       TEXT DEFAULT (datetime('now')),
    follow_up_date    TEXT,
    follow_up_note    TEXT,
    call_direction    TEXT CHECK (call_direction IN ('INBOUND', 'OUTBOUND')),
    call_duration     INTEGER,
    call_outcome      TEXT CHECK (call_outcome IN ('ANSWERED', 'NO_ANSWER', 'VOICEMAIL', 'BUSY', 'CALLBACK_REQUESTED', 'WRONG_NUMBER')),
    call_phone_number TEXT,
    meeting_location  TEXT,
    meeting_type      TEXT CHECK (meeting_type IN ('IN_PERSON', 'VIDEO', 'PHONE')),
    email_to          TEXT,
    email_from        TEXT,
    system_event_type TEXT,
    system_event_data TEXT,
    is_active         INTEGER DEFAULT 1,
    created_at        TEXT DEFAULT (datetime('now')),
    updated_at        TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- updated_at Triggers
-- ============================================================================

CREATE TRIGGER IF NOT EXISTS conversations_updated_at AFTER UPDATE ON conversations
BEGIN
    UPDATE conversations SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS conversation_entries_updated_at AFTER UPDATE ON conversation_entries
BEGIN
    UPDATE conversation_entries SET updated_at = datetime('now') WHERE id = new.id;
END;

-- ============================================================================
-- Indexes
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_conversations_client_id  ON conversations(client_id);
CREATE INDEX IF NOT EXISTS idx_conversations_status     ON conversations(status);
CREATE INDEX IF NOT EXISTS idx_conversations_is_active  ON conversations(is_active);
CREATE INDEX IF NOT EXISTS idx_conversations_is_pinned  ON conversations(is_pinned);

CREATE INDEX IF NOT EXISTS idx_conv_entries_conversation_id ON conversation_entries(conversation_id);
CREATE INDEX IF NOT EXISTS idx_conv_entries_client_id       ON conversation_entries(client_id);
CREATE INDEX IF NOT EXISTS idx_conv_entries_entry_type      ON conversation_entries(entry_type);
CREATE INDEX IF NOT EXISTS idx_conv_entries_occurred_at     ON conversation_entries(occurred_at);
CREATE INDEX IF NOT EXISTS idx_conv_entries_follow_up_date  ON conversation_entries(follow_up_date);
CREATE INDEX IF NOT EXISTS idx_conv_entries_is_active       ON conversation_entries(is_active);
