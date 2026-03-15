use serde::Serialize;
use tauri::State;

use crate::db::DbState;
use crate::services::convex_service::{self, BulkPullResult, BulkPushResult, ConvexConfig};

#[derive(Serialize)]
pub struct ConvexConnectionStatus {
    pub connected: bool,
    pub error: Option<String>,
}

/// Test that the configured token and URL can reach Convex.
#[tauri::command]
pub async fn test_convex_connection(
    state: State<'_, DbState>,
) -> Result<ConvexConnectionStatus, String> {
    let config = state
        .with_conn(|conn| Ok(ConvexConfig::from_settings(conn)))
        .map_err(|e| e.to_string())?;

    let Some(config) = config else {
        return Ok(ConvexConnectionStatus {
            connected: false,
            error: Some("Convex token or URL not configured".to_string()),
        });
    };

    match convex_service::test_connection(&config).await {
        Ok(true) => Ok(ConvexConnectionStatus { connected: true, error: None }),
        Ok(false) => Ok(ConvexConnectionStatus {
            connected: false,
            error: Some("Authentication failed — check your token".to_string()),
        }),
        Err(e) => Ok(ConvexConnectionStatus { connected: false, error: Some(e) }),
    }
}

/// Push all local clients and enrollments to Convex.
#[tauri::command]
pub async fn push_all_to_convex(state: State<'_, DbState>) -> Result<BulkPushResult, String> {
    let (config, clients, enrollments) = state
        .with_conn(|conn| {
            let config = ConvexConfig::from_settings(conn);

            // Serialize active clients
            let mut stmt = conn
                .prepare(
                    "SELECT first_name, last_name, middle_name, dob, gender,
                            phone, phone2, email,
                            address_line1, address_line2, city, state, zip, county,
                            mbi, part_a_date, part_b_date, orec,
                            is_dual_eligible, dual_status_code, lis_level,
                            medicaid_id, lead_source, member_record_locator,
                            notes, is_active
                     FROM clients WHERE is_active = 1",
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let clients: Vec<serde_json::Value> = stmt
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "firstName": row.get::<_, Option<String>>(0)?,
                        "lastName": row.get::<_, Option<String>>(1)?,
                        "middleName": row.get::<_, Option<String>>(2)?,
                        "dob": row.get::<_, Option<String>>(3)?,
                        "gender": row.get::<_, Option<String>>(4)?,
                        "phone": row.get::<_, Option<String>>(5)?,
                        "phone2": row.get::<_, Option<String>>(6)?,
                        "email": row.get::<_, Option<String>>(7)?,
                        "addressLine1": row.get::<_, Option<String>>(8)?,
                        "addressLine2": row.get::<_, Option<String>>(9)?,
                        "city": row.get::<_, Option<String>>(10)?,
                        "state": row.get::<_, Option<String>>(11)?,
                        "zip": row.get::<_, Option<String>>(12)?,
                        "county": row.get::<_, Option<String>>(13)?,
                        "mbi": row.get::<_, Option<String>>(14)?,
                        "partADate": row.get::<_, Option<String>>(15)?,
                        "partBDate": row.get::<_, Option<String>>(16)?,
                        "orec": row.get::<_, Option<String>>(17)?,
                        "isDualEligible": row.get::<_, Option<bool>>(18)?,
                        "dualStatusCode": row.get::<_, Option<String>>(19)?,
                        "lisLevel": row.get::<_, Option<String>>(20)?,
                        "medicaidId": row.get::<_, Option<String>>(21)?,
                        "leadSource": row.get::<_, Option<String>>(22)?,
                        "memberRecordLocator": row.get::<_, Option<String>>(23)?,
                        "notes": row.get::<_, Option<String>>(24)?,
                        "isActive": row.get::<_, Option<bool>>(25)?,
                    }))
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            // Serialize enrollments joined with client name/MBI and carrier shortName
            let mut stmt2 = conn
                .prepare(
                    "SELECT c.first_name, c.last_name, c.mbi,
                            ca.short_name,
                            e.plan_name, e.plan_type_code, e.contract_number, e.pbp_number,
                            e.effective_date, e.termination_date, e.application_date,
                            e.status_code, e.enrollment_period, e.confirmation_number,
                            e.enrollment_source, e.is_active
                     FROM enrollments e
                     JOIN clients c ON e.client_id = c.id
                     LEFT JOIN carriers ca ON e.carrier_id = ca.id
                     WHERE c.is_active = 1",
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

            let enrollments: Vec<serde_json::Value> = stmt2
                .query_map([], |row| {
                    Ok(serde_json::json!({
                        "clientFirstName": row.get::<_, Option<String>>(0)?,
                        "clientLastName": row.get::<_, Option<String>>(1)?,
                        "clientMbi": row.get::<_, Option<String>>(2)?,
                        "carrierShortName": row.get::<_, Option<String>>(3)?,
                        "planName": row.get::<_, Option<String>>(4)?,
                        "planTypeCode": row.get::<_, Option<String>>(5)?,
                        "contractNumber": row.get::<_, Option<String>>(6)?,
                        "pbpNumber": row.get::<_, Option<String>>(7)?,
                        "effectiveDate": row.get::<_, Option<String>>(8)?,
                        "terminationDate": row.get::<_, Option<String>>(9)?,
                        "applicationDate": row.get::<_, Option<String>>(10)?,
                        "statusCode": row.get::<_, Option<String>>(11)
                            .map(|s| s.unwrap_or_else(|| "PENDING".to_string()))?,
                        "enrollmentPeriod": row.get::<_, Option<String>>(12)?,
                        "confirmationNumber": row.get::<_, Option<String>>(13)?,
                        "enrollmentSource": row.get::<_, Option<String>>(14)?,
                        "isActive": row.get::<_, Option<bool>>(15)?,
                    }))
                })
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

            Ok((config, clients, enrollments))
        })
        .map_err(|e| e.to_string())?;

    let Some(config) = config else {
        return Err(
            "Convex not configured. Set token and URL in Settings → Compass Cloud.".to_string(),
        );
    };

    convex_service::push_all(&config, clients, enrollments).await
}

/// Pull all cloud data and return counts (does not write to local DB).
#[tauri::command]
pub async fn pull_from_convex(state: State<'_, DbState>) -> Result<BulkPullResult, String> {
    let config = state
        .with_conn(|conn| Ok(ConvexConfig::from_settings(conn)))
        .map_err(|e| e.to_string())?;

    let Some(config) = config else {
        return Err(
            "Convex not configured. Set token and URL in Settings → Compass Cloud.".to_string(),
        );
    };

    convex_service::pull_all(&config).await
}
