# SHEEPS

**Medicare Book of Business Manager**

`Tauri 2.x` `React 18` `TypeScript` `Rust` `SQLCipher`

A local-first, HIPAA-compliant desktop application for Medicare insurance agents to manage their book of business — clients, enrollments, carrier file imports, and reporting.

## Features

- **Client CRM** — Full client records with Medicare-specific fields (MBI, Part A/B dates, dual eligibility, LIS level)
- **Enrollment tracking** — Track plan enrollments across carriers with status lifecycle management
- **Carrier file import** — Import CSV/XLSX files with auto-mapping columns to client fields, validation, and insert/update logic
- **Dashboard analytics** — At-a-glance stats and charts for your book of business
- **PDF reports** — Generate and export reports
- **Encrypted local storage** — All data encrypted at rest with SQLCipher; no cloud, no plaintext files
- **Command palette** — Quick navigation with `Ctrl+K`
- **Full-text search** — Search clients by name, MBI, phone, email, city, or zip

## Tech Stack

| Layer      | Technology                                                  |
| ---------- | ----------------------------------------------------------- |
| Framework  | Tauri 2.x                                                   |
| Frontend   | React 18, TypeScript, Vite                                  |
| UI         | Tailwind CSS, shadcn/ui, Radix UI, Recharts, Lucide icons   |
| State      | Zustand, TanStack Query, React Hook Form + Zod              |
| Backend    | Rust, rusqlite + SQLCipher, Argon2, calamine, genpdf         |
| Database   | SQLite with SQLCipher encryption, FTS5 full-text search      |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Bun](https://bun.sh/) (JavaScript runtime / package manager)
- System dependencies for Tauri:
  - **Arch Linux**: `webkit2gtk-4.1`
  - **Ubuntu/Debian**: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Install & Run

```bash
# Install frontend dependencies
bun install

# Run in development mode
bun tauri dev

# Build for production
bun run build
```

> **Wayland/Hyprland users**: Set `WEBKIT_DISABLE_DMABUF_RENDERER=1` before running to avoid GBM buffer errors:
> ```bash
> WEBKIT_DISABLE_DMABUF_RENDERER=1 bun tauri dev
> ```

## Project Structure

```
sheeps/
├── src/                          # Frontend (React + TypeScript)
│   ├── app/                      # App shell, router, providers
│   ├── components/
│   │   ├── layout/               # AppLayout, CommandPalette
│   │   └── ui/                   # shadcn/ui primitives
│   ├── features/                 # Feature modules
│   │   ├── auth/                 # Login page
│   │   ├── clients/              # Client list, detail, form
│   │   ├── dashboard/            # Dashboard analytics
│   │   ├── enrollments/          # Enrollment management
│   │   ├── import/               # File import wizard
│   │   ├── reports/              # Report generation
│   │   └── settings/             # App settings
│   ├── hooks/                    # TanStack Query hooks
│   ├── stores/                   # Zustand stores
│   ├── lib/                      # Utilities, Tauri invoke wrapper
│   └── types/                    # TypeScript type definitions
├── src-tauri/                    # Backend (Rust)
│   └── src/
│       ├── commands/             # Tauri IPC command handlers
│       ├── services/             # Business logic
│       ├── repositories/         # SQL data access
│       ├── models/               # Data structures
│       ├── db/                   # Connection, migrations, seed data
│       └── error.rs              # Error types
├── CLAUDE.md                     # AI assistant instructions
├── DEVELOPMENT.md                # Developer reference
└── package.json
```

## Security

- **Encryption at rest** — SQLCipher encrypts the entire database file
- **Key derivation** — Argon2id (64 MB memory, 3 iterations, 4 parallelism) derives a 32-byte key from the user's password
- **No plaintext storage** — Database is inaccessible without the correct password; no separate auth system
- **Local-only** — All data stays on the user's machine; no network calls, no telemetry

## License

Copyright (c) 2026 MedStar. All rights reserved.

This is proprietary software. Unauthorized copying, distribution, or modification is prohibited.
