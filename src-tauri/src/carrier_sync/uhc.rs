use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct UhcPortal;

const LOGIN_URL: &str = "https://www.uhcjarvis.com/content/jarvis/en/secure/book-of-business-search.html";

/// Intercept the Jarvis SPA's own bookOfBusiness API response to capture
/// member data directly. Patches both fetch and XHR since Angular may use
/// either. This avoids making a second API call (which can fail due to
/// missing auth headers / CSRF tokens).
const INIT_SCRIPT: &str = r#"
(function() {
    // ── Shared helpers ──

    function toIso(dateStr) {
        if (!dateStr) return null;
        var m = dateStr.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
        return m ? (m[3] + '-' + m[1] + '-' + m[2]) : dateStr;
    }

    function mapMembers(list) {
        return list.map(function(m) {
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
    }

    function sendData(members) {
        if (window.__compassBobFetched) return;
        window.__compassBobFetched = true;
        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(members));
    }

    function sendError(message) {
        window.location.href = 'http://compass-sync.localhost/error?message=' +
            encodeURIComponent(message);
    }

    function isBobDetailsUrl(url) {
        return url.includes('bookOfBusiness') && url.includes('details');
    }

    function extractFromUrl(url) {
        try {
            var urlObj = new URL(url, window.location.origin);
            var opd = urlObj.searchParams.get('opd');
            if (opd) window.__compass_uhc_opd = opd;
            var hp = urlObj.searchParams.get('hasPrincipalOrCorp');
            if (hp !== null) window.__compass_uhc_hasPrincipal = hp;
        } catch (e) {}
    }

    function extractFromBody(body) {
        if (!body) return;
        try {
            var parsed = typeof body === 'string' ? JSON.parse(body) : body;
            if (parsed.partyID) window.__compass_uhc_partyID = parsed.partyID;
        } catch (e) {}
    }

    function processApiResponse(text) {
        try {
            var data = JSON.parse(text);
            if (data.bookOfBusinessList) {
                var members = mapMembers(data.bookOfBusinessList);
                sendData(members);
                return true;
            }
        } catch (e) {}
        return false;
    }

    // ── Patch fetch: intercept RESPONSE from Angular's BoB API call ──
    var origFetch = window.fetch;
    window.fetch = function(resource, init) {
        try {
            var url = typeof resource === 'string' ? resource :
                       (resource instanceof Request ? resource.url : String(resource));
            if (isBobDetailsUrl(url)) {
                extractFromUrl(url);
                if (init && init.body) extractFromBody(init.body);

                // Save the original request info for manual replay
                window.__compass_uhc_lastBobUrl = url;
                window.__compass_uhc_lastBobInit = init;

                // Intercept the response
                return origFetch.apply(this, arguments).then(function(response) {
                    if (response.ok && !window.__compassBobFetched) {
                        var clone = response.clone();
                        clone.text().then(function(text) {
                            processApiResponse(text);
                        }).catch(function() {});
                    }
                    return response; // return original to Angular untouched
                });
            }
        } catch (e) {}
        return origFetch.apply(this, arguments);
    };

    // ── Patch XHR: intercept RESPONSE from Angular's BoB API call ──
    var origOpen = XMLHttpRequest.prototype.open;
    var origSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__compass_url = typeof url === 'string' ? url : String(url);
        return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.send = function(body) {
        try {
            if (this.__compass_url && isBobDetailsUrl(this.__compass_url)) {
                extractFromUrl(this.__compass_url);
                extractFromBody(body);

                // Save the original request info for manual replay
                window.__compass_uhc_lastBobUrl = this.__compass_url;

                // Intercept response when it arrives
                var xhr = this;
                xhr.addEventListener('load', function() {
                    if (xhr.status >= 200 && xhr.status < 300 && !window.__compassBobFetched) {
                        processApiResponse(xhr.responseText);
                    }
                });
            }
        } catch (e) {}
        return origSend.apply(this, arguments);
    };
})();

// ── Manual fetch function (called by Sync Now button) ──
window.__compassFetchUhc = async function(silent) {
    try {
        if (window.__compassBobFetched) return;

        // Strategy 1: Replay the exact request Angular made (same URL, headers, body)
        if (window.__compass_uhc_lastBobInit) {
            try {
                var replayResp = await fetch(
                    window.__compass_uhc_lastBobUrl,
                    window.__compass_uhc_lastBobInit
                );
                if (replayResp.ok) {
                    var replayText = await replayResp.text();
                    try {
                        var replayData = JSON.parse(replayText);
                        if (replayData.bookOfBusinessList) {
                            var members = (function(list) {
                                function toIso(d) {
                                    if (!d) return null;
                                    var m = d.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
                                    return m ? (m[3] + '-' + m[1] + '-' + m[2]) : d;
                                }
                                return list.map(function(m) {
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
                            })(replayData.bookOfBusinessList);
                            window.__compassBobFetched = true;
                            window.location.href = 'http://compass-sync.localhost/data?members=' +
                                encodeURIComponent(JSON.stringify(members));
                            return;
                        }
                    } catch (e) {
                        // Replay got non-JSON, fall through to strategy 2
                    }
                }
            } catch (e) {
                // Replay failed, fall through
            }
        }

        // Strategy 2: Build request from captured/discovered params
        var partyID = window.__compass_uhc_partyID;
        var opd = window.__compass_uhc_opd;

        // Fallback: try to extract opd from Performance API entries
        if (!opd || !partyID) {
            var entries = performance.getEntriesByType('resource');
            for (var i = 0; i < entries.length; i++) {
                if (entries[i].name.includes('bookOfBusiness')) {
                    try {
                        var u = new URL(entries[i].name);
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
                for (var k in obj) {
                    if (typeof obj[k] === 'object') deepFind(obj[k], depth + 1);
                    if (typeof obj[k] === 'string' && obj[k].startsWith('{')) {
                        try { deepFind(JSON.parse(obj[k]), depth + 1); } catch (e) {}
                    }
                }
            }
            var stores = [sessionStorage, localStorage];
            for (var s = 0; s < stores.length; s++) {
                for (var i = 0; i < stores[s].length; i++) {
                    var val = stores[s].getItem(stores[s].key(i));
                    try { deepFind(JSON.parse(val), 0); } catch (e) {}
                    if (partyID && opd) break;
                }
                if (partyID && opd) break;
            }
        }

        // Fallback: call the Jarvis partyID API directly
        if (!partyID) {
            try {
                var pidResp = await fetch('/JarvisAccountInfo/azure/api/secure/userprofile/partyID/v1', {
                    method: 'GET',
                    headers: { 'Accept': 'application/json' }
                });
                if (pidResp.ok) {
                    var pidData = await pidResp.json();
                    if (pidData.partyID) partyID = pidData.partyID;
                    else if (pidData.partyId) partyID = pidData.partyId;
                    else {
                        var txt = JSON.stringify(pidData);
                        var m = txt.match(/"party[Ii][Dd]"\s*:\s*"([^"]+)"/);
                        if (m) partyID = m[1];
                    }
                }
            } catch (e) {}
        }

        if (!partyID) {
            if (silent) return;
            throw new Error(
                'Could not find agent ID. Make sure you are logged in and ' +
                'the Book of Business page has loaded. Then try Sync Now again.'
            );
        }

        var hasPrincipal = window.__compass_uhc_hasPrincipal || 'false';
        var url = '/JarvisMemberProfileAPI/azure/api/secure/bookOfBusiness/details/v1' +
            '?hasPrincipalOrCorp=' + encodeURIComponent(hasPrincipal) +
            '&opd=' + encodeURIComponent(opd || '') +
            '&homePage=false';

        var resp = await fetch(url, {
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
            var text = await resp.text().catch(function() { return ''; });
            throw new Error('API returned ' + resp.status + ': ' + text.substring(0, 300));
        }

        var respText = await resp.text();
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

        var list = data.bookOfBusinessList || [];
        function toIso(dateStr) {
            if (!dateStr) return null;
            var m2 = dateStr.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
            return m2 ? (m2[3] + '-' + m2[1] + '-' + m2[2]) : dateStr;
        }
        var members = list.map(function(m) {
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

// Fallback: if the page already loaded and the BoB API call was already
// made (before our patches took effect), retry periodically.
window.addEventListener('load', function() {
    var attempts = 0;
    var retryIv = setInterval(function() {
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
