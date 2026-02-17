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
| `src-tauri/src/carrier_sync/devoted.rs` | Devoted Health — [docs](carriers/devoted-health.md) |
| `src-tauri/src/carrier_sync/caresource.rs` | CareSource — [docs](carriers/caresource.md) |
| `src-tauri/src/carrier_sync/medmutual.rs` | Medical Mutual — [docs](carriers/medical-mutual.md) |
| `src-tauri/src/carrier_sync/uhc.rs` | UnitedHealthcare — [docs](carriers/unitedhealthcare.md) |
| `src-tauri/src/carrier_sync/humana.rs` | Humana — [docs](carriers/humana.md) |
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

## Carrier Implementations

Each carrier has a detailed doc in `docs/carriers/`:

| Carrier | Approach | Key Technique | Docs |
|---------|----------|---------------|------|
| Devoted Health | GraphQL API | Persisted queries, CSRF token fetch | [devoted-health.md](carriers/devoted-health.md) |
| CareSource | REST API | Init script token capture, 31-day date windowing | [caresource.md](carriers/caresource.md) |
| Medical Mutual | HTML scraping | Server-rendered `#member-table`, `DOMParser` | [medical-mutual.md](carriers/medical-mutual.md) |
| UnitedHealthcare | REST API | Multi-stage partyID fallback, deep storage search | [unitedhealthcare.md](carriers/unitedhealthcare.md) |
| Humana | DOM table scraping | Split-table grid, live DOM, pagination | [humana.md](carriers/humana.md) |

### Approach Summary

Three distinct patterns emerged across the 5 implementations:

1. **API calls** (Devoted, CareSource, UHC): Call the same REST/GraphQL APIs the portal SPA uses, leveraging the webview's authenticated session. Best for SPAs with clean APIs.
2. **Server-rendered HTML** (Medical Mutual): Fetch the HTML page via AJAX and parse with `DOMParser`. Best for traditional server-rendered sites.
3. **Live DOM scraping** (Humana): Read data directly from the rendered page's DOM. Needed when the data is loaded before any script injection and no API can be identified.

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

Ranked by automation difficulty based on auth complexity, anti-bot measures, and portal architecture. Actual difficulty notes reflect real implementation experience where available.

| Rank | Carrier | Difficulty | Notes | Status |
|------|---------|-----------|-------|--------|
| 1 | Devoted Health | Easiest | React SPA, GraphQL persisted queries, no anti-bot. Clean API, straightforward CSRF token flow. | **Done** |
| 2 | Medical Mutual of Ohio | Easy | Server-rendered HTML with semantic `data-col-name` attrs. No API, no JS framework, no auth complexity. Simplest implementation. | **Done** |
| 3 | CareSource | Easy | DRX SPA, cross-origin REST API. Needed init script for Bearer JWT capture. 31-day date window limit required windowed iteration. | **Done** |
| 4 | UnitedHealthcare | Easy-Mod | Angular SPA (Jarvis), REST API. Required multi-stage fallback for partyID capture (init script race condition with WebKit2GTK). Direct API fallback solved it. | **Done** |
| 5 | Humana | Moderate | Angular SPA (Vantage), split-table grid (separate header/body tables). No API discoverable — had to scrape live DOM. Required 5 iterations to handle grid quirks. | **Done** |
| 6 | SummaCare | Easy | CMS/Sitecore, NPN login, no bot detection at all | -- |
| 7 | Alignment Healthcare | Easy | React SPA, Azure AD B2C OAuth2, no anti-bot | -- |
| 8 | Zing Health | Easy-Mod | EvolveNXT platform, reCAPTCHA, email/password + security Q's | -- |
| 9 | Cigna | Easy-Mod | Low anti-bot, agent-number login, multi-format downloads | -- |
| 10 | Molina (EvolveNXT) | Moderate | jQuery server-rendered, reCAPTCHA v3, Excel export | -- |
| 11 | Anthem BCBS (Producer Toolbox) | Moderate | SPA + Akamai, partner API exists | -- |
| 12 | Mutual of Omaha | Mod-Hard | WebSphere, Symantec VIP MFA | -- |
| 13 | WellCare/Centene | Mod-Hard | PingOne SSO | -- |
| 14 | Aetna (Producer World) | Hard | Imperva Incapsula WAF, MFA (Acceptto push/SMS), enterprise Java | -- |
| 15 | BCBS (other) | Hard | Fragmented per-state, Akamai | -- |
| 16 | Kaiser Permanente | Hardest | PingFederate + MFA, per-state | -- |

### Portal URLs

| Carrier | Portal URL | Status |
|---------|-----------|--------|
| Devoted Health | <https://agent.devoted.com/> | Done |
| CareSource | <https://caresource2.destinationrx.com/PC/Agent/Account/Login> | Done |
| Medical Mutual of Ohio | <https://mybrokerlink.com/> | Done |
| UnitedHealthcare | <https://www.uhcjarvis.com/content/jarvis/en/secure/book-of-business-search.html> | Done |
| Humana | <https://agentportal.humana.com/Vantage/apps/index.html?agenthome=-1#!/> | Done |
| SummaCare | <https://www.summacare.com/brokerstorehome> | -- |
| Alignment Healthcare | TBD | -- |
| Zing Health | <https://zing.sb.evolvenxt.com/> | -- |
| Anthem BCBS | TBD (Producer Toolbox) | -- |
| Aetna | <https://www.aetna.com/producer_public/login.fcc> | -- |
