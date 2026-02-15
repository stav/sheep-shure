-- V001: Initial schema for SHEEPS Medicare Book of Business Manager

-- ============================================================================
-- Reference / Lookup Tables
-- ============================================================================

CREATE TABLE IF NOT EXISTS states (
    code TEXT PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS carriers (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    short_name TEXT,
    is_active  INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plan_types (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    category    TEXT CHECK (category IN ('ADVANTAGE', 'PRESCRIPTION', 'SUPPLEMENT', 'OTHER')),
    is_active   INTEGER DEFAULT 1
);

CREATE TABLE IF NOT EXISTS enrollment_statuses (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    is_terminal INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS enrollment_periods (
    code        TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    start_month INTEGER,
    start_day   INTEGER,
    end_month   INTEGER,
    end_day     INTEGER
);

-- ============================================================================
-- Core Tables
-- ============================================================================

CREATE TABLE IF NOT EXISTS clients (
    id                      TEXT PRIMARY KEY,
    first_name              TEXT NOT NULL,
    last_name               TEXT NOT NULL,
    middle_name             TEXT,
    dob                     TEXT,
    gender                  TEXT,
    phone                   TEXT,
    phone2                  TEXT,
    email                   TEXT,
    address_line1           TEXT,
    address_line2           TEXT,
    city                    TEXT,
    state                   TEXT,
    zip                     TEXT,
    county                  TEXT,
    mbi                     TEXT,
    part_a_date             TEXT,
    part_b_date             TEXT,
    orec                    TEXT,
    esrd_status             INTEGER DEFAULT 0,
    is_dual_eligible        INTEGER DEFAULT 0,
    dual_status_code        TEXT,
    lis_level               TEXT,
    medicaid_id             TEXT,
    lead_source             TEXT,
    original_effective_date TEXT,
    is_active               INTEGER DEFAULT 1,
    tags                    TEXT,
    notes                   TEXT,
    created_at              TEXT DEFAULT (datetime('now')),
    updated_at              TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plans (
    id              TEXT PRIMARY KEY,
    carrier_id      TEXT NOT NULL REFERENCES carriers(id),
    plan_type_code  TEXT NOT NULL REFERENCES plan_types(code),
    plan_name       TEXT NOT NULL,
    contract_number TEXT,
    pbp_number      TEXT,
    segment_id      TEXT DEFAULT '000',
    plan_year       INTEGER,
    state           TEXT,
    county_fips     TEXT,
    premium         REAL,
    is_active       INTEGER DEFAULT 1,
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now')),
    UNIQUE(contract_number, pbp_number, segment_id, plan_year)
);

CREATE TABLE IF NOT EXISTS enrollments (
    id                   TEXT PRIMARY KEY,
    client_id            TEXT NOT NULL REFERENCES clients(id),
    plan_id              TEXT REFERENCES plans(id),
    carrier_id           TEXT REFERENCES carriers(id),
    plan_type_code       TEXT,
    plan_name            TEXT,
    contract_number      TEXT,
    pbp_number           TEXT,
    effective_date       TEXT,
    termination_date     TEXT,
    application_date     TEXT,
    status_code          TEXT DEFAULT 'PENDING' REFERENCES enrollment_statuses(code),
    enrollment_period    TEXT REFERENCES enrollment_periods(code),
    disenrollment_reason TEXT,
    premium              REAL,
    confirmation_number  TEXT,
    enrollment_source    TEXT,
    is_active            INTEGER DEFAULT 1,
    created_at           TEXT DEFAULT (datetime('now')),
    updated_at           TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS notes (
    id         TEXT PRIMARY KEY,
    client_id  TEXT NOT NULL REFERENCES clients(id),
    note_type  TEXT DEFAULT 'GENERAL' CHECK (note_type IN ('GENERAL', 'CALL', 'EMAIL', 'MEETING', 'SOA', 'SYSTEM')),
    subject    TEXT,
    body       TEXT,
    is_pinned  INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS import_logs (
    id             TEXT PRIMARY KEY,
    filename       TEXT NOT NULL,
    file_type      TEXT,
    carrier_id     TEXT REFERENCES carriers(id),
    total_rows     INTEGER DEFAULT 0,
    inserted_rows  INTEGER DEFAULT 0,
    updated_rows   INTEGER DEFAULT 0,
    skipped_rows   INTEGER DEFAULT 0,
    error_rows     INTEGER DEFAULT 0,
    column_mapping TEXT,
    error_details  TEXT,
    status         TEXT DEFAULT 'PENDING',
    created_at     TEXT DEFAULT (datetime('now')),
    updated_at     TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_profile (
    id            TEXT PRIMARY KEY,
    first_name    TEXT,
    last_name     TEXT,
    email         TEXT,
    phone         TEXT,
    npn           TEXT,
    agency_name   TEXT,
    license_state TEXT,
    created_at    TEXT DEFAULT (datetime('now')),
    updated_at    TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_carrier_numbers (
    id               TEXT PRIMARY KEY,
    agent_id         TEXT NOT NULL REFERENCES agent_profile(id),
    carrier_id       TEXT NOT NULL REFERENCES carriers(id),
    writing_number   TEXT NOT NULL,
    state            TEXT,
    effective_date   TEXT,
    termination_date TEXT,
    created_at       TEXT DEFAULT (datetime('now')),
    updated_at       TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS app_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- ============================================================================
-- Full-Text Search
-- ============================================================================

CREATE VIRTUAL TABLE IF NOT EXISTS clients_fts USING fts5(
    first_name,
    last_name,
    mbi,
    phone,
    email,
    city,
    zip,
    content=clients,
    content_rowid=rowid
);

-- FTS sync triggers

CREATE TRIGGER IF NOT EXISTS clients_fts_ai AFTER INSERT ON clients BEGIN
    INSERT INTO clients_fts(rowid, first_name, last_name, mbi, phone, email, city, zip)
    VALUES (new.rowid, new.first_name, new.last_name, new.mbi, new.phone, new.email, new.city, new.zip);
END;

CREATE TRIGGER IF NOT EXISTS clients_fts_ad AFTER DELETE ON clients BEGIN
    INSERT INTO clients_fts(clients_fts, rowid, first_name, last_name, mbi, phone, email, city, zip)
    VALUES ('delete', old.rowid, old.first_name, old.last_name, old.mbi, old.phone, old.email, old.city, old.zip);
END;

CREATE TRIGGER IF NOT EXISTS clients_fts_au AFTER UPDATE ON clients BEGIN
    INSERT INTO clients_fts(clients_fts, rowid, first_name, last_name, mbi, phone, email, city, zip)
    VALUES ('delete', old.rowid, old.first_name, old.last_name, old.mbi, old.phone, old.email, old.city, old.zip);
    INSERT INTO clients_fts(rowid, first_name, last_name, mbi, phone, email, city, zip)
    VALUES (new.rowid, new.first_name, new.last_name, new.mbi, new.phone, new.email, new.city, new.zip);
END;

-- ============================================================================
-- updated_at Triggers
-- ============================================================================

CREATE TRIGGER IF NOT EXISTS clients_updated_at AFTER UPDATE ON clients
BEGIN
    UPDATE clients SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS carriers_updated_at AFTER UPDATE ON carriers
BEGIN
    UPDATE carriers SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS plans_updated_at AFTER UPDATE ON plans
BEGIN
    UPDATE plans SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS enrollments_updated_at AFTER UPDATE ON enrollments
BEGIN
    UPDATE enrollments SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS notes_updated_at AFTER UPDATE ON notes
BEGIN
    UPDATE notes SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS import_logs_updated_at AFTER UPDATE ON import_logs
BEGIN
    UPDATE import_logs SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS agent_profile_updated_at AFTER UPDATE ON agent_profile
BEGIN
    UPDATE agent_profile SET updated_at = datetime('now') WHERE id = new.id;
END;

CREATE TRIGGER IF NOT EXISTS agent_carrier_numbers_updated_at AFTER UPDATE ON agent_carrier_numbers
BEGIN
    UPDATE agent_carrier_numbers SET updated_at = datetime('now') WHERE id = new.id;
END;

-- ============================================================================
-- Indexes
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_clients_name            ON clients(last_name, first_name);
CREATE INDEX IF NOT EXISTS idx_clients_mbi             ON clients(mbi);
CREATE INDEX IF NOT EXISTS idx_clients_zip             ON clients(zip);
CREATE INDEX IF NOT EXISTS idx_clients_state           ON clients(state);
CREATE INDEX IF NOT EXISTS idx_clients_dob             ON clients(dob);
CREATE INDEX IF NOT EXISTS idx_clients_is_active       ON clients(is_active);
CREATE INDEX IF NOT EXISTS idx_clients_is_dual         ON clients(is_dual_eligible);

CREATE INDEX IF NOT EXISTS idx_enrollments_client_id      ON enrollments(client_id);
CREATE INDEX IF NOT EXISTS idx_enrollments_plan_id        ON enrollments(plan_id);
CREATE INDEX IF NOT EXISTS idx_enrollments_carrier_id     ON enrollments(carrier_id);
CREATE INDEX IF NOT EXISTS idx_enrollments_status_code    ON enrollments(status_code);
CREATE INDEX IF NOT EXISTS idx_enrollments_effective_date ON enrollments(effective_date);
CREATE INDEX IF NOT EXISTS idx_enrollments_is_active      ON enrollments(is_active);
