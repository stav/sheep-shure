# Devoted Health — Carrier Sync

**Status**: Done
**Difficulty**: Easiest
**Carrier ID**: `carrier-devoted`
**Portal URL**: https://agent.devoted.com/
**Source**: `src-tauri/src/carrier_sync/devoted.rs`

## Portal Overview

Devoted Health's agent portal is a React SPA (codenamed "Orinoco") backed by a GraphQL API. It's the most developer-friendly carrier portal — clean API, standard auth, no anti-bot measures.

## Approach: GraphQL API (webview-injected JS)

**Why this approach**: The portal exposes a well-structured GraphQL API with persisted queries. No need for DOM scraping — we call the same API the React app uses, getting clean structured JSON with full pagination support.

### Auth Mechanism

- **SSO**: Auth0-based, session stored in HttpOnly `devoted_session` JWT cookie
- **CSRF**: Required. Fetched via a dedicated `CSRFToken` GraphQL persisted query, then sent as `x-csrf-token` header on every subsequent request
- **Custom headers**: `x-orinoco-portal: Agents` and `x-orinoco-client-version` (read from `window.__orinoco_config.VERSION`)

### Init Script

None needed. The session cookies are HttpOnly so JS can't read them anyway, but `fetch()` from the webview sends them automatically.

### Fetch Script Flow

1. Read `window.__orinoco_config.VERSION` for the client version header
2. Call the `CSRFToken` persisted query to get a fresh CSRF token
3. Page through `ListBookOfBusinessContacts` (100 members per page) using persisted query hash
4. Collect all members and navigate to `sheeps-sync.localhost/data`

### Persisted Query Hashes

| Query | SHA-256 Hash |
|-------|-------------|
| `CSRFToken` | `0ba70438537351c55da05b9cec107834cf0e6e1126b9107bb382cba283d9dc5a` |
| `ListBookOfBusinessContacts` | `881c07f52080a6a6a04c653b03fa4520acfd30de90ab0ac6ca4caa161f6bbc95` |

### Request Shape

```json
{
    "operationName": "ListBookOfBusinessContacts",
    "variables": {
        "limit": 100,
        "page": 1,
        "order_by": [
            { "by": "LAST_NAME", "direction": "ASC" },
            { "by": "FIRST_NAME", "direction": "ASC" },
            { "by": "MIDDLE_NAME", "direction": "ASC" }
        ],
        "filter_by": { "member_id": { "op": "ISNOTNULL" } },
        "options": { "allow_partial": true, "cap_total_item_count": 10000 }
    },
    "extensions": {
        "persistedQuery": { "version": 1, "sha256Hash": "<hash>" }
    }
}
```

### Response Fields Mapped to PortalMember

| API Field | PortalMember Field |
|-----------|--------------------|
| `first_name` | `first_name` |
| `last_name` | `last_name` |
| `member_id` | `member_id` |
| `birth_date` | `dob` |
| `current_pbp.pbp_name` | `plan_name` |
| `current_pbp.start_date` | `effective_date` |
| `current_pbp.end_date` | `end_date` |
| `status` | `status` |
| `aor_policy_status` | `policy_status` |
| `state` | `state` |
| `city` | `city` |
| `primary_phone` | `phone` |
| `email` | `email` |

### Reqwest Fallback

Devoted is the only carrier with a working Rust-side `fetch_members()` implementation using `reqwest`. It extracts the `devoted-csrf` cookie from the cookie string, then makes the same GraphQL calls server-side. This was implemented as a proof of concept but the webview JS approach is the primary path.

## Challenges Encountered

- **CSRF token flow**: Initial attempt tried to intercept CSRF from the app's own requests. Discovered it's simpler to just call the `CSRFToken` query directly — no init script needed.
- **Persisted queries**: The API only accepts requests with `extensions.persistedQuery` — regular GraphQL queries are rejected. Had to capture the exact SHA-256 hashes from DevTools.

## Date Implemented

Phase 8 (V1 Polish) — first carrier sync implementation, served as the template for the `CarrierPortal` trait design.
