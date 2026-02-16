use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct CareSourcePortal;

const LOGIN_URL: &str = "https://caresource2.destinationrx.com/PC/Agent/Account/Login";

/// Intercept the DRX SPA's own fetch/XHR calls to capture the Bearer JWT
/// and agent GUID from requests to drxwebservices.com.
const INIT_SCRIPT: &str = r#"
(function() {
    const origFetch = window.fetch;
    window.fetch = function(resource, init) {
        try {
            const url = typeof resource === 'string' ? resource :
                         (resource instanceof Request ? resource.url : String(resource));
            if (url.includes('drxwebservices.com')) {
                const guidMatch = url.match(/\/Agent\/([0-9a-f-]{36})\//i);
                if (guidMatch) window.__sheeps_drx_agent_guid = guidMatch[1];
                const baseMatch = url.match(/(https:\/\/www\.drxwebservices\.com\/[^\/]+\/v\d+)/);
                if (baseMatch) window.__sheeps_drx_api_base = baseMatch[1];
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
                        window.__sheeps_drx_token = auth.substring(7);
                }
            }
        } catch (e) {}
        return origFetch.apply(this, arguments);
    };

    const origOpen = XMLHttpRequest.prototype.open;
    const origSetHeader = XMLHttpRequest.prototype.setRequestHeader;
    XMLHttpRequest.prototype.open = function(method, url) {
        this.__sheeps_url = typeof url === 'string' ? url : String(url);
        return origOpen.apply(this, arguments);
    };
    XMLHttpRequest.prototype.setRequestHeader = function(name, value) {
        try {
            const url = this.__sheeps_url || '';
            if (url.includes('drxwebservices.com')) {
                if (name.toLowerCase() === 'authorization' && value.startsWith('Bearer '))
                    window.__sheeps_drx_token = value.substring(7);
                const guidMatch = url.match(/\/Agent\/([0-9a-f-]{36})\//i);
                if (guidMatch) window.__sheeps_drx_agent_guid = guidMatch[1];
                const baseMatch = url.match(/(https:\/\/www\.drxwebservices\.com\/[^\/]+\/v\d+)/);
                if (baseMatch) window.__sheeps_drx_api_base = baseMatch[1];
            }
        } catch (e) {}
        return origSetHeader.apply(this, arguments);
    };
})();
"#;

/// Fetch all members by iterating 31-day date ranges from Oct 1 of the
/// previous year through today, deduplicating by memberID.
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        const token = window.__sheeps_drx_token;
        const agentGuid = window.__sheeps_drx_agent_guid;
        const apiBase = window.__sheeps_drx_api_base;

        if (!token || !agentGuid) {
            throw new Error(
                'Auth token or agent ID not captured yet. ' +
                'Navigate to the Reports page first so the app makes an API call, ' +
                'then click Sync Now again.'
            );
        }

        const base = apiBase || ('https://www.drxwebservices.com/spa' + new Date().getFullYear() + '/v1');
        const endpoint = base + '/Agent/' + agentGuid + '/MemberProfileSearch';

        // Build 31-day date ranges from Oct 1 of previous year through today
        const now = new Date();
        const ranges = [];
        let start = new Date(now.getFullYear() - 1, 9, 1); // Oct 1 prev year

        while (start < now) {
            let end = new Date(start);
            end.setDate(end.getDate() + 30);
            if (end > now) end = new Date(now);
            ranges.push({
                start: start.toISOString().replace(/T.*/, 'T00:00:00.000Z'),
                end:   end.toISOString().replace(/T.*/, 'T23:59:59.000Z')
            });
            start = new Date(start);
            start.setDate(start.getDate() + 31);
        }

        const allMembers = new Map(); // memberID -> PortalMember

        for (const range of ranges) {
            const resp = await fetch(endpoint, {
                method: 'POST',
                headers: {
                    'Authorization': 'Bearer ' + token,
                    'Content-Type': 'application/json',
                    'Accept': 'application/json, text/plain, */*'
                },
                body: JSON.stringify({
                    applicationStartDate: range.start,
                    applicationEndDate:   range.end,
                    enrollmentType: 'medicare',
                    agentReport: true
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

            const members = await resp.json();
            for (const m of members) {
                const id = m.memberID;
                if (id && !allMembers.has(id)) {
                    const enrollment = (m.enrollments && m.enrollments.length > 0)
                        ? m.enrollments[0] : null;
                    allMembers.set(id, {
                        first_name: m.firstName || '',
                        last_name:  m.lastName || '',
                        member_id:  id,
                        dob:        null,
                        plan_name:  enrollment ? enrollment.plan : null,
                        effective_date: enrollment ? enrollment.enrollmentDate : null,
                        end_date:   null,
                        status:     m.carrierStatus || null,
                        policy_status: null,
                        state: m.state || null,
                        city:  m.city || null,
                        phone: m.homePhone || null,
                        email: m.primaryEmailAddress || null
                    });
                }
            }
        }

        const result = Array.from(allMembers.values());
        window.location.href = 'http://sheeps-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(result));
    } catch (e) {
        window.location.href = 'http://sheeps-sync.localhost/error?message=' +
            encodeURIComponent(e.toString());
    }
})();
"#;

#[async_trait]
impl CarrierPortal for CareSourcePortal {
    fn carrier_id(&self) -> &str {
        "carrier-caresource"
    }

    fn carrier_name(&self) -> &str {
        "CareSource"
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
        Err(AppError::CarrierSync("CareSource reqwest fallback not implemented yet".into()))
    }
}
