# UnitedHealthcare — Carrier Sync

**Status**: Done
**Difficulty**: Easy-Moderate
**Carrier ID**: `carrier-uhc`
**Portal URL**: https://www.uhcjarvis.com/content/jarvis/en/secure/book-of-business-search.html
**Source**: `src-tauri/src/carrier_sync/uhc.rs`

## Portal Overview

UnitedHealthcare's agent portal ("Jarvis") is an Angular SPA with REST APIs. The Book of Business page at `/content/jarvis/en/secure/book-of-business-search.html` uses a POST API that requires two agent identifiers: `partyID` (internal agent number) and `opd` (operator code / agency ID).

## Approach: REST API with Init Script Parameter Capture

**Why this approach**: The Jarvis BoB API returns clean JSON with all member fields. The challenge is capturing the two required identifiers (`partyID` and `opd`) that the Angular app uses internally — they aren't in cookies or headers, they're in request bodies and URL query strings.

### Auth Mechanism

- **Session cookies**: Same-origin, HttpOnly, handled automatically by `fetch()`
- **CSRF**: None required
- **Agent identifiers**: `partyID` (from POST body) and `opd` (from URL query string) — must be captured from the SPA's own API calls

### Init Script

The init script monkey-patches `fetch` and `XMLHttpRequest` to intercept calls to URLs containing `bookOfBusiness`, extracting:
- **`opd`**: From URL query parameter `?opd=medstar216`
- **`hasPrincipalOrCorp`**: From URL query parameter
- **`partyID`**: From POST request body JSON `{ "partyID": "..." }`

### Fetch Script Flow — Multi-Stage Fallback

The partyID capture proved unreliable with just init script interception (the SPA sometimes makes the call before our script injects). The fetch script implements a three-stage fallback:

#### Stage 1: Check Init Script Captures
Read `window.__sheeps_uhc_partyID` and `window.__sheeps_uhc_opd`.

#### Stage 2: Performance API + Deep Storage Search
- Check `performance.getEntriesByType('resource')` for URLs containing `bookOfBusiness` and extract `opd` from the query string
- Deep-search `localStorage` and `sessionStorage` (up to 4 levels deep, including stringified JSON inside values) for `partyID` and `opd`

```javascript
function deepFind(obj, depth) {
    if (!obj || typeof obj !== 'object' || depth > 4) return;
    if (!partyID && (obj.partyID || obj.partyId)) partyID = obj.partyID || obj.partyId;
    if (!opd && obj.opd) opd = obj.opd;
    for (const k in obj) {
        if (typeof obj[k] === 'object') deepFind(obj[k], depth + 1);
        if (typeof obj[k] === 'string' && obj[k].startsWith('{')) {
            try { deepFind(JSON.parse(obj[k]), depth + 1); } catch (e) {}
        }
    }
}
```

#### Stage 3: Direct API Call
If partyID is still missing, call the Jarvis user profile API directly:
```
GET /JarvisAccountInfo/azure/api/secure/userprofile/partyID/v1
```
Parse the response for `partyID` / `partyId` (handles both casings).

### API Call

```
POST /JarvisMemberProfileAPI/azure/api/secure/bookOfBusiness/details/v1
     ?hasPrincipalOrCorp=false&opd=medstar216&homePage=false
```

### Request Body

```json
{
    "contractNumber": null,
    "memberFirstName": "",
    "memberLastName": "",
    "memberNumber": null,
    "planStatus": ["Active"],
    "partyID": "<captured>",
    "state": null,
    "product": null
}
```

### Response Fields Mapped to PortalMember

| API Field | PortalMember Field |
|-----------|--------------------|
| `memberFirstName` | `first_name` |
| `memberLastName` | `last_name` |
| `memberNumber` / `mbiNumber` | `member_id` |
| `dateOfBirth` | `dob` (converted MM/DD/YYYY → YYYY-MM-DD) |
| `planName` | `plan_name` |
| `policyEffectiveDate` | `effective_date` |
| `policyTermDate` | `end_date` (filtered: `2300-01-01` → null) |
| `memberStatus` | `status` (`"A"` → `"Active"`) |
| `memberState` | `state` |
| `memberCity` | `city` |
| `memberPhone` | `phone` |
| `memberEmail` | `email` |

**Note**: The API uses a sentinel date `2300-01-01` for "no end date". The script filters this to null.

### No Pagination

The API returns up to 500 records in a single response. No pagination needed for typical agent book sizes.

## Challenges Encountered

- **partyID null bug**: The initial implementation only relied on init script interception, but the Angular SPA sometimes makes the BoB API call *before* the init script has a chance to inject (race condition with document-start timing in WebKit2GTK). The partyID was consistently null.
  - **Fix**: Added the three-stage fallback — deep storage search finds partyID nested inside stringified JSON in sessionStorage, and the direct API call (`/userprofile/partyID/v1`) works as a last resort.
- **opd was captured, partyID was not**: The `opd` is in the URL (captured by Performance API entries), but `partyID` is only in the POST body. Debug output showed `partyID: null, opd: "medstar216"`, which pointed to the init script not running early enough.
- **Discovery via Performance API**: The `performance.getEntriesByType('resource')` trick (looking for URLs containing `Jarvis` or `bookOfBusiness`) was key to finding the partyID API endpoint that made the direct fallback possible.

## Date Implemented

V2 carrier sync expansion — fourth carrier implementation. Required the most debugging due to the partyID capture race condition. Established the "multi-stage fallback" pattern for capturing SPA-internal identifiers.
