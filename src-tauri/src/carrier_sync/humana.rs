use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct HumanaPortal;

const LOGIN_URL: &str = "https://agentportal.humana.com/Vantage/apps/index.html?agenthome=-1#!/";

/// Scrape the Humana Vantage "My Humana Business" table at #!/businessCenter.
/// The table columns are:
///   0:Name  1:Type  2:Coverage Type  3:Plan Type  4:Sales Product
///   5:Soa Verification Code  6:Humana ID  7:Effective Date  8:Status
///   9:Status Reason  10:Status Description  11:Inactive Date  12:Plan Exit
///   13:Future Term Reason  14:Signature Date  15:Phone  16:Email
///   17:BirthDate  18:Deceased Date
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

        // Navigate to businessCenter if not already there
        if (!window.location.hash.includes('businessCenter')) {
            window.location.hash = '#!/businessCenter';
            await new Promise(function(r) { setTimeout(r, 3000); });
        }

        // Wait for the member table to appear
        const table = await waitFor(function() {
            const tables = document.querySelectorAll('table');
            for (const t of tables) {
                const ths = t.querySelectorAll('th');
                for (const th of ths) {
                    if (th.textContent.trim() === 'Humana ID') return t;
                }
            }
            return null;
        }, 10000);

        if (!table) {
            throw new Error('Could not find the member table. Make sure you are logged in and on the Business Center page.');
        }

        // Convert M/D/YYYY or MM/DD/YYYY to YYYY-MM-DD
        function toIso(dateStr) {
            if (!dateStr) return null;
            const m = dateStr.match(/^(\d{1,2})\/(\d{1,2})\/(\d{4})$/);
            if (!m) return dateStr;
            return m[3] + '-' + m[1].padStart(2, '0') + '-' + m[2].padStart(2, '0');
        }

        // Parse "LAST, FIRST M" into {first, last}
        function parseName(nameStr) {
            if (!nameStr) return { first: '', last: '' };
            const commaIdx = nameStr.indexOf(',');
            if (commaIdx === -1) return { first: nameStr.trim(), last: '' };
            const last = nameStr.substring(0, commaIdx).trim();
            const first = nameStr.substring(commaIdx + 1).trim();
            return { first: first, last: last };
        }

        // The Vantage grid uses split tables: one for headers, one for
        // the scrollable body.  Find both.
        function findHeaderTable() {
            const tables = document.querySelectorAll('table');
            for (const t of tables) {
                const ths = t.querySelectorAll('th');
                for (const th of ths) {
                    if (th.textContent.trim() === 'Humana ID') return t;
                }
            }
            return null;
        }

        function findBodyTable() {
            // Strategy 1: the body table is the other table with <td> rows
            const tables = document.querySelectorAll('table');
            const headerTbl = findHeaderTable();
            for (const t of tables) {
                if (t === headerTbl) continue;
                if (t.querySelectorAll('td').length > 0) return t;
            }
            // Strategy 2: if the header table itself has tbody rows, use it
            if (headerTbl && headerTbl.querySelectorAll('tbody tr td').length > 0) {
                return headerTbl;
            }
            return null;
        }

        // Build column index map from the header table
        function buildColMap() {
            const tbl = findHeaderTable();
            if (!tbl) return {};
            const ths = tbl.querySelectorAll('th');
            const map = {};
            ths.forEach(function(th, idx) {
                map[th.textContent.trim()] = idx;
            });
            return map;
        }

        // Scrape rows from the body table
        function scrapeRows() {
            const bodyTbl = findBodyTable();
            if (!bodyTbl) return [];
            const colMap = buildColMap();
            var rows = bodyTbl.querySelectorAll('tbody tr');
            if (rows.length === 0) rows = bodyTbl.querySelectorAll('tr');
            const members = [];
            for (const row of rows) {
                const cells = row.querySelectorAll('td');
                if (cells.length < 6) continue;

                function cell(name) {
                    const idx = colMap[name];
                    if (idx === undefined || idx >= cells.length) return null;
                    const text = cells[idx].textContent.trim();
                    return text || null;
                }

                const rawName = cell('Name');
                if (!rawName) continue;
                const name = parseName(rawName);
                const planType = cell('Plan Type') || '';
                const salesProduct = cell('Sales Product') || '';
                const planName = [planType, salesProduct].filter(Boolean).join(' - ');
                const status = cell('Status') || cell('Status Reason') || 'Active';
                const phone = cell('Phone');

                members.push({
                    first_name: name.first,
                    last_name: name.last,
                    member_id: cell('Humana ID'),
                    dob: toIso(cell('BirthDate')),
                    plan_name: planName || null,
                    effective_date: toIso(cell('Effective Date')),
                    end_date: toIso(cell('Inactive Date')),
                    status: status,
                    policy_status: null,
                    state: null,
                    city: null,
                    phone: (phone && phone !== 'Unavailable') ? phone : null,
                    email: cell('Email')
                });
            }
            return members;
        }

        // Click "View all customers" to remove filters if present
        const allLinks = document.querySelectorAll('a');
        for (const a of allLinks) {
            if (a.textContent.trim().toLowerCase() === 'view all customers') {
                a.click();
                // Wait for Angular to re-render the grid
                await new Promise(function(r) { setTimeout(r, 3000); });
                await waitFor(findBodyTable, 5000);
                break;
            }
        }

        // Collect all pages
        var allMembers = [];
        var maxPages = 50;

        for (var page = 0; page < maxPages; page++) {
            const pageMembers = scrapeRows();
            allMembers = allMembers.concat(pageMembers);

            // Look for next-page button
            const buttons = document.querySelectorAll('button, a');
            var nextBtn = null;
            for (const btn of buttons) {
                const text = btn.textContent.trim();
                const aria = (btn.getAttribute('aria-label') || '').toLowerCase();
                const title = (btn.getAttribute('title') || '').toLowerCase();
                if ((text === '>' || text === '\u203A' || text === '\u00BB' ||
                     aria.includes('next') || title.includes('next')) &&
                    !btn.disabled &&
                    btn.offsetParent !== null) {
                    nextBtn = btn;
                    break;
                }
            }

            if (!nextBtn) break;

            // Check if we're on the last page by looking at pagination text
            var isLastPage = false;
            const leafNodes = document.querySelectorAll('*');
            for (const el of leafNodes) {
                if (el.children.length === 0) {
                    const t = el.textContent.trim();
                    const m = t.match(/(\d+)\s*[-\u2013]\s*(\d+)\s+of\s+(\d+)/i);
                    if (m && parseInt(m[2]) >= parseInt(m[3])) {
                        isLastPage = true;
                        break;
                    }
                }
            }
            if (isLastPage) break;

            nextBtn.click();
            await new Promise(function(r) { setTimeout(r, 1500); });
        }

        if (allMembers.length === 0) {
            // Debug: report what we see so we can diagnose
            const hdrTbl = findHeaderTable();
            const bdyTbl = findBodyTable();
            const dbg = {
                headerTableFound: !!hdrTbl,
                bodyTableFound: !!bdyTbl,
                bodyRowCount: bdyTbl ? bdyTbl.querySelectorAll('tr').length : 0,
                bodyTbodyRowCount: bdyTbl ? bdyTbl.querySelectorAll('tbody tr').length : 0,
                bodyTdCount: bdyTbl ? bdyTbl.querySelectorAll('td').length : 0,
                headers: hdrTbl ? Array.from(hdrTbl.querySelectorAll('th')).map(function(th) { return th.textContent.trim(); }) : [],
                firstBodyRowHtml: bdyTbl && bdyTbl.querySelector('tr') ? bdyTbl.querySelector('tr').innerHTML.substring(0, 500) : 'none',
                allTablesCount: document.querySelectorAll('table').length
            };
            throw new Error('No members found. Debug: ' + JSON.stringify(dbg));
        }

        window.location.href = 'http://sheeps-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(allMembers));
    } catch (e) {
        window.location.href = 'http://sheeps-sync.localhost/error?message=' +
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

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("Humana reqwest fallback not implemented yet".into()))
    }
}
