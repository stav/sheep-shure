use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;

use crate::error::AppError;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MatchTier {
    MbiExact,
    NameDobExact,
    NameDobFuzzy,
    NameOnlyUnique,
}

impl std::fmt::Display for MatchTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchTier::MbiExact => write!(f, "mbi_exact"),
            MatchTier::NameDobExact => write!(f, "name_dob_exact"),
            MatchTier::NameDobFuzzy => write!(f, "name_dob_fuzzy"),
            MatchTier::NameOnlyUnique => write!(f, "name_only_unique"),
        }
    }
}

pub struct ClientMatch {
    pub client_id: String,
    pub tier: MatchTier,
}

pub struct MatchOptions {
    pub allow_name_only_unique: bool,
    pub active_only: bool,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            allow_name_only_unique: false,
            active_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateCandidate {
    pub client_id: String,
    pub first_name: String,
    pub last_name: String,
    pub dob: Option<String>,
    pub mbi: Option<String>,
    pub match_tier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroupClient {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub dob: Option<String>,
    pub mbi: Option<String>,
    pub is_suggested_keeper: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub clients: Vec<DuplicateGroupClient>,
    pub match_tier: String,
}

// ── Normalization helpers ────────────────────────────────────────────────────

/// Normalize a date string from various formats into YYYY-MM-DD.
pub fn normalize_date(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Try ISO datetime: 2025-07-01T00:00:00 or with fractional seconds
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }
    // Already ISO date
    if let Ok(d) = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // US format: 07/06/1960
    if let Ok(d) = chrono::NaiveDate::parse_from_str(raw, "%m/%d/%Y") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    // LeadsMaster / portal: "Sep 25 1958 12:00AM"
    let compressed = raw.replace("  ", " ");
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&compressed, "%b %d %Y %I:%M%p") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&compressed, "%b %d %Y %I:%M%P") {
        return Some(dt.format("%Y-%m-%d").to_string());
    }

    None
}

/// Normalize MBI: strip dashes/spaces, uppercase, validate 11 alphanumeric chars.
pub fn normalize_mbi(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if cleaned.len() == 11 {
        Some(cleaned)
    } else {
        None
    }
}

/// Normalize phone: strip non-digits, validate 10 digits.
pub fn normalize_phone(raw: &str) -> Option<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 11 && digits.starts_with('1') {
        return Some(digits[1..].to_string());
    }
    if digits.len() == 10 {
        Some(digits)
    } else {
        None
    }
}

/// Strip a trailing single-letter middle initial from a first name, lowercase.
/// e.g. "Brian L" → "brian", "Kenneth E" → "kenneth"
pub fn normalize_first_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let trimmed = lower.trim();
    // If the name ends with " X" where X is a single letter, strip it
    if trimmed.len() >= 3 {
        let bytes = trimmed.as_bytes();
        let len = bytes.len();
        if bytes[len - 2] == b' ' && bytes[len - 1].is_ascii_alphabetic() {
            return trimmed[..len - 2].to_string();
        }
    }
    trimmed.to_string()
}

/// Fuzzy first-name comparison.
/// Normalizes both (strip middle initial, lowercase) then checks exact, prefix, or edit distance.
pub fn fuzzy_first_name(a: &str, b: &str) -> bool {
    let na = normalize_first_name(a);
    let nb = normalize_first_name(b);

    if na == nb {
        return true;
    }

    // Prefix match (e.g. "rob" vs "robert")
    if na.len() >= 3 && nb.len() >= 3 && (na.starts_with(&*nb) || nb.starts_with(&*na)) {
        return true;
    }

    levenshtein(&na, &nb) <= 2
}

/// Simple Levenshtein distance (fine for short first names).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());

    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

// ── Canonical matching ──────────────────────────────────────────────────────

/// Find a single best client match using a tiered cascade:
/// 1. MBI exact match (active only)
/// 2. Name + DOB exact (case-insensitive last, normalized first, normalized DOB)
/// 3. Name + DOB fuzzy (case-insensitive last, fuzzy first, normalized DOB)
/// 4. Name-only unique (opt-in, exactly 1 case-insensitive match)
pub fn find_client_match(
    conn: &Connection,
    mbi: Option<&str>,
    first_name: &str,
    last_name: &str,
    dob: Option<&str>,
    opts: &MatchOptions,
) -> Option<ClientMatch> {
    let active_clause = if opts.active_only { " AND is_active = 1" } else { "" };

    // Tier 1: MBI exact
    if let Some(mbi_val) = mbi {
        let normalized = normalize_mbi(mbi_val).unwrap_or_else(|| mbi_val.to_string());
        if !normalized.is_empty() {
            let sql = format!("SELECT id FROM clients WHERE mbi = ?1{}", active_clause);
            if let Ok(id) = conn.query_row(&sql, params![normalized], |row| row.get::<_, String>(0)) {
                return Some(ClientMatch { client_id: id, tier: MatchTier::MbiExact });
            }
        }
    }

    // Normalize the incoming DOB for comparison
    let dob_normalized = dob.and_then(normalize_date);

    // Tiers 2 & 3: Name + DOB (need DOB)
    if let Some(ref dob_norm) = dob_normalized {
        let sql = format!(
            "SELECT id, first_name, dob FROM clients WHERE LOWER(last_name) = LOWER(?1){}",
            active_clause
        );
        if let Ok(mut stmt) = conn.prepare(&sql) {
            let rows: Vec<(String, String, Option<String>)> = stmt
                .query_map(params![last_name], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .ok()
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();

            // Tier 2: exact first name + DOB
            for (id, db_first, db_dob) in &rows {
                let db_dob_norm = db_dob.as_deref().and_then(normalize_date);
                if db_dob_norm.as_deref() != Some(dob_norm.as_str()) {
                    continue;
                }
                if normalize_first_name(db_first) == normalize_first_name(first_name) {
                    return Some(ClientMatch { client_id: id.clone(), tier: MatchTier::NameDobExact });
                }
            }

            // Tier 3: fuzzy first name + DOB
            for (id, db_first, db_dob) in &rows {
                let db_dob_norm = db_dob.as_deref().and_then(normalize_date);
                if db_dob_norm.as_deref() != Some(dob_norm.as_str()) {
                    continue;
                }
                if fuzzy_first_name(db_first, first_name) {
                    return Some(ClientMatch { client_id: id.clone(), tier: MatchTier::NameDobFuzzy });
                }
            }
        }
    }

    // Tier 4: Name-only unique (opt-in)
    if opts.allow_name_only_unique {
        let sql = format!(
            "SELECT id FROM clients WHERE LOWER(first_name) = LOWER(?1) AND LOWER(last_name) = LOWER(?2){}",
            active_clause
        );
        if let Ok(mut stmt) = conn.prepare(&sql) {
            let ids: Vec<String> = stmt
                .query_map(params![first_name, last_name], |row| row.get(0))
                .ok()
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();
            if ids.len() == 1 {
                return Some(ClientMatch {
                    client_id: ids.into_iter().next().unwrap(),
                    tier: MatchTier::NameOnlyUnique,
                });
            }
        }
    }

    None
}

// ── Single-client duplicate check (for create form) ─────────────────────────

/// Check for potential duplicate clients across tiers 1-3.
/// Returns ALL matches (not just the first), for the UI to display.
pub fn check_for_duplicates(
    conn: &Connection,
    first_name: &str,
    last_name: &str,
    dob: Option<&str>,
    mbi: Option<&str>,
) -> Vec<DuplicateCandidate> {
    let mut candidates = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Tier 1: MBI exact
    if let Some(mbi_val) = mbi {
        let normalized = normalize_mbi(mbi_val).unwrap_or_else(|| mbi_val.to_string());
        if !normalized.is_empty() {
            if let Ok(mut stmt) = conn.prepare(
                "SELECT id, first_name, last_name, dob, mbi FROM clients WHERE mbi = ?1 "
            ) {
                if let Ok(rows) = stmt.query_map(params![normalized], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                }) {
                    for r in rows.flatten() {
                        if seen_ids.insert(r.0.clone()) {
                            candidates.push(DuplicateCandidate {
                                client_id: r.0,
                                first_name: r.1,
                                last_name: r.2,
                                dob: r.3,
                                mbi: r.4,
                                match_tier: MatchTier::MbiExact.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    let dob_normalized = dob.and_then(normalize_date);

    // Tiers 2 & 3: Name + DOB
    if let Some(ref dob_norm) = dob_normalized {
        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, first_name, last_name, dob, mbi FROM clients WHERE LOWER(last_name) = LOWER(?1)"
        ) {
            let rows: Vec<(String, String, String, Option<String>, Option<String>)> = stmt
                .query_map(params![last_name], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                })
                .ok()
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();

            for (id, db_first, db_last, db_dob, db_mbi) in rows {
                if seen_ids.contains(&id) {
                    continue;
                }
                let db_dob_norm = db_dob.as_deref().and_then(normalize_date);
                if db_dob_norm.as_deref() != Some(dob_norm.as_str()) {
                    continue;
                }

                if normalize_first_name(&db_first) == normalize_first_name(first_name) {
                    seen_ids.insert(id.clone());
                    candidates.push(DuplicateCandidate {
                        client_id: id,
                        first_name: db_first,
                        last_name: db_last,
                        dob: db_dob,
                        mbi: db_mbi,
                        match_tier: MatchTier::NameDobExact.to_string(),
                    });
                } else if fuzzy_first_name(&db_first, first_name) {
                    seen_ids.insert(id.clone());
                    candidates.push(DuplicateCandidate {
                        client_id: id,
                        first_name: db_first,
                        last_name: db_last,
                        dob: db_dob,
                        mbi: db_mbi,
                        match_tier: MatchTier::NameDobFuzzy.to_string(),
                    });
                }
            }
        }
    }

    candidates
}

// ── Batch duplicate scan ────────────────────────────────────────────────────

struct ClientRow {
    id: String,
    first_name: String,
    last_name: String,
    dob: Option<String>,
    mbi: Option<String>,
    non_null_count: i32,
    created_at: String,
}

/// Scan all active clients for duplicate groups.
/// Groups by normalized MBI and by (last_name, DOB) with fuzzy first-name matching.
pub fn find_duplicate_clients(conn: &Connection) -> Result<Vec<DuplicateGroup>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, first_name, last_name, dob, mbi, created_at,
                (CASE WHEN middle_name IS NOT NULL AND middle_name != '' THEN 1 ELSE 0 END
               + CASE WHEN dob IS NOT NULL AND dob != '' THEN 1 ELSE 0 END
               + CASE WHEN phone IS NOT NULL AND phone != '' THEN 1 ELSE 0 END
               + CASE WHEN email IS NOT NULL AND email != '' THEN 1 ELSE 0 END
               + CASE WHEN mbi IS NOT NULL AND mbi != '' THEN 1 ELSE 0 END
               + CASE WHEN address_line1 IS NOT NULL AND address_line1 != '' THEN 1 ELSE 0 END) as non_null_count
         FROM clients"
    )?;

    let clients: Vec<ClientRow> = stmt
        .query_map([], |row| {
            Ok(ClientRow {
                id: row.get(0)?,
                first_name: row.get(1)?,
                last_name: row.get(2)?,
                dob: row.get(3)?,
                mbi: row.get(4)?,
                created_at: row.get(5)?,
                non_null_count: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let n = clients.len();

    // Union-Find
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }

    // Index by normalized MBI
    let mut mbi_index: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, c) in clients.iter().enumerate() {
        if let Some(ref mbi) = c.mbi {
            if let Some(norm) = normalize_mbi(mbi) {
                mbi_index.entry(norm).or_default().push(i);
            }
        }
    }

    // Union MBI groups
    for indices in mbi_index.values() {
        if indices.len() > 1 {
            for w in indices.windows(2) {
                union(&mut parent, w[0], w[1]);
            }
        }
    }

    // Index by (lowercase last_name, normalized DOB)
    let mut name_dob_index: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (i, c) in clients.iter().enumerate() {
        if let Some(ref dob) = c.dob {
            if let Some(dob_norm) = normalize_date(dob) {
                let key = (c.last_name.to_ascii_lowercase(), dob_norm);
                name_dob_index.entry(key).or_default().push(i);
            }
        }
    }

    // Within each (last_name, DOB) bucket, fuzzy-compare first names
    for indices in name_dob_index.values() {
        if indices.len() > 1 {
            for i in 0..indices.len() {
                for j in (i + 1)..indices.len() {
                    let a = indices[i];
                    let b = indices[j];
                    if fuzzy_first_name(&clients[a].first_name, &clients[b].first_name) {
                        union(&mut parent, a, b);
                    }
                }
            }
        }
    }

    // Collect groups
    let mut groups_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups_map.entry(root).or_default().push(i);
    }

    let mut result = Vec::new();
    for indices in groups_map.values() {
        if indices.len() < 2 {
            continue;
        }

        // Determine match tier for this group
        let has_mbi_match = {
            let mbis: Vec<Option<String>> = indices.iter()
                .map(|&i| clients[i].mbi.as_deref().and_then(normalize_mbi))
                .collect();
            mbis.iter().any(|a| {
                a.is_some() && mbis.iter().filter(|b| *b == a).count() > 1
            })
        };
        let tier = if has_mbi_match { "mbi_exact" } else { "name_dob_fuzzy" };

        // Pick suggested keeper: most non-null fields, then earliest created_at
        let keeper_idx = *indices.iter().max_by(|&&a, &&b| {
            clients[a].non_null_count.cmp(&clients[b].non_null_count)
                .then(clients[b].created_at.cmp(&clients[a].created_at))
        }).unwrap();

        let group_clients: Vec<DuplicateGroupClient> = indices.iter().map(|&i| {
            let c = &clients[i];
            DuplicateGroupClient {
                id: c.id.clone(),
                first_name: c.first_name.clone(),
                last_name: c.last_name.clone(),
                dob: c.dob.clone(),
                mbi: c.mbi.clone(),
                is_suggested_keeper: i == keeper_idx,
            }
        }).collect();

        result.push(DuplicateGroup {
            clients: group_clients,
            match_tier: tier.to_string(),
        });
    }

    Ok(result)
}
