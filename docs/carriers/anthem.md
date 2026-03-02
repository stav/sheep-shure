# Anthem/Elevance (Broker Portal)

Carrier sync implementation for Anthem's Broker Portal (Producer Toolbox).

## Portal Info

| Field | Value |
|-------|-------|
| Portal URL | `https://brokerportal.anthem.com/apps/ptb/login` |
| Carrier ID | `carrier-anthem` |
| Approach | REST API (Bearer + XSRF token interception) |
| File | `src-tauri/src/carrier_sync/anthem.rs` |

## Architecture

Anthem uses a REST API approach with three injected scripts:

### 1. Init Script (token interception)

Runs at document-start. Monkey-patches `fetch()` and `XMLHttpRequest.setRequestHeader()` to intercept:

- **Bearer tokens** from `Authorization` headers on requests matching `ptb`, `bob`, or `broker` URLs. Stored in `window.__compass_anthem_token`.
- **XSRF tokens** from `X-XSRF-TOKEN` headers. Stored in `window.__compass_anthem_xsrf`. Falls back to reading the `XSRF-TOKEN` cookie.

### 2. Auto-Login Script

Polls for the login form and auto-fills credentials from `window.__compass_creds`:

- Broadened username selectors: `input[type="text"]`, `input[type="email"]`, `input[name*="user"]`, `input[name*="login"]`, `input[id*="user"]`, `input[id*="login"]`
- Uses native `HTMLInputElement.prototype.value.set` to trigger React/Angular change detection
- Includes diagnostic logging (`[Compass:Anthem]` tag) for debugging SSO issues
- 2-second initial delay to let SSO redirects complete, then polls every 500ms for up to 15 seconds

### 3. Fetch Script (REST API)

Fetches the Book of Business via paginated POST requests:

```
POST /apps/ptb/api/client/summary?pageNumber=N&pageSize=100&sortBy=ClientName
Headers: Authorization: Bearer <token>, X-XSRF-TOKEN: <xsrf>
```

Paginates through all pages using `data.metadata.page.totalPages`. Maps response fields:

| API Field | Portal Member Field |
|-----------|-------------------|
| `clientName` | `first_name`, `last_name` (parsed from "LAST, FIRST M" format) |
| `clientID` | `member_id` |
| `clientStatus` | `status` (lowercased) |
| `planType` / `productType` | `plan_name` |
| `originalEffectiveDate` / `effectiveDate` | `effective_date` |
| `cancellationDate` | `end_date` |
| `latestBillStatus` | `policy_status` |
| `state` | `state` |
| `dob`, `city`, `phone`, `email` | Not available (all `null`) |

## Special Behaviors

### `override_window_open` = false

Anthem's SSO flow uses `window.open()` in ways that cause redirect loops when overridden. This is the only carrier that opts out of the `window.open()` override.

### Name-only client matching

Since Anthem does not provide DOB or MBI, the sync service uses `allow_name_only_unique: true` to match portal members to local clients by name alone when exactly one client with that name exists.

### Status determination

`policy_status` (from `latestBillStatus`) takes priority over `status`:

- Contains "inactive" -> `CANCELLED`
- Contains "active" -> `ACTIVE` (covers "Active Policy" and "Future Active Policy")
- Empty/missing -> falls back to `status` field

## Key Differences from Other Carriers

| Feature | Anthem | Most Others |
|---------|--------|-------------|
| Token capture | Bearer + XSRF | Bearer only or cookies |
| Auto-login | Yes | No |
| `window.open` override | Disabled | Enabled |
| DOB available | No | Varies |
| MBI available | No | Varies |
| Client matching | Name-only unique | Name + DOB |
