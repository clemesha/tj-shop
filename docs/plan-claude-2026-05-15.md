# tj-shop v1 — master plan

Single source of truth for v1 architecture and scaffolding decisions.
Synthesized from two independently-reconciled architecture seeds:

- `docs/claude/arch-init/reconciled-architecture-2026-05-15.md` (Claude agent)
- `docs/codex/arch-init/reconciled-v1-architecture-seed-2026-05-15.md` (Codex agent)

When this plan disagrees with either source, **this plan wins**. The
sources remain useful for fuller rationale on individual choices.

## What v1 is

- Rust JSON API backend in `backend/` (Axum + SQLx + Postgres 16 + sqlx
  migrations + Tokio).
- TypeScript/React frontend in `frontend/` (Vite), consuming `/api/...`.
- Local dev on a laptop with **containerized Postgres** for the database
  only; backend and frontend run natively.
- Production on **a single minimal VPS** with **native** Postgres, Caddy
  reverse-proxying, and systemd supervising the Rust binary.
- The backend is pure JSON — never renders HTML, never bundles or
  compiles the frontend.

Deferred from v1: auth, OpenAPI, Redis, server-rendered HTML, background
jobs, distributed tracing, repository traits, clean-architecture layering.

## Repo layout

```
tj-shop/
├── backend/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── .env.sample
│   ├── .sqlx/                       # commit after `cargo sqlx prepare` (when CI lands)
│   ├── migrations/
│   ├── src/
│   └── tests/
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
├── Caddyfile                        # prod: Caddy fronts both apps on the VPS
├── compose.yaml                     # dev only: containerized Postgres
├── tj-shop.service                  # prod: systemd unit
├── justfile                         # canonical command shortcuts (for humans + agents)
├── README.md
├── docs/
│   ├── claude/
│   └── codex/
└── v0/                              # archived legacy app
```

- **Two functional dirs** (`backend/`, `frontend/`). Everything else is
  small enough to live at the top level.
- **No Cargo workspace.** Single crate at `backend/`. Reconsider only if
  a second deployable appears.
- **No `deploy/` directory.** Three infra files at root.

## Backend module layout

```
backend/src/
├── main.rs              # thin bootstrap: env, tracing, db, migrate, serve
├── lib.rs               # AppState + pub fn router(state) -> Router
├── config.rs            # Config struct + from_env()
├── db.rs                # PgPool factory + sqlx::migrate!() wrapper
├── error.rs             # ApiError + IntoResponse + Result alias
├── telemetry.rs         # tracing-subscriber init + TraceLayer factory
├── middleware.rs        # request-id, timeout, normalize-path, dev-only CORS
└── routes/
    ├── mod.rs           # builds the /api Router; mounts /health at root
    ├── health.rs        # GET /health
    ├── products.rs
    └── lists.rs
```

- `routes/` flat. Inline `sqlx::query!` in handlers; extract to a `repos/`
  module only when a query is shared by two handlers.
- `AppState` lives in `lib.rs`, not a separate file — one less file, no
  discoverability cost at this scale.
- Domain types live in route files until two files need the same one;
  then promote to a flat `domain.rs`.

## AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: std::sync::Arc<crate::config::Config>,
}
```

- `PgPool` is internally `Arc`'d; clone is cheap.
- `Config` in `Arc` to avoid cloning `String`-heavy struct per request.
- No `Db` wrapper; `PgPool` is already an Arc'd handle.

## Configuration

Hand-rolled `std::env::var` + `dotenvy`. No `config` / `figment` crate.

```rust
pub struct Config {
    pub database_url: String,
    pub listen_addr: std::net::SocketAddr,
    pub db_pool_max: u32,
    pub app_env: AppEnv,
}
pub enum AppEnv { Dev, Prod }
```

Env vars:
- `DATABASE_URL` — required.
- `LISTEN_ADDR` — default `127.0.0.1:3000`.
- `DB_POOL_MAX` — default `10`.
- `APP_ENV` — default `dev`.
- `RUST_LOG` — read by tracing subscriber.

## API surface

Application routes under `/api/`. Operational endpoint at `/health` (not
`/healthz` — simpler reading; add `/livez` / `/readyz` only if K8s shows up).

```
GET   /health
GET   /api/products
POST  /api/products/refresh
GET   /api/lists
POST  /api/lists
GET   /api/lists/:id
PUT   /api/lists/:id/items
PATCH /api/lists/:id/items/:item_id
```

Rationale for the `/api/` prefix: load-bearing for Caddy's split between
backend proxy (`handle /api/*`) and the React SPA fallback
(`try_files {path} /index.html`).

## Error handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("not found")]
    NotFound,
    #[error("invalid request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error")]
    Sqlx(#[from] sqlx::Error),
    #[error("internal error")]
    Anyhow(#[from] anyhow::Error),
}

pub type Result<T, E = ApiError> = std::result::Result<T, E>;
```

One `IntoResponse` impl maps each variant to a status code and returns a
flat `{"error": "..."}` JSON body. Internal errors log via
`tracing::error!` with the real message before sanitizing the response.

## Database and migrations

- Schema source of truth: `backend/migrations/`, managed by `sqlx-cli`.
- Migrations applied at startup via `sqlx::migrate!("./migrations")`.
  Idempotent; first VPS boot applies all, subsequent boots no-op.
- Native Postgres `uuid` columns (not `char(36)`).
- Every table: `created_at timestamptz not null default now()`,
  `updated_at timestamptz not null default now()`. Trigger helper for
  `updated_at` in the first migration once two tables need it.
- Direct SQLx queries via `query!` / `query_as!`. No repository traits,
  no ORM patterns. `query_file!` as escape hatch for queries that get
  large.

Likely initial tables:
- `products` — TJ catalog cache + manual rows
- `shopping_lists` — list metadata
- `shopping_list_items` — products in lists, with quantities, checked
  state, ordering

## Testing — agent-driven TDD

Backend development proceeds test-first, driven by the coding agent.

Cadence (per endpoint):
1. User writes the spec — 3–5 lines: path, method, request shape,
   response shape, error cases.
2. Agent writes failing integration test(s). Happy path + 1–2 error
   cases. No impl.
3. User reviews the test. Gate point.
4. Agent writes migration (if needed) and handler — minimum to pass.
5. Agent runs `just backend-test`, observes green, reports.
6. Optional refactor pass with tests as safety net.

Rules to encode in `backend/CLAUDE.md` before any code lands:
- Tests before impl. No same-turn test+impl.
- Never weaken a test to make it pass. Escalate ("I think the test is
  wrong because X") instead.
- Body assertions required — not just status codes.
- Integration tests use `#[sqlx::test]` against real Postgres. No DB
  mocking.
- Schema/migration design is the only TDD carve-out (exploratory until
  shape stabilizes).
- Task is not "done" until `just backend-test` is green.

Test mechanics:
- `#[sqlx::test]` gives each test a fresh Postgres database with
  migrations applied; the handler receives a `PgPool`.
- Exercise the Axum router via `tower::ServiceExt::oneshot` — no TCP.
- SQL fixtures live in `backend/tests/fixtures/`.

## Local development

Three processes:

```
repo root:   docker compose up -d         # Postgres only
backend/:    cargo run                    # 127.0.0.1:3000
frontend/:   npm run dev                  # 127.0.0.1:5173, proxies /api/* to :3000
```

Vite's `/api/*` proxy means the browser sees same-origin in dev — CORS
should not fire in normal use. The `cors_layer()` is wired only when
`APP_ENV=dev` as a safety net.

`compose.yaml` runs Postgres 16, port-mapped to host `5432`, with the
`tj_shop` user/password/database matching `backend/.env.sample`'s
`DATABASE_URL`. Data persists in a named Docker volume; wipe with
`just db-reset`.

## VPS production

Topology:

```
Internet ──TLS──▶ Caddy (:443)
                   ├── /api/*  ──▶  Rust (127.0.0.1:3000) ──▶ Postgres (127.0.0.1:5432)
                   └── /*      ──▶  /var/www/tj-shop/ (built React)
```

- Caddy: TLS via Let's Encrypt, serves `frontend/dist/` contents,
  reverse-proxies `/api/*` to Rust.
- Rust binds `127.0.0.1:3000` only. Caddy is the public interface.
- Postgres on the same box, loopback only. `apt install postgresql-16`.
- Rust supervised by systemd: `EnvironmentFile=/etc/tj-shop.env`,
  `Restart=on-failure`, `After=network.target postgresql.service`,
  stdout/stderr to journald.

Deploy mechanic:
- **If VPS ≥ 4 GB RAM**: `git pull && cargo build --release && systemctl restart tj-shop`.
- **If VPS 1–2 GB RAM**: cross-compile with `cargo-zigbuild` on macOS,
  rsync the binary. Native compile will OOM-kill on 1 GB.
- The deployable is one binary + `backend/migrations/`. Nothing else.
- Frontend: `npm run build` then rsync `frontend/dist/` to
  `/var/www/tj-shop/`.

## Shared VPS Postgres

Postgres is multi-tenant by design. The VPS Postgres install is shared
across all small apps:

- One database per app. tj-shop owns `tj_shop`.
- One role per app. `tj_shop` role owns the `tj_shop` database.
- App code never connects as `postgres` superuser.
- Each app's `sqlx::migrate!()` is scoped to its own database.
- `PgPoolOptions::max_connections(10)` per app. Default
  `max_connections=100`, so ~10 apps fit before tuning.
- Caddy gains another site block (or another `handle` inside one) per
  new app domain or path prefix.
- Each app has its own systemd unit, all `Requires=postgresql.service`.

Local dev across multiple apps: easiest is each app's `compose.yaml` on
a different host port (`5433`, `5434`, …). Consolidate into one shared
laptop-wide Postgres only if managing N containers gets annoying.

## Dependencies

```toml
[dependencies]
axum         = "0.8"
tokio        = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower        = "0.5"
tower-http   = { version = "0.6", features = [
  "trace", "cors", "timeout", "request-id", "normalize-path", "sensitive-headers"
] }

sqlx = { version = "0.8", features = [
  "runtime-tokio-rustls", "postgres", "macros", "migrate",
  "uuid", "chrono", "json",
] }

thiserror = "2"
anyhow    = "1"

serde       = { version = "1", features = ["derive"] }
serde_json  = "1"

dotenvy = "0.15"

tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

uuid   = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
http-body-util = "0.1"
tower          = { version = "0.5", features = ["util"] }
serde_json     = "1"
```

Notes:
- Time crate: **`chrono`**. Either crate works; chrono picked for wider
  ecosystem support. Mechanical to switch if you change your mind.
- TLS: legacy-spelled `runtime-tokio-rustls` feature. The newer
  `runtime-tokio` + `tls-rustls-ring-native-roots` split also works
  identically; pick the older spelling because it matches most reference
  repos and is fewer features to remember.
- `request-id` and `normalize-path` features enabled day-one (request
  correlation is high-value the first time concurrent traffic shows up).
- `sensitive-headers` included preemptively for when auth lands.

## `justfile` (canonical commands)

```just
default:
    @just --list

db-up:
    docker compose up -d db

db-down:
    docker compose down

db-reset:
    docker compose down -v && docker compose up -d db

backend-run:
    cd backend && cargo run

backend-test:
    cd backend && cargo test

backend-check:
    cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings

frontend-dev:
    cd frontend && npm run dev

frontend-build:
    cd frontend && npm run build
```

Agents discover commands via `just --list`. New commands land here, not
in scattered shell scripts or README prose.

## First vertical slice

Order of operations:

1. Create `backend/` Rust crate (`cargo new backend --lib && cd backend && cargo add ...`).
2. Implement `Config::from_env()`, tracing init, `PgPool` factory,
   migration runner, graceful shutdown.
3. Add `routes::health::router()` returning `Json(serde_json::json!({"status": "ok"}))`.
4. Add `backend/tests/health.rs` with a `#[sqlx::test]` that exercises
   `GET /health` via `tower::ServiceExt::oneshot` and asserts status +
   body shape. Test goes red first, then green.
5. Write `compose.yaml` for local Postgres; write `.env.sample`.
6. Write `justfile` with the commands above.
7. Write `backend/CLAUDE.md` with the TDD rules.
8. Run `just db-up && just backend-test`. Green = slice complete.

End state: app boots, test runner works against compose Postgres,
discipline rules live in version control, plumbing proven.

## Then: first real domain slice

1. Resolve Q5 below (product data source).
2. Write the first migration creating `products`.
3. Write failing `GET /api/products` integration test.
4. Implement handler with inline `sqlx::query!`.
5. Test green. Commit.

## Deferred decisions (not blocking the first slice)

- **sqlx offline mode** (`cargo sqlx prepare` + committed `.sqlx/`).
  Recommended on when CI lands; not before.
- **CI pipeline.** Standard shape: Postgres service, `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`. Wire it
  when there's a CI provider chosen.
- **OpenAPI generation.** Skip for v1.
- **Auth.** Skip for v1. If a write endpoint should not be world-writable,
  gate it with a single `Bearer <token>` against an env var — five lines
  of middleware, no auth library.
- **Background jobs / scheduled sync.**
- **Repository abstraction.** Add only if queries get reused across
  routes.
- **`time` vs `chrono`.** Pick chrono; revisit only if a downstream
  crate forces the choice.

## Open items requiring user input

These genuinely block forward motion or have no defensible default:

1. **Product data source of truth (Q5).** Three options:
   - One-off `seed.sql` migration with v0's `products.json` scrape baked
     in.
   - `POST /api/products/refresh` endpoint pulling live from TJ GraphQL
     on demand.
   - Scheduled background sync from TJ GraphQL.

   Decision needed before the first domain cycle (not before the
   `/health` slice).

2. **VPS specs** — locks in the deploy mechanic default (build-on-box vs
   cross-compile + rsync).

3. **Postgres major version** — recommended 16, both locally (compose
   image tag) and on VPS (apt package).

## Source documents

- `docs/claude/arch-init/reconciled-architecture-2026-05-15.md` — Claude's
  reconciliation (concrete `IntoResponse`, RAM-aware deploy, multi-app
  Postgres connection budget, agent TDD failure modes).
- `docs/codex/arch-init/reconciled-v1-architecture-seed-2026-05-15.md` — Codex's
  reconciliation (initial schema sketch, `/api/` prefix rationale,
  refresh endpoint as data-source hint).
- `docs/claude/arch-init/recommended-layout-2026-05-15.md` — Claude's initial
  layout (most detailed module rationale, error code).
- `docs/codex/arch-init/architecture-seed-axum-sqlx-postgres-2026-05-15.md` —
  Codex's initial seed (reference repo survey, broader deferred list).
- `docs/claude/arch-init/reference-repos-2026-05-15.md` — Six surveyed repos with
  patterns to copy and skip.
- `docs/claude/arch-init/open-questions-2026-05-15.md` — Genuine forks remaining.

---

*Last updated: 2026-05-15*
