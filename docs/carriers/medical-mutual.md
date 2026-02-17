# Medical Mutual of Ohio — Carrier Sync

**Status**: Done
**Difficulty**: Easy
**Carrier ID**: `carrier-medmutual`
**Portal URL**: https://mybrokerlink.com/
**Source**: `src-tauri/src/carrier_sync/medmutual.rs`

## Portal Overview

Medical Mutual's MyBrokerLink portal is a traditional **server-rendered Sitecore CMS** site. No SPA, no JavaScript framework, no API — the member data is delivered as plain HTML in a server-rendered table.

## Approach: Server-Rendered HTML Table Scraping

**Why this approach**: There is no API to call. The portal renders the entire Book of Business as an HTML table at `/mybusiness/bookofbusiness`. The simplest and most reliable approach is to fetch that HTML page via AJAX (using the browser's session cookies) and parse the DOM.

### Auth Mechanism

- **Session cookies**: Same-origin, HttpOnly, sent automatically by `fetch()`
- **CSRF**: None required
- **No init script needed**: Cookies handle everything

### Init Script

None. This is the simplest implementation — no token interception, no API discovery. The browser's session cookies are all that's needed.

### Fetch Script Flow

1. `fetch('/mybusiness/bookofbusiness')` — gets the full BoB HTML page
2. Parse with `new DOMParser().parseFromString(html, 'text/html')`
3. Find `#member-table` in the parsed document
4. Iterate `tbody tr` rows, extracting cell values via `td[data-col-name="..."]` selectors
5. Each `<td>` contains a `.sb-content` child with the actual text
6. Navigate to `sheeps-sync.localhost/data`

### Table Structure

The `#member-table` uses `data-col-name` attributes on each `<td>`, making column identification robust regardless of column order:

```html
<tr>
    <td data-col-name="Name"><div class="sb-content">John Smith</div></td>
    <td data-col-name="GroupNumber"><div class="sb-content">12345</div></td>
    <td data-col-name="DateOfBirth"><div class="sb-content">01/15/1950</div></td>
    ...
</tr>
```

### Fields Extracted

| HTML Column (`data-col-name`) | PortalMember Field |
|-------------------------------|--------------------|
| `Name` | `first_name`, `last_name` (split by whitespace) |
| `GroupNumber` | `member_id` |
| `DateOfBirth` | `dob` (converted MM/DD/YYYY → YYYY-MM-DD) |
| `MarketSegment` | `plan_name` |
| `EffectiveDate` | `effective_date` (converted MM/DD/YYYY → YYYY-MM-DD) |
| `Attention` (button text) | `status` ("Canceled" or null → "Active") |
| `State` | `state` |
| `City` | `city` |
| `Phone` | `phone` |
| `Email` | `email` |

### Name Parsing

Names are in `FIRST LAST` format (space-separated). Parsed as:
- `parts[0]` → `first_name`
- `parts[1..]` joined → `last_name`

### Status Detection

The "Attention" column contains a `<button>` element when the member has a status issue (e.g., "Canceled"). If no button is present, the member is assumed "Active".

## Challenges Encountered

- **Minimal**: This was the easiest implementation. Server-rendered HTML with semantic `data-col-name` attributes made parsing straightforward.
- **No pagination concerns**: The BoB page renders all members at once (no client-side pagination or lazy loading).
- **AJAX fetch works**: Despite being a server-rendered site, `fetch()` from JS correctly returns the HTML page with all data, no redirect issues.

## Date Implemented

Phase 8 (V1 Polish) — third carrier implementation. Demonstrated that the `CarrierPortal` trait works well for simple HTML scraping, not just API-based integrations.
