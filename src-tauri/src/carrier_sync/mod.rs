pub mod anthem;
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

    /// JS to auto-fill and submit the login form using window.__compass_creds.
    /// Default is empty (carrier doesn't support auto-login yet).
    fn auto_login_script(&self) -> &str {
        ""
    }

    /// JS code to inject into the webview after the user has logged in.
    /// The script should fetch member data from the portal API and then navigate to:
    ///   `http://compass-sync.localhost/data?members=<encodeURIComponent(JSON)>`
    /// on success, or:
    ///   `http://compass-sync.localhost/error?message=<encodeURIComponent(msg)>`
    /// on failure.
    fn fetch_script(&self) -> &str;

    /// Whether this carrier auto-fetches data after login (via init_script).
    /// When true, the UI skips the manual "Sync Now" step and shows
    /// "Syncing automatically after login…" instead.
    fn auto_fetch(&self) -> bool {
        false
    }

    /// Instruction text shown in the UI while the user is in the login phase.
    fn sync_instruction(&self) -> &str {
        "Log in, navigate to the Book of Business page, then click Sync Now."
    }

    /// Fetch members via HTTP using cookies (fallback approach).
    async fn fetch_members(&self, cookies: &str) -> Result<Vec<PortalMember>, AppError>;
}

/// Look up the carrier portal implementation by carrier_id.
pub fn get_portal(carrier_id: &str) -> Option<Box<dyn CarrierPortal>> {
    match carrier_id {
        "carrier-anthem" => Some(Box::new(anthem::AnthemPortal)),
        "carrier-devoted" => Some(Box::new(devoted::DevotedPortal)),
        "carrier-caresource" => Some(Box::new(caresource::CareSourcePortal)),
        "carrier-medmutual" => Some(Box::new(medmutual::MedMutualPortal)),
        "carrier-uhc" => Some(Box::new(uhc::UhcPortal)),
        "carrier-humana" => Some(Box::new(humana::HumanaPortal)),
        _ => None,
    }
}
