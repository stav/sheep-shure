use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct UhcPortal;

const LOGIN_URL: &str = "https://www.uhcjarvis.com/content/jarvis/en/secure/book-of-business-search.html";

/// Intercept the Jarvis SPA's own bookOfBusiness API call to capture the
/// agent's partyID (from request body) and opd (from query string).
/// Patches both fetch and XHR since Angular may use either.
const INIT_SCRIPT: &str = r#"
(function() {
    function extractFromUrl(url) {
        try {
            const urlObj = new URL(url, window.location.origin);
            const opd = urlObj.searchParams.get('opd');
            if (opd) window.__compass_uhc_opd = opd;
            const hp = urlObj.searchParams.get('hasPrincipalOrCorp');
            if (hp !== null) window.__compass_uhc_hasPrincipal = hp;
        } catch (e) {}
    }

    function extractFromBody(body) {
        if (!body) return;
        try {
            const parsed = typeof body === 'string' ? JSON.parse(body) : body;
            if (parsed.partyID) window.__compass_uhc_partyID = parsed.partyID;
        } catch (e) {}
    }

    // Auto-trigger fetch once both credentials are captured
    function tryAutoFetch() {
        if (window.__compass_uhc_partyID && window.__compass_uhc_opd && !window.__compassBobFetched) {
            setTimeout(function() {
                if (window.__compassFetchUhc && !window.__compassBobFetched) {
                    window.__compassFetchUhc(true);
                }
            }, 500);
        }
    }

    // Patch fetch
    const origFetch = window.fetch;
    window.fetch = function(resource, init) {
        try {
            const url = typeof resource === 'string' ? resource :
                         (resource instanceof Request ? resource.url : String(resource));
            if (url.includes('bookOfBusiness')) {
                extractFromUrl(url);
                if (init && init.body) extractFromBody(init.body);
                tryAutoFetch();
            }
        } catch (e) {}
        return origFetch.apply(this, arguments);
    };

    // Patch XHR
    const origOpen = XMLHttpRequest.prototype.open;
    const origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__compass_url = typeof url === 'string' ? url : String(url);
        return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function(body) {
        try {
            if (this.__compass_url && this.__compass_url.includes('bookOfBusiness')) {
                extractFromUrl(this.__compass_url);
                extractFromBody(body);
                tryAutoFetch();
            }
        } catch (e) {}
        return origSend.apply(this, arguments);
    };
})();

// ── Fetch function (also called by init interceptor above) ──
window.__compassFetchUhc = async function(silent) {
    try {
        if (window.__compassBobFetched) return;

        let partyID = window.__compass_uhc_partyID;
        let opd = window.__compass_uhc_opd;

        // Fallback: try to extract opd from Performance API entries
        if (!opd || !partyID) {
            const entries = performance.getEntriesByType('resource');
            for (const entry of entries) {
                if (entry.name.includes('bookOfBusiness')) {
                    try {
                        const u = new URL(entry.name);
                        if (!opd) opd = u.searchParams.get('opd');
                    } catch (e) {}
                }
            }
        }

        // Fallback: deep-search localStorage and sessionStorage
        if (!partyID || !opd) {
            function deepFind(obj, depth) {
                if (!obj || typeof obj !== 'object' || depth > 4) return;
                if (!partyID && (obj.partyID || obj.partyId)) partyID = obj.partyID || obj.partyId;
                if (!opd && obj.opd) opd = obj.opd;
                for (const k in obj) {
                    if (typeof obj[k] === 'object') deepFind(obj[k], depth + 1);
                    if (typeof obj[k] === 'string' && obj[k].startsWith('{')) {
                        try { deepFind(JSON.parse(obj[k]), depth + 1); } catch (e) {}
                    }
                }
            }
            for (const store of [sessionStorage, localStorage]) {
                for (let i = 0; i < store.length; i++) {
                    const val = store.getItem(store.key(i));
                    try { deepFind(JSON.parse(val), 0); } catch (e) {}
                    if (partyID && opd) break;
                }
                if (partyID && opd) break;
            }
        }

        // Fallback: call the Jarvis partyID API directly
        if (!partyID) {
            try {
                const pidResp = await fetch('/JarvisAccountInfo/azure/api/secure/userprofile/partyID/v1', {
                    method: 'GET',
                    headers: { 'Accept': 'application/json' }
                });
                if (pidResp.ok) {
                    const pidData = await pidResp.json();
                    if (pidData.partyID) partyID = pidData.partyID;
                    else if (pidData.partyId) partyID = pidData.partyId;
                    else {
                        const txt = JSON.stringify(pidData);
                        const m = txt.match(/"party[Ii][Dd]"\s*:\s*"([^"]+)"/);
                        if (m) partyID = m[1];
                    }
                }
            } catch (e) {}
        }

        if (!partyID) {
            if (silent) return;
            const debug = {
                captured: { partyID: partyID || null, opd: opd || null },
                ls_keys: Object.keys(localStorage),
                ss_keys: Object.keys(sessionStorage),
                cookies: document.cookie.split(';').map(function(c) { return c.trim().split('=')[0]; }),
                perf_bob: performance.getEntriesByType('resource')
                    .filter(function(e) { return e.name.includes('Jarvis') || e.name.includes('bookOfBusiness'); })
                    .map(function(e) { return e.name.substring(0, 150); }),
                url: window.location.href
            };
            throw new Error('Could not find agent ID / operator code. Debug: ' + JSON.stringify(debug));
        }

        const hasPrincipal = window.__compass_uhc_hasPrincipal || 'false';
        const url = '/JarvisMemberProfileAPI/azure/api/secure/bookOfBusiness/details/v1' +
            '?hasPrincipalOrCorp=' + encodeURIComponent(hasPrincipal) +
            '&opd=' + encodeURIComponent(opd || '') +
            '&homePage=false';

        const resp = await fetch(url, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json'
            },
            body: JSON.stringify({
                contractNumber: null,
                memberFirstName: '',
                memberLastName: '',
                memberNumber: null,
                planStatus: ['Active'],
                partyID: partyID,
                state: null,
                product: null
            })
        });

        if (!resp.ok) {
            if (silent) return;
            if (resp.status === 401 || resp.status === 403) {
                throw new Error(
                    'Session expired (HTTP ' + resp.status + '). ' +
                    'Close this window, re-open the portal, log in again, and retry.'
                );
            }
            const text = await resp.text().catch(() => '');
            throw new Error('API returned ' + resp.status + ': ' + text.substring(0, 300));
        }

        const respText = await resp.text();
        var data;
        try {
            data = JSON.parse(respText);
        } catch (parseErr) {
            if (silent) return;
            throw new Error(
                'API returned non-JSON response (possible session issue). ' +
                'Try closing this window, re-opening the portal, and syncing again. ' +
                'Response preview: ' + respText.substring(0, 200)
            );
        }
        if (data.errors && data.errors.length > 0) {
            if (silent) return;
            throw new Error('API errors: ' + data.errors.join('; '));
        }

        const list = data.bookOfBusinessList || [];

        function toIso(dateStr) {
            if (!dateStr) return null;
            const m = dateStr.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
            return m ? (m[3] + '-' + m[1] + '-' + m[2]) : dateStr;
        }

        const members = list.map(function(m) {
            return {
                first_name: (m.memberFirstName || '').trim(),
                last_name:  (m.memberLastName || '').trim(),
                member_id:  m.memberNumber || m.mbiNumber || null,
                dob:        toIso(m.dateOfBirth),
                plan_name:  m.planName || null,
                effective_date: m.policyEffectiveDate || null,
                end_date:   (m.policyTermDate && m.policyTermDate !== '2300-01-01')
                                ? m.policyTermDate : null,
                status:     m.memberStatus === 'A' ? 'Active' : (m.memberStatus || null),
                policy_status: null,
                state: m.memberState || null,
                city:  m.memberCity || null,
                phone: m.memberPhone || null,
                email: m.memberEmail || null
            };
        });

        window.__compassBobFetched = true;
        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(members));
    } catch (e) {
        if (!silent) {
            window.location.href = 'http://compass-sync.localhost/error?message=' +
                encodeURIComponent(e.toString());
        }
    }
};

// Fallback: if the fetch/XHR interceptor didn't trigger auto-fetch
// (e.g. zone.js overwrote our patches), retry periodically after load.
window.addEventListener('load', () => {
    var attempts = 0;
    var retryIv = setInterval(() => {
        if (window.__compassBobFetched || attempts >= 6) {
            clearInterval(retryIv);
            return;
        }
        attempts++;
        if (window.__compassFetchUhc) {
            window.__compassFetchUhc(true);
        }
    }, 5000);
});
"#;

/// Auto-login script: fills and submits the UHC/Jarvis login form.
/// Handles multi-step login (username + Continue → password + Sign In).
const AUTO_LOGIN_SCRIPT: &str = r#"
(function() {
    if (!window.__compass_creds) return;
    var usernameFilled = false;
    // Find a clickable button by type="submit" or by text content
    function findButton() {
        var btn = document.querySelector('input[type="submit"], button[type="submit"]');
        if (btn) return btn;
        var buttons = document.querySelectorAll('button');
        for (var i = 0; i < buttons.length; i++) {
            var text = buttons[i].textContent.trim().toLowerCase();
            if (text === 'continue' || text === 'sign in' || text === 'log in' || text === 'next') {
                return buttons[i];
            }
        }
        return null;
    }
    function tryLogin() {
        var passField = document.querySelector('input[type="password"]');
        var userField = document.querySelector('input[type="text"], input[type="email"]');
        var nativeSet = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
        // Step 1: username visible, no password yet
        if (userField && !passField && !usernameFilled) {
            nativeSet.call(userField, window.__compass_creds.username);
            userField.dispatchEvent(new Event('input', { bubbles: true }));
            userField.dispatchEvent(new Event('change', { bubbles: true }));
            usernameFilled = true;
            var nextBtn = findButton();
            if (nextBtn) nextBtn.click();
            return false; // keep polling for password screen
        }
        // Step 2: password visible
        if (passField) {
            nativeSet.call(passField, window.__compass_creds.password);
            passField.dispatchEvent(new Event('input', { bubbles: true }));
            passField.dispatchEvent(new Event('change', { bubbles: true }));
            if (userField && !usernameFilled) {
                nativeSet.call(userField, window.__compass_creds.username);
                userField.dispatchEvent(new Event('input', { bubbles: true }));
                userField.dispatchEvent(new Event('change', { bubbles: true }));
            }
            var submit = findButton();
            if (submit) { submit.click(); return true; }
        }
        return false;
    }
    // Delay before polling to let SSO redirects complete naturally
    setTimeout(function() {
        var iv = setInterval(function() { if (tryLogin()) clearInterval(iv); }, 500);
        setTimeout(function() { clearInterval(iv); }, 15000);
    }, 2000);
})();
"#;

/// Manual fetch script: resets flag and runs with error reporting.
const FETCH_SCRIPT: &str = r#"
window.__compassBobFetched = false;
window.__compassFetchUhc(false);
"#;

#[async_trait]
impl CarrierPortal for UhcPortal {
    fn carrier_id(&self) -> &str {
        "carrier-uhc"
    }

    fn carrier_name(&self) -> &str {
        "UnitedHealthcare"
    }

    fn login_url(&self) -> &str {
        LOGIN_URL
    }

    fn init_script(&self) -> &str {
        INIT_SCRIPT
    }

    fn fetch_script(&self) -> &str {
        FETCH_SCRIPT
    }

    fn auto_login_script(&self) -> &str {
        AUTO_LOGIN_SCRIPT
    }

    fn auto_fetch(&self) -> bool {
        true
    }

    fn sync_instruction(&self) -> &str {
        "Log in to Jarvis — data will sync automatically."
    }

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("UHC reqwest fallback not implemented yet".into()))
    }
}
