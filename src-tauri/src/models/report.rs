use serde::{Deserialize, Serialize};

use super::client::ClientFilters;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDefinition {
    pub name: String,
    pub filters: ClientFilters,
    pub columns: Vec<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub group_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_active_clients: i64,
    pub new_this_month: i64,
    pub lost_this_month: i64,
    pub pending_enrollments: i64,
    pub by_plan_type: Vec<(String, i64)>,
    pub by_carrier: Vec<(String, i64)>,
    pub by_state: Vec<(String, i64)>,
    pub monthly_trend: Vec<MonthlyTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyTrend {
    pub month: String,
    pub new_clients: i64,
    pub lost_clients: i64,
    pub net: i64,
}

impl Default for DashboardStats {
    fn default() -> Self {
        DashboardStats {
            total_active_clients: 0,
            new_this_month: 0,
            lost_this_month: 0,
            pending_enrollments: 0,
            by_plan_type: Vec::new(),
            by_carrier: Vec::new(),
            by_state: Vec::new(),
            monthly_trend: Vec::new(),
        }
    }
}
