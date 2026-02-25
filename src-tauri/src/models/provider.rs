use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientProvider {
    pub id: String,
    pub client_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub npi: Option<String>,
    pub specialty: Option<String>,
    pub phone: Option<String>,
    pub is_pcp: Option<i32>,
    pub source: Option<String>,
    pub is_active: Option<i32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderInput {
    pub client_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub npi: Option<String>,
    pub specialty: Option<String>,
    pub phone: Option<String>,
    pub is_pcp: Option<i32>,
    pub source: Option<String>,
}
