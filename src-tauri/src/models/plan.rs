use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub carrier_id: String,
    pub plan_type_code: String,
    pub plan_name: String,
    pub contract_number: Option<String>,
    pub pbp_number: Option<String>,
    pub segment_id: Option<String>,
    pub plan_year: Option<i32>,
    pub state: Option<String>,
    pub county_fips: Option<String>,
    pub premium: Option<f64>,
    pub is_active: Option<i32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanListItem {
    pub id: String,
    pub carrier_name: Option<String>,
    pub plan_type_code: String,
    pub plan_name: String,
    pub contract_number: Option<String>,
    pub pbp_number: Option<String>,
    pub plan_year: Option<i32>,
    pub premium: Option<f64>,
}
