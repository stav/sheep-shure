# Development Guide

Developer reference for working on the SHEEPS codebase.

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

There is no separate authentication system. The password **is** the encryption key:

1. User enters password
2. Read salt from `sheeps.salt` (or generate on first run)
3. Derive 32-byte key via Argon2id (64 MB, 3 iterations, 4 parallelism)
4. Pass key as `PRAGMA key` to SQLCipher
5. Verify with `SELECT count(*) FROM sqlite_master` — if it fails, wrong password
6. Enable WAL mode and foreign keys
7. Store the `Connection` in `DbState`

## Frontend Architecture

### Router

All routes are guarded by `AuthGuard` which checks `useAuthStore().isAuthenticated`:

| Route                | Page             |
| -------------------- | ---------------- |
| `/login`             | LoginPage        |
| `/dashboard`         | DashboardPage    |
| `/clients`           | ClientsPage      |
| `/clients/new`       | ClientFormPage   |
| `/clients/:id`       | ClientDetailPage |
| `/clients/:id/edit`  | ClientFormPage   |
| `/enrollments`       | EnrollmentsPage  |
| `/import`            | ImportPage       |
| `/reports`           | ReportsPage      |
| `/settings`          | SettingsPage     |

See `src/app/router.tsx`.

### Zustand Stores

| Store          | Purpose                           | File                     |
| -------------- | --------------------------------- | ------------------------ |
| `useAuthStore` | Auth state (isAuthenticated, etc) | `src/stores/authStore.ts` |
| `useAppStore`  | UI state (sidebar collapsed)      | `src/stores/appStore.ts`  |

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

## Backend Architecture

### Tauri Commands

Registered in `src-tauri/src/lib.rs` via `tauri::generate_handler![]`. Organized by domain:

| Module                | Commands                                             |
| --------------------- | ---------------------------------------------------- |
| `auth_commands`       | check_first_run, create_account, login, logout       |
| `client_commands`     | get_clients, get_client, create/update/delete_client, delete_all_clients |
| `enrollment_commands` | get_enrollments, create/update_enrollment             |
| `carrier_commands`    | get_carriers                                          |
| `import_commands`     | parse_import_file, validate_import, execute_import    |
| `report_commands`     | get_report, export_report_pdf, get_dashboard_stats   |
| `settings_commands`   | get/update_settings, get/save_agent_profile, backup_database |

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
}
```

`AppError` implements `Serialize`, so Tauri's blanket `From<T: Serialize> for InvokeError` handles the conversion automatically — no manual `From` impl needed.

### Models

Defined in `src-tauri/src/models/`:

| Model        | File             |
| ------------ | ---------------- |
| `Client`     | `client.rs`      |
| `Enrollment` | `enrollment.rs`  |
| `Carrier`    | `carrier.rs`     |
| `Plan`       | `plan.rs`        |
| Report types | `report.rs`      |

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

Current migrations: `v001_initial.sql`

### Schema

10 tables, 1 FTS virtual table:

| Table                     | Purpose                       |
| ------------------------- | ----------------------------- |
| `clients`                 | Core client records           |
| `enrollments`             | Plan enrollment tracking      |
| `plans`                   | Plan definitions              |
| `carriers`                | Insurance carriers            |
| `plan_types`              | Plan type codes (MA, MAPD...) |
| `enrollment_statuses`     | Status lifecycle codes        |
| `enrollment_periods`      | AEP, OEP, SEP, etc.          |
| `states`                  | US states + territories       |
| `notes`                   | Client notes                  |
| `import_logs`             | Import history                |
| `agent_profile`           | Agent info and NPN            |
| `agent_carrier_numbers`   | Agent writing numbers         |
| `app_settings`            | Key-value app settings        |
| `clients_fts`             | FTS5 full-text search index   |

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
| Carriers            | 14    | UHC, Humana, Aetna, BCBS, Cigna...          |
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

Records use `is_active INTEGER DEFAULT 1`. Queries filter by `is_active = 1`. Records are never physically deleted.

### IDs and Dates

- **UUIDs** everywhere (`uuid::Uuid::new_v4()`)
- **ISO 8601 text dates** stored as `TEXT DEFAULT (datetime('now'))`

### TanStack Query Cache Invalidation

Mutations invalidate relevant query keys on success:

- `create_client` / `delete_client` → invalidate `["clients"]`
- `update_client` → invalidate `["clients"]` and `["client", id]`

## Import System

The import follows a 4-step pipeline:

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

### Rust (33 `.rs` files + 1 `.sql`)

| Layer        | Files                                                              |
| ------------ | ------------------------------------------------------------------ |
| Commands     | auth, client, carrier, enrollment, import, report, settings + mod  |
| Services     | auth, client, dashboard, enrollment, import, report + mod          |
| Repositories | carrier, client, enrollment, report + mod                          |
| Models       | carrier, client, enrollment, plan, report + mod                    |
| DB           | connection, migrations, seed + mod                                 |
| Other        | lib.rs, main.rs, error.rs                                          |
| SQL          | v001_initial.sql                                                   |

### Frontend (41 `.ts`/`.tsx` files)

| Area       | Files                                                               |
| ---------- | ------------------------------------------------------------------- |
| App        | App.tsx, router.tsx, providers.tsx, main.tsx                         |
| Layout     | AppLayout.tsx, CommandPalette.tsx, index.ts                         |
| UI         | button, card, dialog, input, label, separator, tooltip              |
| Auth       | LoginPage.tsx, index.ts                                             |
| Clients    | ClientsPage, ClientDetailPage, ClientFormPage, index.ts            |
| Dashboard  | DashboardPage.tsx, index.ts                                         |
| Enrollments| EnrollmentsPage.tsx, index.ts                                       |
| Import     | ImportPage.tsx, index.ts                                            |
| Reports    | ReportsPage.tsx, index.ts                                           |
| Settings   | SettingsPage.tsx, index.ts                                          |
| Hooks      | useClients, useEnrollments, useKeyboardShortcuts, index.ts          |
| Stores     | authStore.ts, appStore.ts                                           |
| Lib        | tauri.ts, utils.ts                                                  |
| Types      | index.ts                                                            |
| Other      | ErrorBoundary.tsx, vite-env.d.ts                                    |
