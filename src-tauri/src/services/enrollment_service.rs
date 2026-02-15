use rusqlite::Connection;
use uuid::Uuid;
use crate::error::AppError;
use crate::models::{Enrollment, EnrollmentListItem, CreateEnrollmentInput, UpdateEnrollmentInput};
use crate::repositories::enrollment_repo;
use crate::services::conversation_service;

pub fn get_enrollments(conn: &Connection, client_id: Option<&str>) -> Result<Vec<EnrollmentListItem>, AppError> {
    enrollment_repo::get_enrollments(conn, client_id)
}

pub fn create_enrollment(conn: &Connection, input: &CreateEnrollmentInput) -> Result<Enrollment, AppError> {
    // Business rule: only one active/pending enrollment per plan category per client
    if let Some(ref plan_type_code) = input.plan_type_code {
        if enrollment_repo::has_active_enrollment_in_category(conn, &input.client_id, plan_type_code, None)? {
            return Err(AppError::Validation(
                "Client already has an active or pending enrollment in this plan category".to_string()
            ));
        }
    }

    let id = Uuid::new_v4().to_string();
    enrollment_repo::create_enrollment(conn, &id, input)?;

    let enrollment = enrollment_repo::get_enrollment(conn, &id)?;

    // Generate system event for enrollment creation
    let event_data = serde_json::json!({
        "enrollment_id": enrollment.id,
        "plan_name": enrollment.plan_name,
        "status": enrollment.status_code,
    })
    .to_string();
    let _ = conversation_service::create_system_event(
        conn,
        &input.client_id,
        "ENROLLMENT_CREATED",
        Some(&event_data),
    );

    Ok(enrollment)
}

pub fn update_enrollment(conn: &Connection, id: &str, input: &UpdateEnrollmentInput) -> Result<Enrollment, AppError> {
    enrollment_repo::update_enrollment(conn, id, input)?;

    let enrollment = enrollment_repo::get_enrollment(conn, id)?;

    // Generate system event for enrollment update
    let event_data = serde_json::json!({
        "enrollment_id": enrollment.id,
        "plan_name": enrollment.plan_name,
        "status": enrollment.status_code,
    })
    .to_string();
    let _ = conversation_service::create_system_event(
        conn,
        &enrollment.client_id,
        "ENROLLMENT_UPDATED",
        Some(&event_data),
    );

    Ok(enrollment)
}
