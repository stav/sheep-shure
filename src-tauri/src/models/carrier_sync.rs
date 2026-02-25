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
    pub gender: Option<String>,
    pub middle_name: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub zip: Option<String>,
    pub county: Option<String>,
    pub mbi: Option<String>,
    pub application_date: Option<String>,
    pub member_record_locator: Option<String>,
    pub medicaid_id: Option<String>,
    pub provider_first_name: Option<String>,
    pub provider_last_name: Option<String>,
}

/// The result of comparing portal data against local enrollments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub carrier_name: String,
    pub portal_count: usize,
    pub local_count: usize,
    pub matched: usize,
    pub matched_members: Vec<SyncMatch>,
    pub disenrolled: Vec<SyncDisenrollment>,
    pub new_in_portal: Vec<PortalMember>,
}

/// A portal member that was matched to a local enrollment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMatch {
    pub client_name: String,
    pub client_id: String,
    pub portal_member: PortalMember,
}

/// A local enrollment that was not found in the portal (candidate for disenrollment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDisenrollment {
    pub client_name: String,
    pub client_id: String,
    pub enrollment_id: String,
    pub plan_name: Option<String>,
}

/// Result of importing portal members as new clients + enrollments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPortalResult {
    pub imported: usize,
    pub errors: Vec<String>,
}

/// Result of confirming disenrollment candidates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmDisenrollmentResult {
    pub disenrolled: usize,
    pub errors: Vec<String>,
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
