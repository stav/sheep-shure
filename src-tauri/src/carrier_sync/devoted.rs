use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct DevotedPortal;

const LOGIN_URL: &str = "https://agent.devoted.com/";

// Placeholder — the real GraphQL endpoint and query shape need to be
// reverse-engineered by inspecting DevTools while the user is logged in.
// Once we know the exact URL, query name, and response shape, we fill
// these in.
const GRAPHQL_ENDPOINT: &str = "https://agent.devoted.com/graphql";

const MEMBERS_QUERY: &str = r#"
query AgentMembers($first: Int, $after: String) {
    agentMembers(first: $first, after: $after) {
        edges {
            node {
                firstName
                lastName
                memberId
                planName
                effectiveDate
                endDate
                status
            }
        }
        pageInfo {
            hasNextPage
            endCursor
        }
    }
}
"#;

/// The expected shape of Devoted's GraphQL response.
/// This is a best-guess placeholder — adjust once real response is captured.
#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQLData {
    agent_members: MembersConnection,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MembersConnection {
    edges: Vec<MemberEdge>,
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct MemberEdge {
    node: MemberNode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemberNode {
    first_name: Option<String>,
    last_name: Option<String>,
    member_id: Option<String>,
    plan_name: Option<String>,
    effective_date: Option<String>,
    end_date: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[async_trait]
impl CarrierPortal for DevotedPortal {
    fn carrier_id(&self) -> &str {
        "devoted"
    }

    fn carrier_name(&self) -> &str {
        "Devoted Health"
    }

    fn login_url(&self) -> &str {
        LOGIN_URL
    }

    async fn fetch_members(&self, auth_token: &str) -> Result<Vec<PortalMember>, AppError> {
        let client = reqwest::Client::new();
        let mut all_members = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let variables = serde_json::json!({
                "first": 100,
                "after": cursor,
            });

            let body = serde_json::json!({
                "query": MEMBERS_QUERY,
                "variables": variables,
            });

            let resp = client
                .post(GRAPHQL_ENDPOINT)
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, format!("Bearer {}", auth_token))
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

            for edge in &data.agent_members.edges {
                let node = &edge.node;
                all_members.push(PortalMember {
                    first_name: node.first_name.clone().unwrap_or_default(),
                    last_name: node.last_name.clone().unwrap_or_default(),
                    member_id: node.member_id.clone(),
                    plan_name: node.plan_name.clone(),
                    effective_date: node.effective_date.clone(),
                    end_date: node.end_date.clone(),
                    status: node.status.clone(),
                });
            }

            if data.agent_members.page_info.has_next_page {
                cursor = data.agent_members.page_info.end_cursor;
            } else {
                break;
            }
        }

        Ok(all_members)
    }
}
