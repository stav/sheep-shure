use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct HumanaPortal;

const LOGIN_URL: &str = "https://account.humana.com/";

/// Auto-login script: fills and submits the Humana login form.
/// Uses a global flag to prevent re-submitting after a failed login attempt
/// (the init script re-runs on every page load/navigation).
/// Also handles the post-login "Select where you want to sign in" modal
/// by automatically clicking the "Agent" button.
const AUTO_LOGIN_SCRIPT: &str = r#"
(function() {
    if (!window.__compass_creds) return;
    if (window.__compass_login_submitted) return;

    // ── Submit the portal-selection form as "Agent" ──
    function tryPickAgent() {
        var form = document.getElementById('multiPortalAccessForm');
        if (!form) return false;

        // Set the hidden SelectedPortal field
        var sel = form.querySelector('input[name="SelectedPortal"]');
        if (sel) sel.value = 'Agent';

        // Find the Agent submit button (second button in the form)
        var buttons = form.querySelectorAll('button[type="submit"]');
        var agentBtn = null;
        for (var i = 0; i < buttons.length; i++) {
            if (buttons[i].textContent.indexOf('Agent') !== -1) {
                agentBtn = buttons[i];
                break;
            }
        }

        // Try requestSubmit with the button (preserves submitter info)
        if (agentBtn && form.requestSubmit) {
            try { form.requestSubmit(agentBtn); return true; } catch (e) {}
        }
        // Fallback: plain form.submit()
        form.submit();
        return true;
    }

    // ── Check if login error is showing — don't retry ──
    function hasLoginError() {
        var t = document.body ? document.body.textContent : '';
        return t.indexOf('don\u2019t recognize') !== -1 ||
               t.indexOf("don't recognize") !== -1 ||
               t.indexOf('invalid credentials') !== -1 ||
               t.indexOf('account is locked') !== -1 ||
               t.indexOf('too many attempts') !== -1;
    }

    // ── Try to fill and submit the login form ──
    function tryLogin() {
        if (hasLoginError()) return 'error';
        var passField = document.querySelector('input[type="password"]');
        if (!passField) return false;
        var form = passField.closest('form');
        var userField = form
            ? form.querySelector('input[type="text"], input[type="email"]')
            : document.querySelector('input[type="text"], input[type="email"]');
        if (!userField) return false;
        var nativeSet = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
        nativeSet.call(userField, window.__compass_creds.username);
        userField.dispatchEvent(new Event('input', { bubbles: true }));
        userField.dispatchEvent(new Event('change', { bubbles: true }));
        nativeSet.call(passField, window.__compass_creds.password);
        passField.dispatchEvent(new Event('input', { bubbles: true }));
        passField.dispatchEvent(new Event('change', { bubbles: true }));
        var submit = form
            ? (form.querySelector('button[type="submit"], input[type="submit"]') || form.querySelector('button'))
            : document.querySelector('button[type="submit"], input[type="submit"]');
        if (submit) {
            window.__compass_login_submitted = true;
            submit.click();
            return true;
        }
        return false;
    }

    // ── Unified polling: handle login form OR role-selection modal ──
    setTimeout(function() {
        var iv = setInterval(function() {
            // If the role-selection modal appeared, submit the form as "Agent"
            var form = document.getElementById('multiPortalAccessForm');
            if (form) {
                clearInterval(iv);
                tryPickAgent();
                return;
            }
            // Otherwise try login
            var result = tryLogin();
            if (result === true || result === 'error') clearInterval(iv);
        }, 500);
        setTimeout(function() { clearInterval(iv); }, 30000);
    }, 1000);
})();
"#;

/// Fetch Humana Vantage member data via the business center API.
/// This is more reliable than DOM scraping since the Vantage React SPA
/// has click-handling issues in the Tauri webview.
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        function parseName(nameStr) {
            if (!nameStr) return { first: '', last: '' };
            var commaIdx = nameStr.indexOf(',');
            if (commaIdx === -1) return { first: nameStr.trim(), last: '' };
            var last = nameStr.substring(0, commaIdx).trim();
            var first = nameStr.substring(commaIdx + 1).trim();
            return { first: first, last: last };
        }

        function toIsoDate(dateStr) {
            if (!dateStr) return null;
            // Handle ISO datetime like "2025-08-01T00:00:00Z"
            var m = dateStr.match(/^(\d{4}-\d{2}-\d{2})/);
            return m ? m[1] : dateStr;
        }

        // Fetch all pages from the Vantage API
        var allRecords = [];
        var page = 0;
        var pageSize = 50;
        var totalRecords = null;

        while (true) {
            var resp = await fetch('/Vantage/api/businesscenter/search-policies-and-applications', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Accept': 'application/json',
                    'Authorization': 'Basic VmFudGFnZVdlYkFwcDpwN1JFdmVkIzE='
                },
                body: JSON.stringify({
                    filters: { dateFilter: null, filterValuesIds: [] },
                    insightId: 'all',
                    resultPaging: { amount: pageSize, page: page },
                    resultSort: { columnId: 49, order: 'asc' }
                })
            });

            if (!resp.ok) {
                if (resp.status === 401 || resp.status === 403) {
                    throw new Error(
                        'Session expired (HTTP ' + resp.status + '). ' +
                        'Close this window, re-open the portal, log in again, and retry.'
                    );
                }
                var errText = await resp.text().catch(function() { return ''; });
                throw new Error('API returned ' + resp.status + ': ' + errText.substring(0, 300));
            }

            var data = await resp.json();

            if (!data.records || data.records.length === 0) {
                if (page === 0) {
                    throw new Error('No records returned from the API. Make sure you are logged in as an agent.');
                }
                break;
            }

            allRecords = allRecords.concat(data.records);
            totalRecords = data.totalRecords || allRecords.length;

            if (allRecords.length >= totalRecords) break;
            page++;
            if (page > 100) break; // safety limit
        }

        var members = allRecords.map(function(r) {
            var name = parseName(r.mbrName);
            var phone = r.mbrPrimPhone;
            return {
                first_name: name.first,
                last_name: name.last,
                member_id: r.umid || null,
                dob: toIsoDate(r.birthDate),
                plan_name: r.planAltDesc || [r.planType, r.salesProduct].filter(Boolean).join(' - ') || null,
                effective_date: toIsoDate(r.covEffDate),
                end_date: toIsoDate(r.covTermDate),
                status: r.statusReasonDesc || r.status || 'Active',
                policy_status: r.status || null,
                state: null,
                city: null,
                phone: (phone && phone !== 'Unavailable') ? phone : null,
                email: r.mbrEmail || null
            };
        });

        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(members));
    } catch (e) {
        window.location.href = 'http://compass-sync.localhost/error?message=' +
            encodeURIComponent(e.toString());
    }
})();
"#;

#[async_trait]
impl CarrierPortal for HumanaPortal {
    fn carrier_id(&self) -> &str {
        "carrier-humana"
    }

    fn carrier_name(&self) -> &str {
        "Humana"
    }

    fn login_url(&self) -> &str {
        LOGIN_URL
    }

    fn fetch_script(&self) -> &str {
        FETCH_SCRIPT
    }

    fn auto_login_script(&self) -> &str {
        AUTO_LOGIN_SCRIPT
    }

    fn sync_instruction(&self) -> &str {
        "Log in, navigate to My Humana Business, then click Sync Now."
    }

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("Humana reqwest fallback not implemented yet".into()))
    }
}
