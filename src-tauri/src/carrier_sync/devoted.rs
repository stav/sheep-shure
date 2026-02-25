use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, COOKIE};
use serde::Deserialize;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct DevotedPortal;

const LOGIN_URL: &str = "https://agent.devoted.com/";
const GRAPHQL_ENDPOINT: &str = "https://agent.devoted.com/graphql/agents/";

const PERSISTED_QUERY_HASH: &str =
    "881c07f52080a6a6a04c653b03fa4520acfd30de90ab0ac6ca4caa161f6bbc95";

const PAGE_LIMIT: i64 = 100;

/// JS injected at document-start (before any page scripts) to intercept
/// CSRF tokens from the Devoted React app's own fetch/XHR calls.
const INIT_SCRIPT: &str = "";

/// JS that runs when the user clicks "Sync Now".
/// Reads the CSRF token captured by the init script, then fetches all members.
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        // Read orinoco config for the client version header
        const orinocoConfig = window.__orinoco_config || {};
        const clientVersion = orinocoConfig.VERSION || 'unknown';

        // Step 1: Fetch the CSRF token via the dedicated GraphQL query
        const csrfResp = await fetch('/graphql/agents/', {
            method: 'POST',
            headers: {
                'Accept': 'application/json; charset=utf-8',
                'Content-Type': 'application/json; charset=utf-8',
                'x-orinoco-portal': 'Agents',
                'x-orinoco-client-version': clientVersion,
                'x-csrf-token': 'undefined'
            },
            body: JSON.stringify({
                operationName: 'CSRFToken',
                variables: {},
                extensions: {
                    persistedQuery: {
                        version: 1,
                        sha256Hash: '0ba70438537351c55da05b9cec107834cf0e6e1126b9107bb382cba283d9dc5a'
                    }
                }
            })
        });
        if (!csrfResp.ok) {
            const body = await csrfResp.text().catch(() => '');
            throw new Error('CSRF fetch returned ' + csrfResp.status + ': ' + body.substring(0, 300));
        }
        const csrfJson = await csrfResp.json();
        const csrfToken = csrfJson.data && csrfJson.data.CSRFToken;
        if (!csrfToken) {
            throw new Error('CSRFToken query returned no token: ' + JSON.stringify(csrfJson));
        }

        // Step 2: Fetch members using the real CSRF token
        let allMembers = [];
        let page = 1;
        let hasNext = true;

        while (hasNext) {
            const resp = await fetch('/graphql/agents/', {
                method: 'POST',
                headers: {
                    'Accept': 'application/json; charset=utf-8',
                    'Content-Type': 'application/json; charset=utf-8',
                    'x-orinoco-portal': 'Agents',
                    'x-orinoco-client-version': clientVersion,
                    'x-csrf-token': csrfToken
                },
                body: JSON.stringify({
                    operationName: 'ListBookOfBusinessContacts',
                    variables: {
                        limit: 100,
                        page: page,
                        order_by: [
                            { by: 'LAST_NAME', direction: 'ASC' },
                            { by: 'FIRST_NAME', direction: 'ASC' },
                            { by: 'MIDDLE_NAME', direction: 'ASC' }
                        ],
                        filter_by: { member_id: { op: 'ISNOTNULL' } },
                        options: { allow_partial: true, cap_total_item_count: 10000 }
                    },
                    extensions: {
                        persistedQuery: {
                            version: 1,
                            sha256Hash: '881c07f52080a6a6a04c653b03fa4520acfd30de90ab0ac6ca4caa161f6bbc95'
                        }
                    }
                })
            });

            if (!resp.ok) {
                const body = await resp.text().catch(() => '');
                throw new Error('API returned ' + resp.status + ': ' + body.substring(0, 300));
            }

            const json = await resp.json();
            if (json.errors) throw new Error(json.errors.map(e => e.message).join('; '));

            const result = json.data.ListBookOfBusinessContacts;
            for (const c of result.items) {
                allMembers.push({
                    first_name: c.first_name || '',
                    last_name: c.last_name || '',
                    member_id: c.member_id || null,
                    dob: c.birth_date || null,
                    plan_name: c.current_pbp ? c.current_pbp.pbp_name : null,
                    effective_date: c.current_pbp ? c.current_pbp.start_date : null,
                    end_date: c.current_pbp ? c.current_pbp.end_date : null,
                    status: c.status || null,
                    policy_status: c.aor_policy_status || null,
                    state: c.state || null,
                    city: c.city || null,
                    phone: c.primary_phone || null,
                    email: c.email || null
                });
            }

            hasNext = result.page_info.has_next_page;
            page++;
        }

        window.location.href = 'http://sheeps-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(allMembers));
    } catch (e) {
        window.location.href = 'http://sheeps-sync.localhost/error?message=' +
            encodeURIComponent(e.toString());
    }
})();
"#;

// ── GraphQL response types (for the reqwest fallback) ───────────────────────

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct GraphQLData {
    ListBookOfBusinessContacts: BobResponse,
}

#[derive(Debug, Deserialize)]
struct BobResponse {
    items: Vec<BobContact>,
    page_info: BobPageInfo,
}

#[derive(Debug, Deserialize)]
struct BobContact {
    first_name: Option<String>,
    last_name: Option<String>,
    member_id: Option<String>,
    birth_date: Option<String>,
    status: Option<String>,
    aor_policy_status: Option<String>,
    current_pbp: Option<BobPbp>,
    state: Option<String>,
    city: Option<String>,
    primary_phone: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BobPbp {
    pbp_name: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BobPageInfo {
    has_next_page: bool,
    #[allow(dead_code)]
    total_item_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

// ── Trait implementation ────────────────────────────────────────────────────

#[async_trait]
impl CarrierPortal for DevotedPortal {
    fn carrier_id(&self) -> &str {
        "carrier-devoted"
    }

    fn carrier_name(&self) -> &str {
        "Devoted Health"
    }

    fn login_url(&self) -> &str {
        LOGIN_URL
    }

    fn init_script(&self) -> &str {
        INIT_SCRIPT
    }

    fn fetch_script(&self) -> &str {
        FETCH_SCRIPT
    }

    async fn fetch_members(&self, cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        let csrf_token = cookies
            .split(';')
            .filter_map(|pair| {
                let mut parts = pair.trim().splitn(2, '=');
                let key = parts.next()?.trim();
                let val = parts.next()?.trim();
                if key == "devoted-csrf" { Some(val.to_string()) } else { None }
            })
            .next()
            .ok_or_else(|| AppError::CarrierSync(
                "devoted-csrf cookie not found".into()
            ))?;

        let client = reqwest::Client::new();
        let mut all_members = Vec::new();
        let mut page: i64 = 1;

        loop {
            let body = serde_json::json!({
                "operationName": "ListBookOfBusinessContacts",
                "variables": {
                    "limit": PAGE_LIMIT,
                    "page": page,
                    "order_by": [
                        { "by": "LAST_NAME", "direction": "ASC" },
                        { "by": "FIRST_NAME", "direction": "ASC" },
                        { "by": "MIDDLE_NAME", "direction": "ASC" }
                    ],
                    "filter_by": {
                        "member_id": { "op": "ISNOTNULL" }
                    },
                    "options": {
                        "allow_partial": true,
                        "cap_total_item_count": 10000
                    }
                },
                "extensions": {
                    "persistedQuery": {
                        "version": 1,
                        "sha256Hash": PERSISTED_QUERY_HASH
                    }
                }
            });

            let resp = client
                .post(GRAPHQL_ENDPOINT)
                .header(CONTENT_TYPE, "application/json")
                .header(COOKIE, cookies)
                .header("x-csrf-token", &csrf_token)
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AppError::CarrierSync(format!(
                    "Devoted API returned {}: {}",
                    status, text
                )));
            }

            let gql_resp: GraphQLResponse = resp.json().await?;

            if let Some(errors) = gql_resp.errors {
                let msgs: Vec<String> = errors.into_iter().map(|e| e.message).collect();
                return Err(AppError::CarrierSync(format!(
                    "Devoted GraphQL errors: {}",
                    msgs.join("; ")
                )));
            }

            let data = gql_resp
                .data
                .ok_or_else(|| AppError::CarrierSync("No data in GraphQL response".into()))?;

            for contact in &data.ListBookOfBusinessContacts.items {
                let (plan_name, effective_date, end_date) = match &contact.current_pbp {
                    Some(pbp) => (pbp.pbp_name.clone(), pbp.start_date.clone(), pbp.end_date.clone()),
                    None => (None, None, None),
                };

                all_members.push(PortalMember {
                    first_name: contact.first_name.clone().unwrap_or_default(),
                    last_name: contact.last_name.clone().unwrap_or_default(),
                    member_id: contact.member_id.clone(),
                    dob: contact.birth_date.clone(),
                    plan_name,
                    effective_date,
                    end_date,
                    status: contact.status.clone(),
                    policy_status: contact.aor_policy_status.clone(),
                    state: contact.state.clone(),
                    city: contact.city.clone(),
                    phone: contact.primary_phone.clone(),
                    email: contact.email.clone(),
                });
            }

            if data.ListBookOfBusinessContacts.page_info.has_next_page {
                page += 1;
            } else {
                break;
            }
        }

        Ok(all_members)
    }
}
