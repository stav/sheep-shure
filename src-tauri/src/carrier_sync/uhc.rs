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
            if (opd) window.__sheeps_uhc_opd = opd;
            const hp = urlObj.searchParams.get('hasPrincipalOrCorp');
            if (hp !== null) window.__sheeps_uhc_hasPrincipal = hp;
        } catch (e) {}
    }

    function extractFromBody(body) {
        if (!body) return;
        try {
            const parsed = typeof body === 'string' ? JSON.parse(body) : body;
            if (parsed.partyID) window.__sheeps_uhc_partyID = parsed.partyID;
        } catch (e) {}
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
            }
        } catch (e) {}
        return origFetch.apply(this, arguments);
    };

    // Patch XHR
    const origOpen = XMLHttpRequest.prototype.open;
    const origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__sheeps_url = typeof url === 'string' ? url : String(url);
        return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function(body) {
        try {
            if (this.__sheeps_url && this.__sheeps_url.includes('bookOfBusiness')) {
                extractFromUrl(this.__sheeps_url);
                extractFromBody(body);
            }
        } catch (e) {}
        return origSend.apply(this, arguments);
    };
})();
"#;

/// Fetch all active members from the Jarvis Book of Business API.
/// Uses partyID and opd captured by init_script from the SPA's own call.
/// No pagination — API returns up to 500 records in a single response.
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        let partyID = window.__sheeps_uhc_partyID;
        let opd = window.__sheeps_uhc_opd;

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
                    // Handle stringified JSON nested inside values
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
                    // Search the response object for partyID
                    else {
                        const txt = JSON.stringify(pidData);
                        const m = txt.match(/"party[Ii][Dd]"\s*:\s*"([^"]+)"/);
                        if (m) partyID = m[1];
                    }
                }
            } catch (e) {}
        }

        if (!partyID || !opd) {
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

        const hasPrincipal = window.__sheeps_uhc_hasPrincipal || 'false';
        const url = '/JarvisMemberProfileAPI/azure/api/secure/bookOfBusiness/details/v1' +
            '?hasPrincipalOrCorp=' + encodeURIComponent(hasPrincipal) +
            '&opd=' + encodeURIComponent(opd) +
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

        if (resp.status === 401 || resp.status === 403) {
            throw new Error(
                'Session expired (HTTP ' + resp.status + '). ' +
                'Close this window, re-open the portal, log in again, and retry.'
            );
        }
        if (!resp.ok) {
            const text = await resp.text().catch(() => '');
            throw new Error('API returned ' + resp.status + ': ' + text.substring(0, 300));
        }

        const data = await resp.json();
        if (data.errors && data.errors.length > 0) {
            throw new Error('API errors: ' + data.errors.join('; '));
        }

        const list = data.bookOfBusinessList || [];

        // Convert MM/DD/YYYY to YYYY-MM-DD
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

        window.location.href = 'http://sheeps-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(members));
    } catch (e) {
        window.location.href = 'http://sheeps-sync.localhost/error?message=' +
            encodeURIComponent(e.toString());
    }
})();
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

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("UHC reqwest fallback not implemented yet".into()))
    }
}
