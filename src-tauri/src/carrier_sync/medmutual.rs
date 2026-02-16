use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct MedMutualPortal;

const LOGIN_URL: &str = "https://mybrokerlink.com/";

/// Fetch the Book of Business page and parse the server-rendered HTML table.
/// Works regardless of which page the user is currently on — fetches
/// /mybusiness/bookofbusiness via AJAX using the browser's session cookies.
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        // Fetch the BoB page (session cookies sent automatically)
        const resp = await fetch('/mybusiness/bookofbusiness');
        if (resp.status === 401 || resp.status === 302 || resp.status === 403) {
            throw new Error('Session expired. Close this window, re-open the portal, log in again, and retry.');
        }
        if (!resp.ok) {
            throw new Error('Failed to fetch Book of Business page: HTTP ' + resp.status);
        }

        const html = await resp.text();
        const doc = new DOMParser().parseFromString(html, 'text/html');
        const table = doc.querySelector('#member-table');
        if (!table) {
            throw new Error(
                'Could not find the member table. ' +
                'Make sure you are logged in to MyBrokerLink.'
            );
        }

        // Convert MM/DD/YYYY to YYYY-MM-DD
        function toIso(dateStr) {
            if (!dateStr) return null;
            const m = dateStr.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
            return m ? (m[3] + '-' + m[1] + '-' + m[2]) : dateStr;
        }

        const rows = table.querySelectorAll('tbody tr');
        const members = [];

        for (const row of rows) {
            const getText = (colName) => {
                const td = row.querySelector('td[data-col-name="' + colName + '"]');
                if (!td) return null;
                const content = td.querySelector('.sb-content');
                if (!content) return null;
                const text = content.textContent.trim();
                return text || null;
            };

            const fullName = getText('Name') || '';
            const parts = fullName.split(/\s+/);
            const firstName = parts[0] || '';
            const lastName = parts.slice(1).join(' ') || '';

            // Status: empty = active, "Canceled" button text = canceled
            const statusTd = row.querySelector('td[data-col-name="Attention"]');
            let status = null;
            if (statusTd) {
                const btn = statusTd.querySelector('button');
                status = btn ? btn.textContent.trim() : null;
            }

            members.push({
                first_name: firstName,
                last_name:  lastName,
                member_id:  getText('GroupNumber'),
                dob:        toIso(getText('DateOfBirth')),
                plan_name:  getText('MarketSegment'),
                effective_date: toIso(getText('EffectiveDate')),
                end_date:   null,
                status:     status || 'Active',
                policy_status: null,
                state: getText('State'),
                city:  getText('City'),
                phone: getText('Phone'),
                email: getText('Email')
            });
        }

        window.location.href = 'http://sheeps-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(members));
    } catch (e) {
        window.location.href = 'http://sheeps-sync.localhost/error?message=' +
            encodeURIComponent(e.toString());
    }
})();
"#;

#[async_trait]
impl CarrierPortal for MedMutualPortal {
    fn carrier_id(&self) -> &str {
        "carrier-medmutual"
    }

    fn carrier_name(&self) -> &str {
        "Medical Mutual of Ohio"
    }

    fn login_url(&self) -> &str {
        LOGIN_URL
    }

    fn fetch_script(&self) -> &str {
        FETCH_SCRIPT
    }

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("Medical Mutual reqwest fallback not implemented yet".into()))
    }
}
