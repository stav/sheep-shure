# Commissions System

Complete documentation for the Compass commissions module — rate management, statement import, reconciliation, deposit tracking, and carrier summary reporting.

**Introduced in**: commit `da7e4c7` on the `commissions` branch.

---

## Overview

The commissions system tracks insurance carrier commission payments against expected rates for client enrollments. It supports:

1. **Rate Table** — Define expected commission rates by carrier, plan type, and year
2. **Statement Import** — Parse carrier statement files (CSV/XLSX) and match entries to clients
3. **Reconciliation** — Compare statement amounts against expected rates, flag discrepancies
4. **Deposit Tracking** — Record bank deposits and reconcile against statement totals
5. **Carrier Summary** — Aggregate view of commission performance by carrier and month

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Frontend (React + TypeScript)                      │
│  src/features/commissions/                          │
│    CommissionsPage  (tabbed layout)                  │
│    ├── RatesTab                                      │
│    ├── StatementImportTab                            │
│    ├── ReconciliationTab                             │
│    ├── DepositsTab                                   │
│    └── CarrierSummaryTab                             │
│  src/hooks/useCommissions.ts  (React Query hooks)    │
│  src/types/index.ts           (TypeScript types)     │
├─────────────────────────────────────────────────────┤
│  Tauri Commands (RPC boundary)                      │
│  src-tauri/src/commands/commission_commands.rs       │
├─────────────────────────────────────────────────────┤
│  Service Layer (business logic)                     │
│  src-tauri/src/services/commission_service.rs       │
├─────────────────────────────────────────────────────┤
│  Repository Layer (SQL queries)                     │
│  src-tauri/src/repositories/commission_repo.rs      │
├─────────────────────────────────────────────────────┤
│  Database (SQLite)                                  │
│  src-tauri/src/db/migrations/v007_commissions.sql   │
│  Tables: commission_rates, commission_entries,       │
│          commission_deposits                         │
└─────────────────────────────────────────────────────┘
```

---

## Database Schema

### `commission_rates`

Stores expected commission rates per carrier/plan type/year.

| Column          | Type    | Description                              |
|-----------------|---------|------------------------------------------|
| `id`            | TEXT PK | UUID                                     |
| `carrier_id`    | TEXT FK | References `carriers.id`                 |
| `plan_type_code`| TEXT FK | References `plan_types.code`             |
| `plan_year`     | INTEGER | The plan year (e.g. 2025, 2026)          |
| `initial_rate`  | REAL    | $/month for first-year enrollments       |
| `renewal_rate`  | REAL    | $/month for subsequent-year enrollments  |
| `notes`         | TEXT    | Optional                                 |
| `created_at`    | TEXT    | Auto-set                                 |
| `updated_at`    | TEXT    | Auto-updated via trigger                 |

**Unique constraint**: `(carrier_id, plan_type_code, plan_year)`

### `commission_entries`

Individual commission line items, either imported from statements or generated during reconciliation.

| Column            | Type    | Description                                         |
|-------------------|---------|-----------------------------------------------------|
| `id`              | TEXT PK | UUID                                                |
| `client_id`       | TEXT FK | References `clients.id` — NULL if unmatched         |
| `enrollment_id`   | TEXT FK | References `enrollments.id`                         |
| `carrier_id`      | TEXT FK | References `carriers.id`                            |
| `plan_type_code`  | TEXT    | MA, MAPD, DSNP, PDP, MedSupF, etc.                 |
| `commission_month`| TEXT    | YYYY-MM format                                      |
| `statement_amount`| REAL    | Gross commission from carrier statement              |
| `paid_amount`     | REAL    | Net amount received                                 |
| `member_name`     | TEXT    | Name as it appears on the statement                 |
| `member_id`       | TEXT    | MBI, Medicare ID, or subscriber ID from statement   |
| `is_initial`      | INTEGER | 1 = first year, 0 = renewal (set during reconcile)  |
| `expected_rate`   | REAL    | Looked up from commission_rates during reconcile     |
| `rate_difference` | REAL    | `statement_amount - expected_rate`                   |
| `status`          | TEXT    | OK, UNDERPAID, OVERPAID, MISSING, ZERO_RATE, UNMATCHED, PENDING |
| `import_batch_id` | TEXT    | Groups entries from the same import for undo         |
| `raw_data`        | TEXT    | Original statement row data (JSON, added in v009)    |
| `notes`           | TEXT    | Optional                                             |
| `created_at`      | TEXT    | Auto-set                                             |
| `updated_at`      | TEXT    | Auto-updated via trigger                             |

**Unique constraint**: `(carrier_id, client_id, commission_month)` where `client_id IS NOT NULL`

**Indexes**: On `client_id`, `carrier_id`, `commission_month`, `import_batch_id`, `status`

### `commission_deposits`

Bank deposits received from carriers.

| Column          | Type    | Description                          |
|-----------------|---------|--------------------------------------|
| `id`            | TEXT PK | UUID                                 |
| `carrier_id`    | TEXT FK | References `carriers.id`             |
| `deposit_month` | TEXT    | YYYY-MM                              |
| `deposit_amount`| REAL    | Total cash deposited                 |
| `deposit_date`  | TEXT    | Date of deposit                      |
| `reference`     | TEXT    | Check number or ACH reference        |
| `notes`         | TEXT    | Optional                             |
| `created_at`    | TEXT    | Auto-set                             |
| `updated_at`    | TEXT    | Auto-updated via trigger             |

Multiple deposits per carrier per month are allowed (unique constraint removed in v008).

---

## Status Values

| Status       | Color  | Meaning                                                      |
|--------------|--------|--------------------------------------------------------------|
| `OK`         | Green  | Statement amount matches expected rate (within $0.01)        |
| `UNDERPAID`  | Red    | Statement amount is less than expected rate                   |
| `OVERPAID`   | Orange | Statement amount exceeds expected rate                       |
| `MISSING`    | Yellow | Active enrollment exists but no commission entry for month   |
| `ZERO_RATE`  | Gray   | No rate configured or rate is $0.00                          |
| `UNMATCHED`  | Yellow | Statement entry could not be matched to a client             |
| `PENDING`    | Blue   | Imported but not yet reconciled                              |

---

## Plan Types

The system supports 21 plan types:

**Medicare Advantage**: MA, MAPD, DSNP, CSNP, ISNP, MMP, PACE, MSA, PFFS, COST

**Prescription Drug**: PDP

**Medicare Supplement**: MedSupA, MedSupB, MedSupC, MedSupD, MedSupF, MedSupG, MedSupK, MedSupL, MedSupM, MedSupN

---

## Key Business Logic

### Initial vs. Renewal Determination

During reconciliation, each entry is classified as initial or renewal by comparing the enrollment's effective date year to the commission month's year:

```
effective_date year == commission_month year  →  Initial (is_initial = 1)
effective_date year != commission_month year  →  Renewal (is_initial = 0)
```

### Rate Lookup

The expected rate is looked up from `commission_rates` using:
- `carrier_id` + `plan_type_code` + commission year

Then:
- If initial → use `initial_rate`
- If renewal → use `renewal_rate`

### Status Determination

```
client_id is NULL          →  UNMATCHED
no rate found              →  ZERO_RATE
expected_rate == 0.0       →  ZERO_RATE
|difference| < $0.01       →  OK
difference < 0             →  UNDERPAID  (received less than expected)
difference > 0             →  OVERPAID   (received more than expected)
```

Where `difference = statement_amount - expected_rate`.

### Member Matching (Statement Import)

When importing a carrier statement, the system attempts to match each row to a client in this priority order:

1. **MBI match** — exact match on `clients.mbi`
2. **Exact name + enrollment** — first/last name match with an active enrollment for the carrier
3. **Base first name + enrollment** — first word only (strips middle initial) + last name with enrollment
4. **Name match without enrollment** — first/last name match ignoring enrollment status
5. **Unmatched** — entry saved with `UNMATCHED` status and `client_id = NULL`

### Auto Column Mapping (Statement Import)

The import system recognizes common column header aliases:

| Field            | Recognized Headers                                              |
|------------------|-----------------------------------------------------------------|
| `member_name`    | member name, name, member, subscriber name, enrollee, etc.     |
| `member_id`      | member id, member number, subscriber id, id, mbi, hicn, etc.  |
| `statement_amount`| amount, commission, owed, statement amount, etc.               |
| `paid_amount`    | paid, paid amount, net amount, net commission, etc.            |
| `plan_type`      | plan type, product type, plan, lob, etc.                       |
| `commission_month`| month, commission month, period, service month, etc.           |

---

## Tauri Commands

### Rates

| Command                | Parameters                            | Returns                      |
|------------------------|---------------------------------------|------------------------------|
| `get_commission_rates` | `carrier_id?`, `plan_year?`           | `Vec<CommissionRateListItem>`|
| `create_commission_rate`| `input: CreateCommissionRateInput`   | `CommissionRateListItem`     |
| `update_commission_rate`| `id`, `input: UpdateCommissionRateInput` | `()`                    |
| `delete_commission_rate`| `id`                                 | `()`                         |

### Entries

| Command                   | Parameters                | Returns                        |
|---------------------------|---------------------------|--------------------------------|
| `get_commission_entries`  | `filters: CommissionFilters` | `Vec<CommissionEntryListItem>`|
| `update_commission_entry` | `id`, `input: UpdateCommissionEntryInput` | `()`              |
| `delete_commission_entry` | `id`                      | `()`                           |
| `delete_commission_batch` | `batch_id`                | `usize` (count deleted)        |

### Reconciliation

| Command                     | Parameters                      | Returns                       |
|-----------------------------|---------------------------------|-------------------------------|
| `reconcile_commissions`     | `carrier_id?`, `month?`        | `usize` (entries updated)     |
| `find_missing_commissions`  | `carrier_id`, `month`           | `usize` (entries created)     |
| `get_reconciliation_entries`| `filters: CommissionFilters`    | `Vec<ReconciliationRow>`      |
| `get_commission_summary`    | `month?`                        | `Vec<CarrierMonthSummary>`    |

### Statement Import

| Command                       | Parameters                                                      | Returns                 |
|-------------------------------|-----------------------------------------------------------------|-------------------------|
| `parse_commission_statement`  | `file_path`                                                     | `ParsedFile` (headers + sample rows) |
| `import_commission_statement` | `file_path`, `carrier_id`, `commission_month`, `column_mapping` | `StatementImportResult` |
| `import_commission_csv`       | `csv_content`, `carrier_id`, `commission_month`                | `StatementImportResult` |
| `trigger_commission_fetch`    | `carrier_id`                                                    | `()`                    |

### Deposits

| Command                      | Parameters                        | Returns                         |
|------------------------------|-----------------------------------|---------------------------------|
| `get_commission_deposits`    | `carrier_id?`, `month?`          | `Vec<CommissionDepositListItem>`|
| `create_commission_deposit`  | `input: CreateCommissionDepositInput` | `CommissionDeposit`        |
| `update_commission_deposit`  | `id`, `input`                     | `()`                            |
| `delete_commission_deposit`  | `id`                              | `()`                            |

---

## User Workflows

### 1. Set Up Commission Rates

**Tab**: Rates

1. Click **Add Rate**
2. Select Carrier, Plan Type, Plan Year
3. Enter Initial Rate and Renewal Rate ($/month)
4. Save

Rates must be configured before reconciliation can calculate expected amounts.

### 2. Import a Carrier Statement

**Tab**: Import

1. Select the **Carrier**
2. Select the **Commission Month** (YYYY-MM)
3. Click **Choose File** — select a CSV or XLSX statement file
4. Click **Import Statement**
5. Review results:
   - **Matched**: Successfully linked to clients
   - **Unmatched**: Member names that couldn't be matched (listed for manual review)
   - **Errors**: Rows that failed to process
6. If needed, click **Undo Import** to delete the entire batch

All imported entries start with `PENDING` (matched) or `UNMATCHED` status.

### 3. Reconcile Commissions

**Tab**: Reconciliation

1. Optionally filter by **Carrier**, **Month**, or **Status**
2. Click **Reconcile**
3. The system processes each entry:
   - Determines initial vs. renewal
   - Looks up expected rate
   - Calculates difference
   - Sets status (OK, UNDERPAID, OVERPAID, ZERO_RATE)
4. Review results — filter by status to focus on issues
5. Summary stats show total entries, OK count, and issue count

### 4. Find Missing Enrollments

**Tab**: Reconciliation

1. Select a specific **Carrier** and **Month**
2. Click **Find Missing**
3. The system checks all active enrollments for that carrier and creates `MISSING` entries for any that have no commission entry for the selected month

### 5. Record Deposits

**Tab**: Deposits

1. Click **Record Deposit**
2. Enter Carrier, Month, Deposit Amount, and optionally Date/Reference/Notes
3. Save
4. The table shows:
   - **Statement Total** — auto-calculated sum of `paid_amount` from commission entries
   - **Difference** — `deposit_amount - statement_total`
   - Green = balanced, Red = deposit less than statements, Orange = deposit more than statements

### 6. Review Carrier Summary

**Tab**: Summary

1. Optionally filter by **Month**
2. View aggregated data per carrier/month:
   - Expected vs. Statement vs. Paid totals
   - Deposit amount and deposit-vs-paid difference
   - Entry count, OK count, Issue count
3. Click any row to drill down to the Reconciliation tab for that carrier/month

---

## File Map

### Backend (Rust)

| File | Purpose |
|------|---------|
| `src-tauri/src/db/migrations/v007_commissions.sql` | Database schema (tables, indexes, triggers) |
| `src-tauri/src/models/commission.rs` | Data structs (Rate, Entry, Deposit, Summary, etc.) |
| `src-tauri/src/repositories/commission_repo.rs` | SQL queries and data access |
| `src-tauri/src/services/commission_service.rs` | Business logic (import, reconcile, matching) |
| `src-tauri/src/services/commission_importers/mod.rs` | Carrier-specific importer dispatch |
| `src-tauri/src/services/commission_importers/generic.rs` | CSV/XLSX import with auto-column-mapping |
| `src-tauri/src/services/commission_importers/humana.rs` | Pipe-delimited `.txt` Humana format |
| `src-tauri/src/commands/commission_commands.rs` | Tauri command handlers (RPC endpoints) |

### Frontend (React + TypeScript)

| File | Purpose |
|------|---------|
| `src/features/commissions/CommissionsPage.tsx` | Main page with tab navigation |
| `src/features/commissions/RatesTab.tsx` | Rate table management |
| `src/features/commissions/StatementImportTab.tsx` | Statement file import workflow |
| `src/features/commissions/ReconciliationTab.tsx` | Entry reconciliation and review |
| `src/features/commissions/CarrierSummaryTab.tsx` | Aggregated carrier/month summary |
| `src/features/commissions/DepositsTab.tsx` | Deposit recording and tracking |
| `src/features/commissions/components/RateFormDialog.tsx` | Rate add/edit form |
| `src/features/commissions/components/DepositFormDialog.tsx` | Deposit add/edit form |
| `src/features/commissions/components/StatusBadge.tsx` | Color-coded status badge |
| `src/features/commissions/components/EntryEditDialog.tsx` | Edit individual commission entries |
| `src/features/commissions/components/RawDataDialog.tsx` | View raw statement row data |
| `src/features/commissions/ActivityLog.tsx` | Import activity/history log |
| `src/hooks/useCommissions.ts` | React Query hooks for all commission operations |
| `src/types/index.ts` | TypeScript type definitions (lines ~370-519) |
