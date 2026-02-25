use async_trait::async_trait;

use crate::error::AppError;
use crate::models::PortalMember;

use super::CarrierPortal;

pub struct CareSourcePortal;

const LOGIN_URL: &str = "https://acprodcmsl-prod-producerportal-approuter.cfapps.us10.hana.ondemand.com/cp.portal/site";

/// Scrape the Active Book of Business from the SAP UI5 producer portal.
///
/// The portal renders a sap.m.Table (class sapMListTbl / sapMList) with:
///   - Main row cells (class sapMListTblCell) for the visible columns:
///     0: Producer Id, 1: Producer Name, 2: Subscriber Number,
///     3: Subscriber Name, 4: DOB, 5: Policy State, 6: ZIP, 7: Phone, 8: Email
///   - A "popin" detail container (class sapMListTblSubCnt) in the next sibling
///     <tr class="sapMListTblSubRow">, containing sapMListTblSubCntRow elements
///     with labels (sapMLabel, title attr has clean text) and values (sapMText).
const FETCH_SCRIPT: &str = r#"
(async () => {
    try {
        // Search for the SAP UI5 table in the document and all same-origin iframes
        function findTable(doc) {
            // SAP responsive table
            const list = doc.querySelector('.sapMList');
            if (list) return list;
            // Fallback: any element containing the BOB data
            for (const t of doc.querySelectorAll('table')) {
                const text = t.textContent || '';
                if (/subscriber/i.test(text) && /producer/i.test(text)) return t;
            }
            return null;
        }

        let container = findTable(document);
        let targetDoc = document;

        if (!container) {
            for (const iframe of document.querySelectorAll('iframe')) {
                try {
                    const iDoc = iframe.contentDocument || iframe.contentWindow.document;
                    if (iDoc) {
                        container = findTable(iDoc);
                        if (container) { targetDoc = iDoc; break; }
                    }
                } catch (e) {}
            }
        }

        if (!container) {
            throw new Error(
                'Book of Business table not found. ' +
                'Click "Active Book of Business", wait for data to load, then click Sync Now.'
            );
        }

        // Find main data rows: sapMLIB = SAP ListItemBase (each table row)
        const items = container.querySelectorAll('.sapMLIB');
        const members = [];

        const titleCase = s => s.split(/\s+/)
            .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
            .join(' ');

        const parseDate = raw => {
            if (!raw) return null;
            const m = raw.match(/(\d{2})\/(\d{2})\/(\d{4})/);
            return m ? m[3] + '-' + m[1] + '-' + m[2] : null;
        };

        for (const item of items) {
            // Main cells have class sapMListTblCell
            const cells = [...item.querySelectorAll('.sapMListTblCell')];
            if (cells.length < 7) continue;

            const t = cells.map(c => c.textContent.trim());

            // Validate data row: Producer Id (digits), Subscriber Number (digits), DOB (date)
            if (!/^\d+$/.test(t[0]) || !/^\d+$/.test(t[2])) continue;
            if (!/\d{2}\/\d{2}\/\d{4}/.test(t[4])) continue;

            // Parse subscriber name: "LASTNAME, FIRSTNAME"
            const nameRaw = t[3];
            const cp = nameRaw.indexOf(',');
            let first = '', last = '';
            if (cp > 0) {
                last = nameRaw.substring(0, cp).trim();
                first = nameRaw.substring(cp + 1).trim();
            } else {
                last = nameRaw;
            }

            const stateM = t[5].match(/\((\w{2})\)/);
            const phone = (t[7] || '').trim() || null;
            const emailRaw = (t[8] || '').trim();
            const email = (emailRaw && emailRaw.includes('@')) ? emailRaw : null;

            // Find the popin/detail container for this row
            // In block mode it's in the next sibling <tr class="sapMListTblSubRow">
            let popin = null;
            if (item.nextElementSibling) {
                popin = item.nextElementSibling.querySelector('.sapMListTblSubCnt');
            }
            // Fallback: by SAP id convention (item-id + '-subcont')
            if (!popin && item.id) {
                popin = targetDoc.getElementById(item.id + '-subcont');
            }

            // Extract detail fields from the popin using SAP UI5 label/value structure
            const fields = {};
            if (popin) {
                for (const row of popin.querySelectorAll('.sapMListTblSubCntRow')) {
                    const label = row.querySelector('.sapMLabel');
                    const valEl = row.querySelector('.sapMListTblSubCntVal .sapMText')
                               || row.querySelector('.sapMListTblSubCntVal .sapMLnk');
                    if (!label) continue;
                    // title attribute has clean text without soft hyphens
                    const key = (label.getAttribute('title') || label.textContent)
                        .replace(/\u00AD/g, '').trim().toLowerCase();
                    const val = valEl ? valEl.textContent.trim() : '';
                    if (val) fields[key] = val;
                }
            }

            members.push({
                first_name: titleCase(first),
                last_name: titleCase(last),
                member_id: t[2],
                dob: parseDate(t[4]),
                plan_name: fields['product'] || null,
                effective_date: parseDate(fields['effective date']),
                end_date: parseDate(fields['termination date']),
                status: 'active',
                policy_status: null,
                state: stateM ? stateM[1] : null,
                city: null,
                phone: phone,
                email: email,
            });
        }

        if (members.length === 0) {
            throw new Error(
                'No members found in the table. ' +
                'Navigate to Active Book of Business, wait for data to load, then click Sync Now.'
            );
        }

        window.location.href = 'http://compass-sync.localhost/data?members=' +
            encodeURIComponent(JSON.stringify(members));
    } catch (e) {
        window.location.href = 'http://compass-sync.localhost/error?message=' +
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

    fn fetch_script(&self) -> &str {
        FETCH_SCRIPT
    }

    async fn fetch_members(&self, _cookies: &str) -> Result<Vec<PortalMember>, AppError> {
        Err(AppError::CarrierSync("CareSource reqwest fallback not implemented yet".into()))
    }
}
