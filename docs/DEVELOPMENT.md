# Development Guide

Developer reference for working on the Compass codebase.

## Architecture Overview

The backend follows a three-layer architecture:

```
Frontend (React)
    ↓ invoke()
Commands (thin IPC handlers)     ← src-tauri/src/commands/
    ↓
Services (business logic)        ← src-tauri/src/services/
    ↓
Repositories (SQL queries)       ← src-tauri/src/repositories/
```

**Commands** are thin Tauri `#[tauri::command]` functions that extract state, call a service, and return the result. They contain no business logic.

**Services** implement all business logic — validation, transformation, orchestration across repositories.

**Repositories** are pure SQL — they accept a `&Connection` and return model structs.

### DbState Lifecycle

```rust
pub struct DbState {
    pub conn: Mutex<Option<Connection>>,
}
```

- On app start, `DbState` holds `None` — the database is locked
- On login/account creation, `auth_service` derives the SQLCipher key and opens the connection, then stores it via `set_connection()`
- All subsequent commands use `db_state.with_conn(|conn| ...)` to access the connection
- On logout, `clear_connection()` drops the connection and sets it back to `None`

### Auth Flow

There is no separate authentication system. The password **is** the encryption key. The implementation differs between dev and release builds, controlled by `#[cfg(debug_assertions)]` in `auth_service.rs` — this is compile-time conditional compilation, so the compiler physically includes only one code path per build profile.

#### Release builds (encrypted)

Database file: `compass.db` + `compass.salt`

1. User enters password
2. Read salt from `compass.salt` (or generate on first run)
3. Derive 32-byte key via Argon2id (64 MB, 3 iterations, 4 parallelism)
4. Pass key as `PRAGMA key` to SQLCipher
5. Verify with `SELECT count(*) FROM sqlite_master` — if it fails, wrong password
6. Enable WAL mode and foreign keys
7. Store the `Connection` in `DbState`

#### Dev builds (unencrypted)

Database file: `compass-dev.db` (no salt file)

- Plain `Connection::open()` — no encryption, no Argon2 derivation
- The password parameter is accepted but ignored
- DB is inspectable with any SQLite tool (`sqlite3` CLI, DB Browser for SQLite, etc.)
- The login UI still shows the password form for UX consistency, but any value is accepted

The separate filenames prevent accidentally opening a dev DB with a release build or vice versa. Both builds export the same four functions (`is_first_run`, `create_database`, `unlock_database`, `change_password`) with identical signatures, so the command layer requires no `cfg` gating.

## Frontend Architecture

### Router

All routes are guarded by `AuthGuard` which checks `useAuthStore().isAuthenticated`:

| Route                  | Page              |
| ---------------------- | ----------------- |
| `/login`               | LoginPage         |
| `/dashboard`           | DashboardPage     |
| `/clients`             | ClientsPage       |
| `/clients/new`         | ClientFormPage    |
| `/clients/:id`         | ClientDetailPage  |
| `/clients/:id/edit`    | ClientFormPage    |
| `/clients/duplicates`  | DuplicateScanPage |
| `/import`              | ImportPage        |
| `/carrier-sync`        | CarrierSyncPage   |
| `/commissions`         | CommissionsPage   |
| `/settings`            | SettingsPage      |

See `src/app/router.tsx`.

### Zustand Stores

| Store           | Purpose                           | File                      |
| --------------- | --------------------------------- | ------------------------- |
| `useAuthStore`  | Auth state (isAuthenticated, etc) | `src/stores/authStore.ts` |
| `useAppStore`   | UI state (sidebar collapsed)      | `src/stores/appStore.ts`  |
| `useThemeStore` | Theme preferences                 | `src/stores/themeStore.ts` |

### TanStack Query Hooks

Custom hooks in `src/hooks/` wrap Tauri IPC calls with TanStack Query:

```typescript
// Query with automatic caching
export function useClients(filters, page, perPage) {
  return useQuery({
    queryKey: ["clients", filters, page, perPage],
    queryFn: () => tauriInvoke("get_clients", { filters, page, perPage }),
  });
}

// Mutation with cache invalidation
export function useCreateClient() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input) => tauriInvoke("create_client", { input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["clients"] }),
  });
}
```

The `tauriInvoke` wrapper in `src/lib/tauri.ts` is a typed passthrough to `@tauri-apps/api/core`'s `invoke`.

Additional hooks beyond clients/enrollments:

| Hook                  | File                          | Purpose                              |
| --------------------- | ----------------------------- | ------------------------------------ |
| `useCarrierSync`      | `src/hooks/useCarrierSync.ts` | Carrier portal sync queries/mutations |
| `useCommissions`      | `src/hooks/useCommissions.ts` | Commission CRUD and reconciliation   |
| `useConversations`    | `src/hooks/useConversations.ts` | Conversation CRUD and timeline       |
| `useEnrollments`      | `src/hooks/useEnrollments.ts` | Enrollment CRUD                      |
| `useZoom`             | `src/hooks/useZoom.ts`        | Webview zoom control                 |
| `useKeyboardShortcuts`| `src/hooks/useKeyboardShortcuts.ts` | Global keyboard shortcuts       |

## Backend Architecture

### Tauri Commands

Registered in `src-tauri/src/lib.rs` via `tauri::generate_handler![]`. Organized by domain:

| Module                     | Commands                                             |
| -------------------------- | ---------------------------------------------------- |
| `auth_commands`            | check_first_run, create_account, login, logout       |
| `client_commands`          | get_clients, get_client, create/update/delete_client, hard_delete_client, merge_clients, check_client_duplicates, find_duplicate_clients, delete_all_clients |
| `enrollment_commands`      | get_enrollments, get_enrollment, create/update/delete_enrollment |
| `conversation_commands`    | get_conversations, get/create/update_conversation, get/create/update_conversation_entry, get_client_timeline, get_pending_follow_ups, create_system_event |
| `carrier_commands`         | get_carriers                                          |
| `carrier_sync_commands`    | open_carrier_login, trigger_carrier_fetch, process_portal_members, get_carrier_login_url, get_carrier_sync_info, import_portal_members, confirm_disenrollments, get_sync_logs, update_carrier_expected_active, save/get/delete_portal_credentials, get_carriers_with_credentials |
| `import_commands`          | parse_import_file, validate_import, preview_import, execute_import, import_call_log, import_integrity, import_sirem, enrich_leadsmaster |
| `commission_commands`      | get/create/update/delete_commission_rate, get_commission_entries, update/delete_commission_entry, delete_commission_batch, parse/import_commission_statement, import_commission_csv, trigger_commission_fetch, reconcile_commissions, find_missing_commissions, get_reconciliation_entries, get_commission_summary, get/create/update/delete_commission_deposit |
| `report_commands`          | get_dashboard_stats                                   |
| `settings_commands`        | get/update_settings, get/save_agent_profile, backup_database, get_database_info |

### Error Handling

All errors flow through `AppError` (`src-tauri/src/error.rs`):

```rust
pub enum AppError {
    Database(String),
    Auth(String),
    Validation(String),
    NotFound(String),
    Import(String),
    Io(String),
    CarrierSync(String),
}
```

`AppError` derives `thiserror::Error` and `Serialize`. Blanket `From` impls convert `rusqlite::Error`, `reqwest::Error`, and `std::io::Error` into the appropriate variant. Tauri's `From<T: Serialize> for InvokeError` handles the final conversion — no manual impl needed.

### Models

Defined in `src-tauri/src/models/`:

| Model             | File               |
| ----------------- | ------------------ |
| `Client`          | `client.rs`        |
| `Enrollment`      | `enrollment.rs`    |
| `Carrier`         | `carrier.rs`       |
| `CarrierSync`     | `carrier_sync.rs`  |
| `Conversation`    | `conversation.rs`  |
| `Plan`            | `plan.rs`          |
| `Provider`        | `provider.rs`      |
| `Commission`      | `commission.rs`    |
| Dashboard types   | `report.rs`        |

## Database

### SQLCipher Setup

- `rusqlite` with `bundled-sqlcipher` feature compiles SQLCipher from source
- Key set via `PRAGMA key = "x'<hex>'"`
- WAL journal mode and foreign keys enabled after key verification

### Migration System

Migrations use SQLite's `PRAGMA user_version` for tracking (`src-tauri/src/db/migrations.rs`):

1. Read current `user_version`
2. Apply any migrations with a higher version number
3. Update `user_version` after each successful migration
4. Migrations are embedded via `include_str!()` and run on every login (idempotent)

Current migrations:

| Migration                       | Purpose                                           |
| ------------------------------- | ------------------------------------------------- |
| `v001_initial.sql`              | Core schema (clients, enrollments, carriers, etc) |
| `v002_conversations.sql`        | Threaded conversations replacing notes table      |
| `v003_carrier_sync.sql`         | Carrier sync logs table                           |
| `v004_caresource_enrollments.sql` | Seed CareSource enrollments for existing clients |
| `v005_expected_active.sql`      | `expected_active` column on carriers              |
| `v006_member_details.sql`       | `member_record_locator` on clients, `client_providers` table |
| `v007_commissions.sql`          | Commission tables (rates, entries, deposits)          |
| `v008_deposits_allow_multiple.sql` | Remove unique constraint on deposits (allow multiple per carrier/month) |
| `v009_raw_data.sql`             | Add `raw_data` column to `commission_entries`         |

### Schema

Core tables + FTS virtual table:

| Table                     | Purpose                                  |
| ------------------------- | ---------------------------------------- |
| `clients`                 | Core client records                      |
| `enrollments`             | Plan enrollment tracking                 |
| `plans`                   | Plan definitions                         |
| `carriers`                | Insurance carriers (+ `expected_active`) |
| `plan_types`              | Plan type codes (MA, MAPD...)            |
| `enrollment_statuses`     | Status lifecycle codes                   |
| `enrollment_periods`      | AEP, OEP, SEP, etc.                     |
| `states`                  | US states + territories                  |
| `conversations`           | Threaded client conversations            |
| `conversation_entries`    | Individual entries within conversations  |
| `carrier_sync_logs`       | Carrier portal sync history              |
| `client_providers`        | Client PCP/provider records              |
| `commission_rates`        | Expected commission rates by carrier/plan/year |
| `commission_entries`      | Commission line items from statements    |
| `commission_deposits`     | Bank deposits from carriers              |
| `import_logs`             | Import history                           |
| `agent_profile`           | Agent info and NPN                       |
| `agent_carrier_numbers`   | Agent writing numbers                    |
| `app_settings`            | Key-value app settings                   |
| `clients_fts`             | FTS5 full-text search index              |

### FTS5

The `clients_fts` virtual table indexes: `first_name`, `last_name`, `mbi`, `phone`, `email`, `city`, `zip`. Sync triggers on `clients` keep it updated automatically on INSERT, UPDATE, and DELETE.

### Triggers

- **FTS sync** — 3 triggers keep `clients_fts` in sync with `clients`
- **updated_at** — 8 triggers auto-set `updated_at = datetime('now')` on UPDATE for all core tables

### Indexes

14 indexes on `clients` and `enrollments` covering: name, MBI, zip, state, DOB, is_active, is_dual_eligible, client_id, plan_id, carrier_id, status_code, effective_date.

### Seed Data

Populated on first run via `src-tauri/src/db/seed.rs`:

| Data                | Count | Examples                                    |
| ------------------- | ----- | ------------------------------------------- |
| Carriers            | 18    | UHC, Humana, Aetna, Anthem, BCBS, Cigna, CareSource, Zing, MedMutual, SummaCare... |
| Plan types          | 21    | MA, MAPD, PDP, DSNP, Medigap A-N           |
| States/territories  | 53    | 50 states + DC, PR, USVI                    |
| Enrollment statuses | 10    | Active, Pending, Disenrolled (5 reasons)... |
| Enrollment periods  | 8     | AEP, MA OEP, IEP, SEP, 5-Star SEP...       |

## Key Patterns

### `spawn_blocking` for Argon2

Argon2id key derivation is CPU-intensive (64 MB memory, ~200ms). Auth commands use `tauri::async_runtime::spawn_blocking` to run it off the main thread, preventing UI freezes:

```rust
#[tauri::command]
pub async fn login(password: String, ...) -> Result<(), String> {
    let conn = tauri::async_runtime::spawn_blocking(move || {
        auth_service::unlock_database(&data_dir, &password)
    }).await??;
    db_state.set_connection(conn)?;
    Ok(())
}
```

### Soft Deletes

Records use `is_active INTEGER DEFAULT 1`. Queries filter by `is_active = 1`. `delete_client` sets `is_active = 0` (soft delete). `hard_delete_client` physically removes the record (used for duplicate merging and data cleanup).

### IDs and Dates

- **UUIDs** everywhere (`uuid::Uuid::new_v4()`)
- **ISO 8601 text dates** stored as `TEXT DEFAULT (datetime('now'))`

### TanStack Query Cache Invalidation

Mutations invalidate relevant query keys on success:

- `create_client` / `delete_client` → invalidate `["clients"]`
- `update_client` → invalidate `["clients"]` and `["client", id]`

## Import System

### General File Import

The general import follows a 4-step pipeline:

1. **Parse** — Read CSV/XLSX file, extract headers and sample rows (`parse_file`)
2. **Auto-map** — Match source column headers to target fields using alias lookup (`auto_map_columns`)
3. **Validate** — Check required fields (first/last name) and MBI format (`validate_rows`)
4. **Execute** — For each row, find existing client by MBI or name+DOB, then insert or update (`execute_import`)

The import also supports **constant values** — fields that apply to every row (e.g., setting carrier or lead source for an entire file).

Deduplication logic:
- Match by MBI first (exact match)
- Fall back to first_name + last_name + DOB
- If matched: update only non-empty fields that differ
- If no match: insert as new client

### Specialized Importers

Located in `src-tauri/src/services/import/`:

| Module         | Command             | Purpose                                  |
| -------------- | ------------------- | ---------------------------------------- |
| `file_import`  | execute_import      | General CSV/XLSX import with column mapping |
| `call_log`     | import_call_log     | Call log file import                     |
| `integrity`    | import_integrity    | Integrity report import                  |
| `sirem`        | import_sirem        | SIREM file import                        |
| `leadsmaster`  | enrich_leadsmaster  | Enrich existing clients from Leadsmaster |
| `shared`       | —                   | Shared utilities across importers        |
| `matching`     | —                   | Fuzzy client matching service (in `services/matching.rs`) |

## WebKit2GTK Quirks

Working on Linux with WebKit2GTK has specific gotchas:

### Wayland/Hyprland GBM Buffer Errors

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 bun tauri dev
```

Required on Wayland compositors (Hyprland, Sway, etc.) to avoid `dma-buf` renderer crashes.

### Radix TooltipTrigger + asChild

WebKit2GTK can override Tailwind `flex` classes on anchor/button elements inside Radix `TooltipTrigger` with `asChild`. The fix is to separate layout from styling:

```tsx
// Bad — WebKit overrides flex
<TooltipTrigger asChild>
  <a className="flex items-center gap-3 px-4 py-2">
    <Icon /> Label
  </a>
</TooltipTrigger>

// Good — inner span handles layout
<TooltipTrigger asChild>
  <a className="block px-4 py-2">
    <span className="flex items-center gap-3">
      <Icon /> Label
    </span>
  </a>
</TooltipTrigger>
```

### Tooltip Grey Bar Artifacts

Wrapping elements in `Tooltip` when no tooltip is needed can cause grey bar rendering artifacts in WebKit2GTK. Only wrap in `Tooltip` when the tooltip content is meaningful.

### No Inline Styles

Always use Tailwind classes. Never use inline `style` props as workarounds — they bypass the design system and create inconsistencies.

## Environment Setup

### Arch Linux

```bash
# System dependencies
sudo pacman -S webkit2gtk-4.1

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Bun
curl -fsSL https://bun.sh/install | bash
```

### Development Commands

```bash
# Frontend dependencies
bun install

# Development mode
WEBKIT_DISABLE_DMABUF_RENDERER=1 bun tauri dev

# Type-check frontend
bun run build     # runs tsc -b && vite build

# Type-check backend
cargo check --manifest-path src-tauri/Cargo.toml
```

## File Inventory

### Rust (63 `.rs` files + 9 `.sql`)

| Layer          | Files                                                                 |
| -------------- | --------------------------------------------------------------------- |
| Commands       | auth, client, carrier, carrier_sync, commission, conversation, enrollment, import, report, settings + mod |
| Services       | auth, carrier_sync, client, commission, conversation, dashboard, enrollment, matching, provider + mod |
| Services/Import| call_log, file_import, integrity, leadsmaster, shared, sirem + mod    |
| Services/Commission Importers | generic, humana + mod                                  |
| Repositories   | carrier, client, commission, conversation, enrollment, provider, report + mod |
| Models         | carrier, carrier_sync, client, commission, conversation, enrollment, plan, provider, report + mod |
| Carrier Sync   | anthem, caresource, devoted, humana, medmutual, uhc + mod            |
| DB             | connection, migrations, seed + mod                                    |
| Other          | lib.rs, main.rs, error.rs                                             |
| SQL            | v001_initial, v002_conversations, v003_carrier_sync, v004_caresource_enrollments, v005_expected_active, v006_member_details, v007_commissions, v008_deposits_allow_multiple, v009_raw_data |

### Frontend (82 `.ts`/`.tsx` files)

| Area           | Files                                                                |
| -------------- | -------------------------------------------------------------------- |
| App            | App.tsx, router.tsx, providers.tsx, main.tsx                          |
| Layout         | AppLayout.tsx, CommandPalette.tsx, FindInPage.tsx, index.ts           |
| UI             | badge, button, card, checkbox, dialog, dropdown-menu, input, label, scroll-area, select, separator, tabs, textarea, tooltip |
| Auth           | LoginPage.tsx, index.ts                                              |
| Carrier Sync   | CarrierSyncPage, CarrierTable, CredentialsDialog, DisenrollmentSection, NewInPortalSection, SyncResultsPanel, utils, index.ts |
| Clients        | ClientsPage, ClientDetailPage, ClientFormPage, DuplicateScanPage, index.ts |
| Commissions    | CommissionsPage, RatesTab, StatementImportTab, ReconciliationTab, CarrierSummaryTab, DepositsTab, ActivityLog, components/(RateFormDialog, DepositFormDialog, StatusBadge, EntryEditDialog, RawDataDialog), index.ts |
| Dashboard      | DashboardPage.tsx, index.ts                                          |
| Engagement     | ClientEngagementSection, ConversationDetail, ConversationList, EntryFormDialog, FollowUpBadge, NewConversationDialog, TimelineCard, index.ts |
| Enrollments    | EnrollmentFormDialog.tsx, index.ts                                   |
| Import         | ImportPage.tsx, index.ts                                             |
| Settings       | SettingsPage.tsx, index.ts                                           |
| Hooks          | useClients, useCommissions, useConversations, useCarrierSync, useEnrollments, useKeyboardShortcuts, useZoom, index.ts |
| Stores         | authStore.ts, appStore.ts, themeStore.ts                             |
| Lib            | tauri.ts, utils.ts                                                   |
| Types          | index.ts                                                             |
| Other          | ErrorBoundary.tsx, vite-env.d.ts                                     |
