use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct AnthemPortal;

const LOGIN_URL: &str = "https://brokerportal.anthem.com/apps/ptb/bob/CHHGRKJQNZ";

/// Intercept fetch/XHR to capture Bearer tokens and API base URLs
/// from Anthem broker portal requests.
const INIT_SCRIPT: &str = r#"
(function() {
    const origFetch = window.fetch;
    window.fetch = function(resource, init) {
        try {
            const url = typeof resource === 'string' ? resource :
                         (resource instanceof Request ? resource.url : String(resource));
            if (url.includes('ptb') || url.includes('bob') || url.includes('broker')) {
                const baseMatch = url.match(/(https:\/\/[^\/]+(?:\/[^\/]+)*\/(?:ptb|bob|broker)[^\/]*)/i);
                if (baseMatch) window.__compass_anthem_api_base = baseMatch[1];
                let headers = init && init.headers;
                if (!headers && resource instanceof Request) headers = resource.headers;
                if (headers) {
                    let auth;
                    if (headers instanceof Headers) {
                        auth = headers.get('Authorization');
                    } else if (Array.isArray(headers)) {
                        const e = headers.find(([k]) => k.toLowerCase() === 'authorization');
                        auth = e ? e[1] : null;
                    } else {
                        auth = headers['Authorization'] || headers['authorization'];
                    }
                    if (auth && auth.startsWith('Bearer '))
                        window.__compass_anthem_token = auth.substring(7);
                }
            }
        } catch (e) {}
        return origFetch.apply(this, arguments);
    };

    const origOpen = XMLHttpRequest.prototype.open;
    const origSetHeader = XMLHttpRequest.prototype.setRequestHeader;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__compass_url = typeof url === 'string' ? url : String(url);
        return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.setRequestHeader = function(name, value) {
        try {
            const url = this.__compass_url || '';
            if (url.includes('ptb') || url.includes('bob') || url.includes('broker')) {
                if (name.toLowerCase() === 'authorization' && value.startsWith('Bearer '))
                    window.__compass_anthem_token = value.substring(7);
                const baseMatch = url.match(/(https:\/\/[^\/]+(?:\/[^\/]+)*\/(?:ptb|bob|broker)[^\/]*)/i);
                if (baseMatch) window.__compass_anthem_api_base = baseMatch[1];
            }
        } catch (e) {}
        return origSetHeader.apply(this, arguments);
    };
})();
"#;

/// Scrape the Anthem Producer Toolbox "Book of Business" card layout.
/// Each member is a div.expandCard containing:
///   - Name in `.rowHeading h3 a`
///   - Fields in `.columnAndValue` divs with `.columnLabel` + `.columnValue`
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        // Helper: wait for a condition with timeout
        function waitFor(fn, ms) {
            return new Promise(function(resolve) {
                const start = Date.now();
                const iv = setInterval(function() {
                    const result = fn();
                    if (result) { clearInterval(iv); resolve(result); }
                    else if (Date.now() - start > ms) { clearInterval(iv); resolve(null); }
                }, 300);
            });
        }

        // Convert MM/DD/YYYY to YYYY-MM-DD
        function toIso(dateStr) {
            if (!dateStr) return null;
            var m = dateStr.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/);
            if (m) return m[3] + '-' + m[1].padStart(2, '0') + '-' + m[2].padStart(2, '0');
            m = dateStr.match(/^\d{4}-\d{2}-\d{2}$/);
            if (m) return dateStr;
            return dateStr;
        }

        // Parse "last, first M" into {first, last} with title case
        function parseName(nameStr) {
            if (!nameStr) return { first: '', last: '' };
            const commaIdx = nameStr.indexOf(',');
            if (commaIdx === -1) return { first: titleCase(nameStr.trim()), last: '' };
            const last = nameStr.substring(0, commaIdx).trim();
            const first = nameStr.substring(commaIdx + 1).trim();
            return { first: titleCase(first), last: titleCase(last) };
        }

        function titleCase(s) {
            if (!s) return s;
            return s.replace(/\w\S*/g, function(w) {
                return w.charAt(0).toUpperCase() + w.substr(1).toLowerCase();
            });
        }

        // Wait for .expandCard elements to appear
        const found = await waitFor(function() {
            var cards = document.querySelectorAll('.expandCard');
            return cards.length > 0 ? true : null;
        }, 15000);

        if (!found) {
            var dbg = {
                expandCards: document.querySelectorAll('.expandCard').length,
                rowConts: document.querySelectorAll('.row-cont').length,
                columnAndValues: document.querySelectorAll('.columnAndValue').length,
                bodyTextSample: document.body ? document.body.innerText.substring(0, 500) : 'none'
            };
            throw new Error(
                'Could not find member cards (.expandCard). Make sure you are logged in and on the Book of Business page. Debug: ' +
                JSON.stringify(dbg)
            );
        }

        // Check if all records are showing ("X of X Records")
        function allRecordsVisible() {
            var recText = document.body.innerText.match(/(\d+)\s+of\s+(\d+)\s+Records/i);
            if (recText) {
                return parseInt(recText[1]) >= parseInt(recText[2]);
            }
            return true;
        }

        // Scroll down to load more if the portal uses lazy loading
        if (!allRecordsVisible()) {
            for (var scroll = 0; scroll < 30; scroll++) {
                window.scrollTo(0, document.body.scrollHeight);
                await new Promise(function(r) { setTimeout(r, 1000); });
                if (allRecordsVisible()) break;
            }
        }

        // Scrape all .expandCard elements
        var cards = document.querySelectorAll('.expandCard');
        var allMembers = [];

        for (var i = 0; i < cards.length; i++) {
            var card = cards[i];

            // Get name from .rowHeading h3 a
            var nameEl = card.querySelector('.rowHeading h3 a') ||
                         card.querySelector('[data-test="rowHeading"] h3 a') ||
                         card.querySelector('h3 a');
            if (!nameEl) continue;
            var name = parseName(nameEl.textContent.trim());

            // Extract all label-value pairs from .columnAndValue divs
            var fieldMap = {};
            var pairs = card.querySelectorAll('.columnAndValue');
            for (var j = 0; j < pairs.length; j++) {
                var labelEl = pairs[j].querySelector('.columnLabel');
                var valueEl = pairs[j].querySelector('.columnValue') ||
                              pairs[j].querySelector('[data-test="detailsContent"]');
                if (labelEl && valueEl) {
                    var label = labelEl.textContent.trim();
                    var value = valueEl.textContent.trim();
                    if (label && value) {
                        fieldMap[label] = value;
                    }
                }
            }

            var status = (fieldMap['Status'] || 'active').toLowerCase();
            var isActive = status === 'active';

            allMembers.push({
                first_name: name.first,
                last_name: name.last,
                member_id: fieldMap['Client ID'] || null,
                dob: null,
                plan_name: fieldMap['Product(s)'] || null,
                effective_date: isActive ? toIso(fieldMap['Original Effective Date'] || fieldMap['Effective Date'] || null) : null,
                end_date: !isActive ? toIso(fieldMap['Cancellation Date'] || null) : null,
                status: status,
                policy_status: fieldMap['Bill Status'] || null,
                state: fieldMap['State'] || null,
                city: null,
                phone: null,
                email: null
            });
        }

        if (allMembers.length === 0) {
            var cardsFound = document.querySelectorAll('.expandCard').length;
            var dbg2 = {
                expandCardsFound: cardsFound,
                firstCardHtml: cardsFound > 0 ? document.querySelector('.expandCard').innerHTML.substring(0, 500) : 'none',
                columnAndValueCount: document.querySelectorAll('.columnAndValue').length,
                columnLabelCount: document.querySelectorAll('.columnLabel').length
            };
            throw new Error('Found cards but could not scrape members. Debug: ' + JSON.stringify(dbg2));
        }

        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(allMembers));
    } catch (e) {
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

    fn sync_instruction(&self) -> &str {
        "Log in, navigate to the Book of Business page, then click Sync Now."
    }

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("Anthem reqwest fallback not implemented yet".into()))
    }
}
