use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carrier {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub is_active: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierWithCounts {
    pub id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub is_active: Option<i32>,
    pub enrollment_count: i64,
}
