# CareSource — Carrier Sync

**Status**: Done
**Difficulty**: Easy
**Carrier ID**: `carrier-caresource`
**Portal URL**: https://caresource2.destinationrx.com/PC/Agent/Account/Login
**Source**: `src-tauri/src/carrier_sync/caresource.rs`

## Portal Overview

CareSource uses the DestinationRx (DRX) platform — a SPA that communicates with a **cross-origin** REST API at `https://www.drxwebservices.com`. The agent logs in at `caresource2.destinationrx.com` but all member data calls go to `drxwebservices.com`.

## Approach: REST API with Init Script Token Capture

**Why this approach**: The DRX REST API returns clean JSON, making API calls the obvious choice over DOM scraping. However, the API uses **Bearer JWT authentication** on a **cross-origin domain** — so we need the init script to capture the token from the SPA's own requests.

### Auth Mechanism

- **Bearer JWT**: Not cookie-based. The SPA sends `Authorization: Bearer <token>` headers to `drxwebservices.com`
- **Cross-origin**: The login domain (`caresource2.destinationrx.com`) differs from the API domain (`drxwebservices.com`)
- **CSRF**: None required
- **Agent GUID**: Embedded in API URL paths (e.g., `/Agent/{guid}/MemberProfileSearch`)

### Init Script (Critical)

The init script monkey-patches both `fetch` and `XMLHttpRequest.setRequestHeader` to intercept:
1. **Bearer JWT** from the `Authorization` header on requests to `drxwebservices.com`
2. **Agent GUID** extracted from URL path via regex `/Agent/([0-9a-f-]{36})/`
3. **API base URL** (e.g., `https://www.drxwebservices.com/spa2026/v1`) — the year changes annually

These are stored on `window.__sheeps_drx_token`, `window.__sheeps_drx_agent_guid`, and `window.__sheeps_drx_api_base`.

### Fetch Script Flow

1. Read captured token and agent GUID from `window.__sheeps_drx_*`
2. If not captured yet, instruct the user to navigate to the Reports page first (triggers an API call)
3. Build 31-day date ranges from Oct 1 of the previous year through today
4. For each range, POST to `/Agent/{guid}/MemberProfileSearch` with date window
5. Deduplicate by `memberID` using a `Map`
6. Navigate to `sheeps-sync.localhost/data`

### Date Range Windowing

The DRX API has a **31-day maximum query window**. To get the full book of business, we iterate through time in chunks:

```
Oct 1 prev year → Oct 31
Nov 1 → Dec 1
Dec 2 → Jan 1
... (continues through today)
```

Each chunk returns members whose applications fall within that window. We deduplicate by `memberID` to avoid counting members who appear in multiple windows.

### Request Shape

```json
{
    "applicationStartDate": "2025-10-01T00:00:00.000Z",
    "applicationEndDate": "2025-10-31T23:59:59.000Z",
    "enrollmentType": "medicare",
    "agentReport": true
}
```

### Response Fields Mapped to PortalMember

| API Field | PortalMember Field |
|-----------|--------------------|
| `firstName` | `first_name` |
| `lastName` | `last_name` |
| `memberID` | `member_id` |
| *(not available)* | `dob` |
| `enrollments[0].plan` | `plan_name` |
| `enrollments[0].enrollmentDate` | `effective_date` |
| `carrierStatus` | `status` |
| `state` | `state` |
| `city` | `city` |
| `homePhone` | `phone` |
| `primaryEmailAddress` | `email` |

**Note**: DOB is not available from the DRX MemberProfileSearch endpoint. Matching relies on name-only fallback.

## Challenges Encountered

- **Cross-origin auth**: The Bearer JWT can't be read from cookies — it only appears in request headers. Required monkey-patching `fetch` and `XHR.setRequestHeader` in the init script.
- **31-day date limit**: Initial implementation tried fetching all members at once and got empty results. Had to discover the date range limit through trial and error, then implement the windowed iteration with deduplication.
- **Year-specific API base**: The API URL contains the current year (e.g., `/spa2026/v1`). The init script captures this dynamically so it doesn't break on year rollover.
- **Token timing**: The user needs to navigate within the portal before clicking Sync Now, because the SPA doesn't make API calls until the user interacts. The fetch script shows a helpful error if the token hasn't been captured yet.

## Date Implemented

Phase 8 (V1 Polish) — second carrier implementation. Established the init script pattern for token interception that was later reused by UHC.
