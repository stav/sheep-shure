use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: String,
    pub client_id: String,
    pub plan_id: Option<String>,
    pub carrier_id: Option<String>,
    pub plan_type_code: Option<String>,
    pub plan_name: Option<String>,
    pub contract_number: Option<String>,
    pub pbp_number: Option<String>,
    pub effective_date: Option<String>,
    pub termination_date: Option<String>,
    pub application_date: Option<String>,
    pub status_code: Option<String>,
    pub enrollment_period: Option<String>,
    pub disenrollment_reason: Option<String>,
    pub premium: Option<f64>,
    pub confirmation_number: Option<String>,
    pub enrollment_source: Option<String>,
    pub is_active: Option<i32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnrollmentInput {
    pub client_id: String,
    pub plan_id: Option<String>,
    pub carrier_id: Option<String>,
    pub plan_type_code: Option<String>,
    pub plan_name: Option<String>,
    pub contract_number: Option<String>,
    pub pbp_number: Option<String>,
    pub effective_date: Option<String>,
    pub termination_date: Option<String>,
    pub application_date: Option<String>,
    pub status_code: Option<String>,
    pub enrollment_period: Option<String>,
    pub disenrollment_reason: Option<String>,
    pub premium: Option<f64>,
    pub confirmation_number: Option<String>,
    pub enrollment_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEnrollmentInput {
    pub plan_id: Option<String>,
    pub carrier_id: Option<String>,
    pub plan_type_code: Option<String>,
    pub plan_name: Option<String>,
    pub contract_number: Option<String>,
    pub pbp_number: Option<String>,
    pub effective_date: Option<String>,
    pub termination_date: Option<String>,
    pub application_date: Option<String>,
    pub status_code: Option<String>,
    pub enrollment_period: Option<String>,
    pub disenrollment_reason: Option<String>,
    pub premium: Option<f64>,
    pub confirmation_number: Option<String>,
    pub enrollment_source: Option<String>,
    pub is_active: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentListItem {
    pub id: String,
    pub client_name: String,
    pub plan_name: Option<String>,
    pub carrier_name: Option<String>,
    pub plan_type: Option<String>,
    pub status: Option<String>,
    pub effective_date: Option<String>,
    pub termination_date: Option<String>,
}
