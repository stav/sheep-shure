# Carrier Portal Sync

Verify your book of business against carrier portals and auto-update enrollment statuses when mismatches are found.

## How It Works

1. **Open Portal Login** -- opens a Tauri webview to the carrier's agent portal
2. User logs in normally (handles MFA/CAPTCHA naturally since it's a real browser)
3. **Sync Now** -- injects JavaScript into the webview that:
   - Fetches member data using the browser's own session cookies/tokens
   - Approach varies by carrier: GraphQL API, REST API, or DOM scraping
   - Navigates to `sheeps-sync.localhost/data?members=<json>` on success
4. The Rust `on_navigation` handler intercepts that URL and emits a Tauri event
5. The frontend receives the event and calls `process_portal_members`
6. The sync service compares portal members against local enrollments:
   - **Matched**: portal member found in local DB (name + DOB, or name-only fallback)
   - **Disenrolled**: local enrollment NOT found in portal -- auto-updated to `DISENROLLED`
   - **New in portal**: portal member with no local match -- shown for informational purposes

## Architecture

```
Webview (carrier portal)
  │
  ├─ init_script()    -- runs at document-start (optional, per-carrier)
  └─ fetch_script()   -- injected when user clicks "Sync Now"
        │
        ├─ fetches data using browser's own cookies
        └─ navigates to sheeps-sync.localhost/data?members=<json>
              │
              └─ on_navigation handler (Rust)
                    │
                    └─ emits "carrier-sync-data" Tauri event
                          │
                          └─ Frontend calls process_portal_members command
                                │
                                └─ carrier_sync_service::run_sync()
                                      ├─ compares portal vs local enrollments
                                      ├─ auto-disenrolls unmatched
                                      └─ logs to carrier_sync_logs table
```

### Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/carrier_sync/mod.rs` | `CarrierPortal` trait + carrier registry |
| `src-tauri/src/carrier_sync/devoted.rs` | Devoted Health implementation |
| `src-tauri/src/carrier_sync/caresource.rs` | CareSource implementation |
| `src-tauri/src/carrier_sync/medmutual.rs` | Medical Mutual of Ohio implementation |
| `src-tauri/src/commands/carrier_sync_commands.rs` | Tauri IPC commands |
| `src-tauri/src/services/carrier_sync_service.rs` | Comparison logic, auto-disenrollment |
| `src-tauri/src/models/carrier_sync.rs` | `PortalMember`, `SyncResult`, `SyncLogEntry` |
| `src-tauri/src/db/migrations/v003_carrier_sync.sql` | `carrier_sync_logs` table |
| `src/features/carrier-sync/CarrierSyncPage.tsx` | Sync UI with Tauri event listeners |
| `src/hooks/useCarrierSync.ts` | TanStack Query hooks |

### The `CarrierPortal` Trait

Each carrier implements this trait:

```rust
pub trait CarrierPortal: Send + Sync {
    fn carrier_id(&self) -> &str;       // e.g. "carrier-devoted"
    fn carrier_name(&self) -> &str;     // e.g. "Devoted Health"
    fn login_url(&self) -> &str;        // portal login URL
    fn init_script(&self) -> &str;      // optional JS at document-start
    fn fetch_script(&self) -> &str;     // JS to fetch member data
    async fn fetch_members(&self, cookies: &str) -> Result<Vec<PortalMember>, AppError>;
}
```

New carriers are registered in `get_portal()` in `mod.rs`.

### Matching Strategy

Portal members are matched to local enrollments by:

1. **Name + DOB** (case-insensitive, strongest match)
2. **Name only** (fallback when DOB is unavailable)

MBI matching is not used because carrier portal member IDs are internal UUIDs, not MBIs.

## Devoted Health Implementation

- **Portal**: `https://agent.devoted.com/`
- **Framework**: React SPA ("Orinoco") with GraphQL API
- **Auth**: Auth0 SSO, session stored in HttpOnly `devoted_session` JWT cookie
- **CSRF**: Fetched via `CSRFToken` GraphQL persisted query, then sent as `x-csrf-token` header
- **Required headers**: `x-orinoco-portal: Agents`, `x-orinoco-client-version: <from window.__orinoco_config>`
- **Member query**: `ListBookOfBusinessContacts` persisted query, page-based pagination (100/page)
- **Persisted query hashes**:
  - `CSRFToken`: `0ba70438537351c55da05b9cec107834cf0e6e1126b9107bb382cba283d9dc5a`
  - `ListBookOfBusinessContacts`: `881c07f52080a6a6a04c653b03fa4520acfd30de90ab0ac6ca4caa161f6bbc95`

### Member fields extracted

`first_name`, `last_name`, `member_id`, `birth_date`, `status`, `aor_policy_status`, `current_pbp` (plan name, start/end dates), `state`, `city`, `primary_phone`, `email`

## CareSource Implementation

- **Portal**: `https://caresource2.destinationrx.com/PC/Agent/Account/Login`
- **Platform**: DestinationRx (DRX) SPA
- **API**: REST on `https://www.drxwebservices.com/spa{year}/v1/` (cross-origin)
- **Auth**: Bearer JWT token (not cookie-based), cross-origin to `drxwebservices.com`
- **CSRF**: None required
- **init_script**: Monkey-patches `fetch` and `XMLHttpRequest` to capture the Bearer JWT and agent GUID from the SPA's own API calls to `drxwebservices.com`
- **Member endpoint**: `POST /Agent/{agentGUID}/MemberProfileSearch`
- **Date range limit**: 31-day window per request -- fetch_script iterates from Oct 1 of previous year through today in 31-day chunks, deduplicating by `memberID`

### Request body

```json
{
    "applicationStartDate": "2026-01-01T00:00:00.000Z",
    "applicationEndDate": "2026-01-31T23:59:59.000Z",
    "enrollmentType": "medicare",
    "agentReport": true
}
```

### Member fields extracted

`firstName`, `lastName`, `memberID`, `enrollments[].plan`, `enrollments[].enrollmentDate`, `carrierStatus`, `state`, `city`, `homePhone`, `primaryEmailAddress`

Note: DOB is not available from this endpoint.

## Medical Mutual of Ohio Implementation

- **Portal**: `https://mybrokerlink.com/`
- **Platform**: Sitecore CMS, server-rendered HTML
- **Auth**: Session cookies (same-origin, no interception needed)
- **CSRF**: None required
- **Approach**: Pure DOM scraping -- no API calls, no init_script
- **fetch_script**: Fetches `/mybusiness/bookofbusiness` via AJAX, parses the `#member-table` HTML table using `DOMParser`, extracts data from `td[data-col-name="..."]` selectors
- **Table ID**: `#member-table` with `data-col-name` attributes on each `<td>`

### Member fields extracted

`Name`, `GroupNumber`, `DateOfBirth`, `MarketSegment`, `EffectiveDate`, `Attention` (status), `State`, `City`, `Phone`, `Email`

Dates are converted from `MM/DD/YYYY` to `YYYY-MM-DD` (ISO) for matching.

## Adding a New Carrier

1. Create `src-tauri/src/carrier_sync/<carrier>.rs` implementing `CarrierPortal`
2. Add `pub mod <carrier>;` to `carrier_sync/mod.rs`
3. Register in `get_portal()` match arm
4. Add the carrier to the `CARRIERS` array in `CarrierSyncPage.tsx` (change status from `coming_soon` to `available`)
5. Match the `carrier_id` to the seed data in `db/seed.rs` (e.g. `carrier-alignment`)

### Reverse-Engineering a New Carrier Portal

1. Open the portal in a regular browser, log in
2. Open DevTools > Network tab
3. Navigate to the Book of Business / member list page
4. Look for the API calls that load member data
5. Note: endpoint URL, auth mechanism (cookies, headers), request/response shape
6. Check for CSRF -- look for `csrf` in cookies, headers, or dedicated API endpoints
7. Write the `fetch_script` JS that replicates those API calls from within the webview
8. The webview handles auth naturally (cookies are already present after login)

### Tips from Implementations

- **HttpOnly cookies**: JS can't read them, but `fetch()` from the webview sends them automatically
- **CSRF tokens**: May need a dedicated API call to obtain (Devoted uses a `CSRFToken` GraphQL query)
- **Custom headers**: Carrier frameworks often require app-specific headers (e.g. `x-orinoco-portal`)
- **HAR files**: Export from DevTools Network tab -- invaluable for seeing exact request/response patterns
- **`sheeps-sync.localhost` callback**: The JS navigates to this fake URL to pass data back to Rust; `on_navigation` intercepts it
- **Cross-origin APIs**: Some portals (CareSource/DRX) use a different domain for the API. Use `init_script` to monkey-patch `fetch`/`XHR` and capture Bearer tokens from the SPA's own calls
- **Date range limits**: Some APIs limit query windows (CareSource: 31 days). Iterate through multiple date ranges and deduplicate by member ID
- **Server-rendered HTML**: The simplest portals (Medical Mutual) render everything in HTML. Use `DOMParser` to parse the page and extract data from the DOM -- no API interception needed
- **Seed data**: New carriers must exist in `carriers` table. `seed_data()` runs on both create and unlock (INSERT OR IGNORE), so new carriers are added on next login

## Carrier Difficulty Rankings

Ranked by automation difficulty based on auth complexity and anti-bot measures:

| Rank | Carrier | Difficulty | Notes | Status |
|------|---------|-----------|-------|--------|
| 1 | Devoted Health | Easiest | React SPA ("Orinoco"), GraphQL API, no anti-bot | **Done** |
| 2 | CareSource | Easy | DestinationRx SPA, cross-origin REST API, Bearer JWT | **Done** |
| 3 | Medical Mutual of Ohio | Easy | MyBrokerLink, Sitecore, server-rendered HTML table | **Done** |
| 4 | SummaCare | Easy | CMS/Sitecore, NPN login, no bot detection at all | -- |
| 5 | Alignment Healthcare | Easy | React SPA, Azure AD B2C OAuth2, no anti-bot | -- |
| 6 | Zing Health | Easy-Mod | EvolveNXT platform, reCAPTCHA, email/password + security Q's | -- |
| 7 | UnitedHealthcare (Jarvis) | Easy-Mod | Angular SPA, REST APIs, OAuth PKCE, Excel export | -- |
| 8 | Cigna | Easy-Mod | Low anti-bot, agent-number login, multi-format downloads | -- |
| 9 | Molina (EvolveNXT) | Moderate | jQuery server-rendered, reCAPTCHA v3, Excel export | -- |
| 10 | Humana (Vantage) | Moderate | 2FA w/ device-save, proven scrapable | -- |
| 11 | Anthem BCBS (Producer Toolbox) | Moderate | SPA + Akamai, partner API exists | -- |
| 12 | Mutual of Omaha | Mod-Hard | WebSphere, Symantec VIP MFA | -- |
| 13 | WellCare/Centene | Mod-Hard | PingOne SSO | -- |
| 14 | Aetna (Producer World) | Hard | Imperva Incapsula WAF, MFA (Acceptto push/SMS), enterprise Java | -- |
| 15 | BCBS (other) | Hard | Fragmented per-state, Akamai | -- |
| 16 | Kaiser Permanente | Hardest | PingFederate + MFA, per-state | -- |

### Portal URLs

| Carrier | Portal URL |
|---------|-----------|
| Devoted Health | <https://agent.devoted.com/> |
| CareSource | <https://caresource2.destinationrx.com/PC/Agent/Account/Login> |
| Medical Mutual of Ohio | <https://mybrokerlink.com/> |
| SummaCare | <https://www.summacare.com/brokerstorehome> |
| Alignment Healthcare | TBD |
| Zing Health | <https://zing.sb.evolvenxt.com/> |
| UnitedHealthcare | TBD (Jarvis portal) |
| Humana | TBD (Vantage portal) |
| Anthem BCBS | TBD (Producer Toolbox) |
| Aetna | <https://www.aetna.com/producer_public/login.fcc> |
