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

const DETAIL_QUERY_HASH: &str =
    "bbc3ff06615745839c96d4823ef9b60f6171948d0bb8a5f31176ee618aca0c56";

const PAGE_LIMIT: i64 = 100;

/// Init script: defines the fetch function and auto-calls it silently on load.
/// If the user isn't logged in yet, the CSRF/GraphQL calls fail and nothing happens.
const INIT_SCRIPT: &str = r#"
window.addEventListener('load', () => {
    if (window.__compassBobFetched) return;
    window.__compassFetchDevoted(true);
});

window.__compassFetchDevoted = async function(silent) {
    try {
        if (window.__compassBobFetched) return;

        const orinocoConfig = window.__orinoco_config || {};
        const clientVersion = orinocoConfig.VERSION || 'unknown';

        // Step 1: Fetch CSRF token
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
            if (silent) return;
            const body = await csrfResp.text().catch(() => '');
            throw new Error('CSRF fetch returned ' + csrfResp.status + ': ' + body.substring(0, 300));
        }
        const csrfJson = await csrfResp.json();
        const csrfToken = csrfJson.data && csrfJson.data.CSRFToken;
        if (!csrfToken) {
            if (silent) return;
            throw new Error('CSRFToken query returned no token: ' + JSON.stringify(csrfJson));
        }

        // Step 2: Fetch members
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
                if (silent) return;
                const body = await resp.text().catch(() => '');
                throw new Error('API returned ' + resp.status + ': ' + body.substring(0, 300));
            }

            const json = await resp.json();
            if (json.errors) {
                if (silent) return;
                throw new Error(json.errors.map(e => e.message).join('; '));
            }

            const result = json.data.ListBookOfBusinessContacts;
            for (const c of result.items) {
                allMembers.push({
                    _id: c.id || null,
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
                    email: c.email || null,
                    gender: null,
                    middle_name: null,
                    address_line1: null,
                    address_line2: null,
                    zip: null,
                    county: null,
                    mbi: null,
                    application_date: null,
                    member_record_locator: null,
                    medicaid_id: null,
                    provider_first_name: null,
                    provider_last_name: null
                });
            }

            hasNext = result.page_info.has_next_page;
            page++;
        }

        // Fetch detail for each member (parallel, batched by 5)
        const BATCH = 5;
        for (let i = 0; i < allMembers.length; i += BATCH) {
            const batch = allMembers.slice(i, i + BATCH);
            const details = await Promise.all(batch.map(m => {
                if (!m._id) return Promise.resolve(null);
                return fetch('/graphql/agents/', {
                    method: 'POST',
                    headers: {
                        'Accept': 'application/json; charset=utf-8',
                        'Content-Type': 'application/json; charset=utf-8',
                        'x-orinoco-portal': 'Agents',
                        'x-orinoco-client-version': clientVersion,
                        'x-csrf-token': csrfToken
                    },
                    body: JSON.stringify({
                        operationName: 'GetBookOfBusinessContact',
                        variables: { id: m._id },
                        extensions: {
                            persistedQuery: {
                                version: 1,
                                sha256Hash: 'bbc3ff06615745839c96d4823ef9b60f6171948d0bb8a5f31176ee618aca0c56'
                            }
                        }
                    })
                }).then(r => r.json()).catch(() => null);
            }));
            details.forEach((d, j) => {
                if (!d) return;
                const c = d.data && d.data.GetBookOfBusinessContact;
                if (!c) return;
                const idx = i + j;
                allMembers[idx].gender = c.gender || null;
                allMembers[idx].middle_name = c.middle_name || null;
                allMembers[idx].address_line1 = c.address || null;
                allMembers[idx].address_line2 = c.address2 || null;
                allMembers[idx].zip = c.zip_code || null;
                allMembers[idx].county = c.county || null;
                allMembers[idx].mbi = c.medicare_beneficiary_id || null;
                allMembers[idx].application_date = c.application_created_at || null;
                allMembers[idx].member_record_locator = c.member_record_locator || null;
                allMembers[idx].medicaid_id = c.medicaid_id || null;
                if (c.provider) {
                    allMembers[idx].provider_first_name = c.provider.first_name || null;
                    allMembers[idx].provider_last_name = c.provider.last_name || null;
                }
            });
        }

        allMembers.forEach(m => delete m._id);

        window.__compassBobFetched = true;
        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(allMembers));
    } catch (e) {
        if (!silent) {
            window.location.href = 'http://compass-sync.localhost/error?message=' +
                encodeURIComponent(e.toString());
        }
    }
};
"#;

/// Auto-login script: fills and submits the Devoted login form.
/// Devoted uses an Okta-based login with email + password fields.
const AUTO_LOGIN_SCRIPT: &str = r#"
(function() {
    if (!window.__compass_creds) return;
    function tryLogin() {
        var userField = document.querySelector('input[name="identifier"], input[name="username"], input[type="email"]');
        var passField = document.querySelector('input[name="credentials.passcode"], input[name="password"], input[type="password"]');
        if (!userField || !passField) return false;
        var nativeSet = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
        nativeSet.call(userField, window.__compass_creds.username);
        userField.dispatchEvent(new Event('input', { bubbles: true }));
        userField.dispatchEvent(new Event('change', { bubbles: true }));
        nativeSet.call(passField, window.__compass_creds.password);
        passField.dispatchEvent(new Event('input', { bubbles: true }));
        passField.dispatchEvent(new Event('change', { bubbles: true }));
        var submit = document.querySelector('input[type="submit"], button[type="submit"]');
        if (submit) { submit.click(); return true; }
        return false;
    }
    var iv = setInterval(function() { if (tryLogin()) clearInterval(iv); }, 500);
    setTimeout(function() { clearInterval(iv); }, 15000);
})();
"#;

/// Manual fetch script: resets flag and runs with error reporting.
const FETCH_SCRIPT: &str = r#"
window.__compassBobFetched = false;
window.__compassFetchDevoted(false);
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
    id: Option<String>,
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

#[derive(Debug, Deserialize)]
struct DetailGraphQLResponse {
    data: Option<DetailGraphQLData>,
    #[allow(dead_code)]
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct DetailGraphQLData {
    GetBookOfBusinessContact: Option<BobContactDetail>,
}

#[derive(Debug, Deserialize)]
struct BobContactDetail {
    gender: Option<String>,
    middle_name: Option<String>,
    address: Option<String>,
    address2: Option<String>,
    zip_code: Option<String>,
    county: Option<String>,
    medicare_beneficiary_id: Option<String>,
    application_created_at: Option<String>,
    member_record_locator: Option<String>,
    medicaid_id: Option<String>,
    provider: Option<BobProvider>,
}

#[derive(Debug, Deserialize)]
struct BobProvider {
    first_name: Option<String>,
    last_name: Option<String>,
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

    fn auto_login_script(&self) -> &str {
        AUTO_LOGIN_SCRIPT
    }

    fn auto_fetch(&self) -> bool {
        true
    }

    fn sync_instruction(&self) -> &str {
        "Log in to Devoted — data will sync automatically."
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
        let mut all_members: Vec<(Option<String>, PortalMember)> = Vec::new();
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

                all_members.push((
                    contact.id.clone(),
                    PortalMember {
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
                        gender: None,
                        middle_name: None,
                        address_line1: None,
                        address_line2: None,
                        zip: None,
                        county: None,
                        mbi: None,
                        application_date: None,
                        member_record_locator: None,
                        medicaid_id: None,
                        provider_first_name: None,
                        provider_last_name: None,
                    },
                ));
            }

            if data.ListBookOfBusinessContacts.page_info.has_next_page {
                page += 1;
            } else {
                break;
            }
        }

        // Fetch detail for each member to get address, gender, MBI, etc.
        for (contact_id, member) in &mut all_members {
            let cid = match contact_id {
                Some(id) => id.clone(),
                None => continue,
            };

            let body = serde_json::json!({
                "operationName": "GetBookOfBusinessContact",
                "variables": { "id": cid },
                "extensions": {
                    "persistedQuery": {
                        "version": 1,
                        "sha256Hash": DETAIL_QUERY_HASH
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
                .await;

            let resp = match resp {
                Ok(r) => r,
                Err(_) => continue,
            };

            if !resp.status().is_success() {
                continue;
            }

            let detail_resp: DetailGraphQLResponse = match resp.json().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            if let Some(data) = detail_resp.data {
                if let Some(detail) = data.GetBookOfBusinessContact {
                    member.gender = detail.gender;
                    member.middle_name = detail.middle_name;
                    member.address_line1 = detail.address;
                    member.address_line2 = detail.address2;
                    member.zip = detail.zip_code;
                    member.county = detail.county;
                    member.mbi = detail.medicare_beneficiary_id;
                    member.application_date = detail.application_created_at;
                    member.member_record_locator = detail.member_record_locator;
                    member.medicaid_id = detail.medicaid_id;
                    if let Some(provider) = detail.provider {
                        member.provider_first_name = provider.first_name;
                        member.provider_last_name = provider.last_name;
                    }
                }
            }
        }

        Ok(all_members.into_iter().map(|(_, m)| m).collect())
    }
}
