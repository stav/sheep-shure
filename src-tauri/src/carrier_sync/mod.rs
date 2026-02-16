pub mod devoted;

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

/// Trait that each carrier portal integration must implement.
#[async_trait]
pub trait CarrierPortal: Send + Sync {
    /// The carrier_id that matches `carriers.id` in the local database.
    fn carrier_id(&self) -> &str;

    /// Human-readable carrier name.
    fn carrier_name(&self) -> &str;

    /// The URL the user navigates to in the webview to log in.
    fn login_url(&self) -> &str;

    /// Fetch enrolled members from the carrier portal using the provided auth token.
    async fn fetch_members(&self, auth_token: &str) -> Result<Vec<PortalMember>, AppError>;
}

/// Look up the carrier portal implementation by carrier_id.
pub fn get_portal(carrier_id: &str) -> Option<Box<dyn CarrierPortal>> {
    match carrier_id {
        "devoted" => Some(Box::new(devoted::DevotedPortal)),
        _ => None,
    }
}
