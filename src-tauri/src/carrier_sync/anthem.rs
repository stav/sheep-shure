use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct AnthemPortal;

const LOGIN_URL: &str = "https://brokerportal.anthem.com/apps/ptb/login";

/// Auto-login script: fills and submits the Anthem broker portal login form.
const AUTO_LOGIN_SCRIPT: &str = r#"
(function() {
    var TAG = '[Compass:Anthem]';
    console.log(TAG, 'Auto-login script loaded on', window.location.href);
    if (!window.__compass_creds) {
        console.warn(TAG, 'No credentials found, skipping auto-login');
        return;
    }
    console.log(TAG, 'Credentials present, will attempt auto-login');

    var attempt = 0;
    function tryLogin() {
        attempt++;
        // Look for password field with multiple selectors
        var passField = document.querySelector('input[type="password"]');
        if (!passField) {
            if (attempt <= 5 || attempt % 10 === 0) {
                console.log(TAG, 'Attempt', attempt, '- no password field found');
                console.log(TAG, '  All inputs:', Array.from(document.querySelectorAll('input')).map(function(el) {
                    return { type: el.type, name: el.name, id: el.id, placeholder: el.placeholder };
                }));
                console.log(TAG, '  Iframes:', document.querySelectorAll('iframe').length);
            }
            return false;
        }
        console.log(TAG, 'Found password field:', { name: passField.name, id: passField.id });

        var form = passField.closest('form');
        var userField = form
            ? form.querySelector('input[type="text"], input[type="email"], input[name*="user"], input[name*="login"], input[id*="user"], input[id*="login"]')
            : document.querySelector('input[type="text"], input[type="email"], input[name*="user"], input[name*="login"], input[id*="user"], input[id*="login"]');
        if (!userField) {
            console.warn(TAG, 'Password field found but no username field. Form inputs:', form ? Array.from(form.querySelectorAll('input')).map(function(el) {
                return { type: el.type, name: el.name, id: el.id };
            }) : 'no form');
            return false;
        }
        console.log(TAG, 'Found username field:', { name: userField.name, id: userField.id });

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
            console.log(TAG, 'Clicking submit button:', { tag: submit.tagName, type: submit.type, text: submit.textContent.trim().substring(0, 50) });
            submit.click();
            return true;
        }
        console.warn(TAG, 'Fields filled but no submit button found');
        return false;
    }
    // Delay before polling to let SSO redirects complete naturally
    setTimeout(function() {
        console.log(TAG, 'Starting login polling on', window.location.href);
        var iv = setInterval(function() { if (tryLogin()) { console.log(TAG, 'Login submitted!'); clearInterval(iv); } }, 500);
        setTimeout(function() { clearInterval(iv); console.warn(TAG, 'Gave up after 15s'); }, 15000);
    }, 2000);
})();
"#;

/// Intercept fetch/XHR to capture Bearer tokens, XSRF tokens, and API base
/// URLs from Anthem broker portal requests.
const INIT_SCRIPT: &str = r#"
(function() {
    var TAG = '[Compass:Anthem]';
    var origFetch = window.fetch;
    window.fetch = function(resource, init) {
        try {
            var url = typeof resource === 'string' ? resource :
                         (resource instanceof Request ? resource.url : String(resource));
            if (url.includes('ptb') || url.includes('bob') || url.includes('broker')) {
                var headers = init && init.headers;
                if (!headers && resource instanceof Request) headers = resource.headers;
                if (headers) {
                    var auth, xsrf;
                    if (headers instanceof Headers) {
                        auth = headers.get('Authorization');
                        xsrf = headers.get('X-XSRF-TOKEN');
                    } else if (Array.isArray(headers)) {
                        var ae = headers.find(function(h) { return h[0].toLowerCase() === 'authorization'; });
                        auth = ae ? ae[1] : null;
                        var xe = headers.find(function(h) { return h[0].toLowerCase() === 'x-xsrf-token'; });
                        xsrf = xe ? xe[1] : null;
                    } else {
                        auth = headers['Authorization'] || headers['authorization'];
                        xsrf = headers['X-XSRF-TOKEN'] || headers['x-xsrf-token'];
                    }
                    if (auth && auth.startsWith('Bearer ')) {
                        window.__compass_anthem_token = auth.substring(7);
                        console.log(TAG, 'Captured Bearer token from fetch:', url.substring(0, 80));
                    }
                    if (xsrf) {
                        window.__compass_anthem_xsrf = xsrf;
                        console.log(TAG, 'Captured XSRF token from fetch');
                    }
                }
            }
        } catch (e) {}
        return origFetch.apply(this, arguments);
    };

    var origOpen = XMLHttpRequest.prototype.open;
    var origSetHeader = XMLHttpRequest.prototype.setRequestHeader;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__compass_url = typeof url === 'string' ? url : String(url);
        return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.setRequestHeader = function(name, value) {
        try {
            var url = this.__compass_url || '';
            if (url.includes('ptb') || url.includes('bob') || url.includes('broker')) {
                if (name.toLowerCase() === 'authorization' && value.startsWith('Bearer ')) {
                    window.__compass_anthem_token = value.substring(7);
                    console.log(TAG, 'Captured Bearer token from XHR:', url.substring(0, 80));
                }
                if (name.toLowerCase() === 'x-xsrf-token') {
                    window.__compass_anthem_xsrf = value;
                    console.log(TAG, 'Captured XSRF token from XHR');
                }
            }
        } catch (e) {}
        return origSetHeader.apply(this, arguments);
    };
})();
"#;

/// Fetch Anthem Book of Business members via the portal REST API.
/// Uses the Bearer token and XSRF token captured by INIT_SCRIPT.
/// Paginates through all results using /apps/ptb/api/client/summary.
const FETCH_SCRIPT: &str = r#"
(async () => {
    var TAG = '[Compass:Anthem]';
    try {
        var token = window.__compass_anthem_token;
        var xsrf = window.__compass_anthem_xsrf;

        // Also try reading XSRF from cookie if not captured from headers
        if (!xsrf) {
            var match = document.cookie.match(/XSRF-TOKEN=([^;]+)/);
            if (match) xsrf = decodeURIComponent(match[1]);
        }

        if (!token) {
            throw new Error(
                'No Bearer token captured. Navigate to the Book of Business page first so ' +
                'the portal makes an authenticated API call, then click Sync Now again.'
            );
        }
        console.log(TAG, 'Fetching BoB via API. Token:', token.substring(0, 20) + '...', 'XSRF:', xsrf ? 'yes' : 'no');

        // Convert MM/DD/YYYY to YYYY-MM-DD
        function toIso(dateStr) {
            if (!dateStr) return null;
            var m = dateStr.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/);
            if (m) return m[3] + '-' + m[1].padStart(2, '0') + '-' + m[2].padStart(2, '0');
            return dateStr;
        }

        // Parse "LAST, FIRST M" into {first, last} with title case
        function parseName(nameStr) {
            if (!nameStr) return { first: '', last: '' };
            var commaIdx = nameStr.indexOf(',');
            if (commaIdx === -1) return { first: titleCase(nameStr.trim()), last: '' };
            var last = nameStr.substring(0, commaIdx).trim();
            var first = nameStr.substring(commaIdx + 1).trim();
            return { first: titleCase(first), last: titleCase(last) };
        }

        function titleCase(s) {
            if (!s) return s;
            return s.replace(/\w\S*/g, function(w) {
                return w.charAt(0).toUpperCase() + w.substr(1).toLowerCase();
            });
        }

        // Fetch one page of members
        async function fetchPage(pageNumber) {
            var url = '/apps/ptb/api/client/summary?pageNumber=' + pageNumber +
                      '&pageSize=100&sortBy=ClientName';
            var headers = {
                'Authorization': 'Bearer ' + token,
                'Content-Type': 'application/json',
                'Accept': 'application/json'
            };
            if (xsrf) headers['X-XSRF-TOKEN'] = xsrf;

            console.log(TAG, 'Fetching page', pageNumber, url);
            var resp = await fetch(url, { method: 'POST', headers: headers });
            if (!resp.ok) throw new Error('API returned ' + resp.status + ': ' + resp.statusText);
            return resp.json();
        }

        // Fetch all pages
        var allMembers = [];
        var page = 1;
        while (true) {
            var data = await fetchPage(page);
            var members = data.bookOfBusiness || [];
            console.log(TAG, 'Page', page, ':', members.length, 'members,',
                         data.metadata.page.totalElements, 'total');

            for (var i = 0; i < members.length; i++) {
                var m = members[i];
                var name = parseName(m.clientName);
                var status = (m.clientStatus || 'active').toLowerCase();
                var isActive = status === 'active';
                allMembers.push({
                    first_name: name.first,
                    last_name: name.last,
                    member_id: m.clientID || null,
                    dob: null,
                    plan_name: m.planType || m.productType || null,
                    effective_date: toIso(m.originalEffectiveDate || m.effectiveDate || null),
                    end_date: m.cancellationDate ? toIso(m.cancellationDate) : null,
                    status: status,
                    policy_status: null,
                    state: m.state || null,
                    city: null,
                    phone: null,
                    email: null
                });
            }

            var totalPages = data.metadata.page.totalPages;
            if (page >= totalPages) break;
            page++;
        }

        console.log(TAG, 'Total members fetched:', allMembers.length);
        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(allMembers));
    } catch (e) {
        console.error(TAG, 'Fetch error:', e);
        window.location.href = 'http://compass-sync.localhost/error?message=' +
            encodeURIComponent(e.toString());
    }
})();
"#;

#[async_trait]
impl CarrierPortal for AnthemPortal {
    fn carrier_id(&self) -> &str {
        "carrier-anthem"
    }

    fn carrier_name(&self) -> &str {
        "Anthem/Elevance"
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

    fn override_window_open(&self) -> bool {
        false
    }

    fn sync_instruction(&self) -> &str {
        "Log in, navigate to the Book of Business page, then click Sync Now."
    }

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("Anthem reqwest fallback not implemented yet".into()))
    }
}
