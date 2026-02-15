use tauri::State;
use crate::db::DbState;
use crate::models::{CreateEnrollmentInput, Enrollment, EnrollmentListItem, UpdateEnrollmentInput};
use crate::services::enrollment_service;

#[tauri::command]
pub fn get_enrollments(
    client_id: Option<String>,
    state: State<'_, DbState>,
) -> Result<Vec<EnrollmentListItem>, String> {
    state.with_conn(|conn| {
        enrollment_service::get_enrollments(conn, client_id.as_deref())
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_enrollment(input: CreateEnrollmentInput, state: State<'_, DbState>) -> Result<Enrollment, String> {
    state.with_conn(|conn| {
        enrollment_service::create_enrollment(conn, &input)
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_enrollment(id: String, input: UpdateEnrollmentInput, state: State<'_, DbState>) -> Result<Enrollment, String> {
    state.with_conn(|conn| {
        enrollment_service::update_enrollment(conn, &id, &input)
    }).map_err(|e| e.to_string())
}
