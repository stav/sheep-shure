# Humana — Carrier Sync

**Status**: Done
**Difficulty**: Moderate
**Carrier ID**: `carrier-humana`
**Portal URL**: https://agentportal.humana.com/Vantage/apps/index.html?agenthome=-1#!/
**Source**: `src-tauri/src/carrier_sync/humana.rs`

## Portal Overview

Humana's agent portal ("Vantage") is an Angular SPA using hash-based routing (`#!/`). The Book of Business is at `#!/businessCenter` under the page title "My Humana Business". The portal uses a component grid library that renders its header and body as **two separate HTML `<table>` elements** — a split-table pattern common in Kendo UI and similar Angular grid components.

## Approach: Live DOM Table Scraping with Split-Table Handling

**Why this approach**: The Vantage portal renders the member data in an HTML table grid, but the data is loaded before any fetch script runs and no identifiable BoB API endpoint was discovered during discovery mode. The grid component also uses a split-table architecture (separate `<table>` for headers vs data), making standard table scraping assumptions fail. Direct DOM scraping from the live page was the only reliable approach.

### Auth Mechanism

- **Session cookies**: Same-origin, handled automatically
- **Angular hash routing**: SPA navigation via `#!/` fragments
- **CSRF**: None required for DOM scraping (no API calls made)

### Init Script

None needed. We scrape the live DOM rather than calling APIs, so no token/header interception is required.

### Fetch Script Flow

1. **Navigate**: If not already at `#!/businessCenter`, set `window.location.hash`
2. **Wait for table**: Poll for a `<table>` element containing a `<th>` with text "Humana ID" (up to 10s)
3. **Click "View all customers"**: Removes any active filters, then wait 3s + re-verify table exists (Angular re-renders the entire grid)
4. **Find split tables**:
   - `findHeaderTable()` — the `<table>` containing `<th>Humana ID</th>` (headers only)
   - `findBodyTable()` — the *other* `<table>` containing `<td>` elements (data rows)
5. **Build column map** from header table `<th>` elements
6. **Scrape rows** from body table, mapping by column index
7. **Paginate**: Click the `>` next-page button, check "X - Y of Z" pagination text for last page
8. **Navigate** to `sheeps-sync.localhost/data` with collected members

### Split-Table Grid Architecture

The Vantage grid renders as:

```
<div class="grid-container">
    <table>              ← Header table
        <thead>
            <tr><th>Name</th><th>Type</th>...<th>Humana ID</th>...</tr>
        </thead>
    </table>
    <table>              ← Body table (scrollable)
        <tbody>
            <tr><td>GILLETTE, MARY E</td><td>Policy</td>...</tr>
            <tr><td>KERNS, ROGER D</td><td>Policy</td>...</tr>
        </tbody>
    </table>
</div>
```

The scraper uses `findHeaderTable()` for column mapping and `findBodyTable()` for row data. After "View all customers" is clicked, Angular re-renders both tables, so the scraper re-queries the DOM fresh on every call (no stale references).

### Table Columns (19 total)

| Index | Column | Mapped to |
|-------|--------|-----------|
| 0 | Name | `first_name`, `last_name` (parsed from "LAST, FIRST M") |
| 1 | Type | *(not mapped)* |
| 2 | Coverage Type | *(not mapped)* |
| 3 | Plan Type | `plan_name` (combined with Sales Product) |
| 4 | Sales Product | `plan_name` (e.g., "MA - LPPO HNR") |
| 5 | Soa Verification Code | *(not mapped)* |
| 6 | Humana ID | `member_id` |
| 7 | Effective Date | `effective_date` (M/D/YYYY → YYYY-MM-DD) |
| 8 | Status | `status` |
| 9 | Status Reason | `status` (fallback) |
| 10 | Status Description | *(not mapped)* |
| 11 | Inactive Date | `end_date` (M/D/YYYY → YYYY-MM-DD) |
| 12 | Plan Exit | *(not mapped)* |
| 13 | Future Term Reason | *(not mapped)* |
| 14 | Signature Date | *(not mapped)* |
| 15 | Phone | `phone` (filtered: "Unavailable" → null) |
| 16 | Email | `email` |
| 17 | BirthDate | `dob` (M/D/YYYY → YYYY-MM-DD) |
| 18 | Deceased Date | *(not mapped)* |

### Name Parsing

Names are in `LAST, FIRST MIDDLE` format. Parsed by splitting on the first comma:
- Before comma → `last_name` (e.g., "GILLETTE")
- After comma → `first_name` (e.g., "MARY E")

### Pagination

The grid shows "1 - 6 of 6 items" style pagination. The script:
1. Scrapes current page rows
2. Looks for a `>` / `›` / `»` button (or `aria-label="next"`)
3. Checks the "X - Y of Z" text — if Y >= Z, we're on the last page
4. Clicks next and waits 1.5s for the grid to update
5. Maximum 50 pages as a safety limit

## Challenges Encountered

### 1. Wrong Portal URL (Blank Page)
**Problem**: Initial implementation used `https://agent.humana.com/` as the login URL. This returned an empty HTML page (Content-Length: 0).
**Fix**: User provided the correct Vantage URL: `https://agentportal.humana.com/Vantage/apps/index.html?agenthome=-1#!/`

### 2. Discovery Captured Dashboard APIs Only
**Problem**: First discovery mode run captured only dashboard APIs (user-profile, notifications, licensing, etc.) — no BoB-related endpoints.
**Reason**: The user was on the dashboard page, not the Business Center.
**Fix**: Updated discovery script to auto-navigate to `#!/businessCenter` and scan navigation links. Also broadened the init script to capture *all* `/Vantage/api/` calls (not just keyword-filtered ones).

### 3. No Intercepted BoB API
**Problem**: Even on the Business Center page, `intercepted_apis` was empty — no BoB API call was captured.
**Reason**: The table data was loaded before the fetch script ran. The grid likely loads its data during Angular route resolution, before any injected script executes.
**Decision**: Switched from API interception to direct DOM scraping of the rendered table.

### 4. Table Found But Zero Rows (First Attempt)
**Problem**: `scrapeRows()` found the table (via "Humana ID" header) but returned 0 members. Debug showed: `rowCount: 1, tbodyRowCount: 0`.
**Reason**: The table with the headers had no `<tbody>` — it was a header-only table. The actual data rows were in a *second* `<table>` element (split-table grid pattern).
**Fix**: Implemented `findHeaderTable()` and `findBodyTable()` as separate functions. Column mapping comes from the header table; row scraping comes from the body table.

### 5. Stale DOM Reference After "View All Customers"
**Problem**: Second attempt still returned 0 members, even though the table was visible.
**Reason**: Clicking "View all customers" caused Angular to re-render the entire grid, invalidating our stored `table` DOM reference.
**Fix**: Changed `scrapeRows()` to re-query the DOM on every call (`findBodyTable()` + `buildColMap()` fresh each time) instead of using a cached table reference.

## Discovery Mode

The implementation went through two iterations of discovery mode before the final scraper:

1. **Discovery v1** (dashboard): Captured 25 dashboard XHR calls, no BoB APIs. Identified Angular hash routing and basic portal structure.
2. **Discovery v2** (Business Center): Found the member table with 19 columns, 6 members visible. `grids: 1` confirmed a grid component. `intercepted_apis: []` confirmed no API call to intercept.

## Date Implemented

V2 carrier sync expansion — fifth and final carrier implementation. Required the most iterative debugging (5 distinct issues) due to the split-table grid architecture and Angular's aggressive DOM re-rendering.
