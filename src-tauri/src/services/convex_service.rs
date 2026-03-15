use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::models::PortalMember;

/// Configuration for communicating with the Convex HTTP API.
#[derive(Debug, Clone)]
pub struct ConvexConfig {
    pub token: String,
    /// Base URL, e.g. "https://wandering-goose-882.convex.site"
    pub base_url: String,
}

impl ConvexConfig {
    /// Read `convex_token` and `convex_url` from `app_settings`.
    /// Returns `None` if either is missing or empty.
    pub fn from_settings(conn: &Connection) -> Option<Self> {
        let token: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'convex_token'",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let base_url: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'convex_url'",
                [],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        match (token, base_url) {
            (Some(t), Some(u)) if !t.is_empty() && !u.is_empty() => {
                Some(ConvexConfig { token: t, base_url: u })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncPushResult {
    pub inserted: usize,
    pub updated: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkPushResult {
    pub clients_inserted: usize,
    pub clients_updated: usize,
    pub enrollments_inserted: usize,
    pub enrollments_updated: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkPullResult {
    pub client_count: usize,
    pub enrollment_count: usize,
    pub carrier_count: usize,
}

/// Push portal-synced members to Convex via `POST /api/carrier-sync`.
/// `carrier` is the carrier `shortName` as configured in the web app.
/// Fire-and-forget: caller should spawn this in a background task.
pub async fn push_carrier_sync(
    config: &ConvexConfig,
    carrier: &str,
    members: &[PortalMember],
) -> Result<SyncPushResult, String> {
    let client = Client::new();
    let url = format!("{}/api/carrier-sync", config.base_url.trim_end_matches('/'));

    let members_json: Vec<serde_json::Value> = members
        .iter()
        .map(|m| {
            serde_json::json!({
                "firstName": m.first_name,
                "lastName": m.last_name,
                "middleName": m.middle_name,
                "dob": m.dob,
                "gender": m.gender,
                "phone": m.phone,
                "email": m.email,
                "addressLine1": m.address_line1,
                "addressLine2": m.address_line2,
                "city": m.city,
                "state": m.state,
                "zip": m.zip,
                "county": m.county,
                "mbi": m.mbi,
                "memberId": m.member_id,
                "planName": m.plan_name,
                "effectiveDate": m.effective_date,
                "endDate": m.end_date,
                "status": m.status,
                "applicationDate": m.application_date,
                "medicaidId": m.medicaid_id,
                "memberRecordLocator": m.member_record_locator,
                "providerFirstName": m.provider_first_name,
                "providerLastName": m.provider_last_name,
            })
        })
        .collect();

    let body = serde_json::json!({ "carrier": carrier, "members": members_json });

    let resp = client
        .post(&url)
        .bearer_auth(&config.token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Convex returned {}: {}", status, text));
    }

    resp.json::<SyncPushResult>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

/// Verify that the token and URL are valid by hitting `/api/sync/pull`.
pub async fn test_connection(config: &ConvexConfig) -> Result<bool, String> {
    let client = Client::new();
    let url = format!("{}/api/sync/pull", config.base_url.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .bearer_auth(&config.token)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    Ok(resp.status().is_success())
}

/// Push all clients and enrollments (as pre-serialized JSON arrays) to Convex.
pub async fn push_all(
    config: &ConvexConfig,
    clients: Vec<serde_json::Value>,
    enrollments: Vec<serde_json::Value>,
) -> Result<BulkPushResult, String> {
    let client = Client::new();
    let url = format!("{}/api/sync/push", config.base_url.trim_end_matches('/'));

    let body = serde_json::json!({ "clients": clients, "enrollments": enrollments });

    let resp = client
        .post(&url)
        .bearer_auth(&config.token)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Convex returned {}: {}", status, text));
    }

    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let cr = val.get("clients");
    let er = val.get("enrollments");

    Ok(BulkPushResult {
        clients_inserted: cr.and_then(|v| v.get("inserted")).and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        clients_updated: cr.and_then(|v| v.get("updated")).and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        enrollments_inserted: er.and_then(|v| v.get("inserted")).and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        enrollments_updated: er.and_then(|v| v.get("updated")).and_then(|v| v.as_u64()).unwrap_or(0) as usize,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CloudClient {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub dob: Option<String>,
    pub mbi: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address_line1: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

pub async fn pull_clients_full(config: &ConvexConfig) -> Result<Vec<CloudClient>, String> {
    let client = Client::new();
    let url = format!("{}/api/sync/pull", config.base_url.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .bearer_auth(&config.token)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Convex returned {}: {}", status, text));
    }

    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let clients = val
        .get("clients")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value::<CloudClient>(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(clients)
}

/// Pull all cloud data and return record counts.
pub async fn pull_all(config: &ConvexConfig) -> Result<BulkPullResult, String> {
    let client = Client::new();
    let url = format!("{}/api/sync/pull", config.base_url.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .bearer_auth(&config.token)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Convex returned {}: {}", status, text));
    }

    let val: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(BulkPullResult {
        client_count: val.get("clients").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        enrollment_count: val.get("enrollments").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
        carrier_count: val.get("carriers").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
    })
}
