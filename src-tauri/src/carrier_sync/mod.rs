pub mod caresource;
pub mod devoted;
pub mod humana;
pub mod medmutual;
pub mod uhc;

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

    /// Optional JS that runs at document-start in the carrier webview.
    /// Default is empty (no init script needed).
    fn init_script(&self) -> &str {
        ""
    }

    /// JS code to inject into the webview after the user has logged in.
    /// The script should fetch member data from the portal API and then navigate to:
    ///   `http://sheeps-sync.localhost/data?members=<encodeURIComponent(JSON)>`
    /// on success, or:
    ///   `http://sheeps-sync.localhost/error?message=<encodeURIComponent(msg)>`
    /// on failure.
    fn fetch_script(&self) -> &str;

    /// Fetch members via HTTP using cookies (fallback approach).
    async fn fetch_members(&self, cookies: &str) -> Result<Vec<PortalMember>, AppError>;
}

/// Look up the carrier portal implementation by carrier_id.
pub fn get_portal(carrier_id: &str) -> Option<Box<dyn CarrierPortal>> {
    match carrier_id {
        "carrier-devoted" => Some(Box::new(devoted::DevotedPortal)),
        "carrier-caresource" => Some(Box::new(caresource::CareSourcePortal)),
        "carrier-medmutual" => Some(Box::new(medmutual::MedMutualPortal)),
        "carrier-uhc" => Some(Box::new(uhc::UhcPortal)),
        "carrier-humana" => Some(Box::new(humana::HumanaPortal)),
        _ => None,
    }
}
