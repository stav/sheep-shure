use serde::{Deserialize, Serialize};

/// A member record as returned by a carrier portal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalMember {
    pub first_name: String,
    pub last_name: String,
    pub member_id: Option<String>,
    pub dob: Option<String>,
    pub plan_name: Option<String>,
    pub effective_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
    pub policy_status: Option<String>,
    pub state: Option<String>,
    pub city: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// The result of comparing portal data against local enrollments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub carrier_name: String,
    pub portal_count: usize,
    pub local_count: usize,
    pub matched: usize,
    pub disenrolled: Vec<SyncDisenrollment>,
    pub new_in_portal: Vec<PortalMember>,
}

/// A local enrollment that was not found in the portal (candidate for disenrollment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDisenrollment {
    pub client_name: String,
    pub client_id: String,
    pub enrollment_id: String,
    pub plan_name: Option<String>,
}

/// Summary log entry for a completed sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLogEntry {
    pub id: String,
    pub carrier_id: String,
    pub carrier_name: Option<String>,
    pub synced_at: String,
    pub portal_count: i64,
    pub matched: i64,
    pub disenrolled: i64,
    pub new_found: i64,
    pub status: String,
}
