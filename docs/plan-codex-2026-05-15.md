# TJ Shop v1 Master Plan

Date: 2026-05-15

This is the current source-of-truth plan for starting v1. It reconciles:

- `docs/claude/reconciled-architecture-2026-05-15.md`
- `docs/codex/arch-init/reconciled-v1-architecture-seed-2026-05-15.md`

When this file disagrees with earlier architecture notes, use this file.

## Product Shape

TJ Shop v1 is a monorepo with a Rust backend and a TypeScript/React frontend.

- Backend: pure JSON API, Axum + SQLx + PostgreSQL + SQLx migrations.
- Frontend: TypeScript/React app consuming `/api/...`.
- Local development: Postgres in Docker/Podman Compose; backend and frontend run natively.
- Production: minimal VPS with native Postgres, Caddy, and one Rust binary supervised by systemd.

The backend does not server-render HTML, compile TypeScript, or own frontend routing.

## Repository Layout

```text
tj-shop/
  backend/
    Cargo.toml
    Cargo.lock
    .env.sample
    .sqlx/
    migrations/
      20260515000000_init.sql
    src/
      main.rs
      lib.rs
      state.rs
      config.rs
      db.rs
      error.rs
      telemetry.rs
      middleware.rs
      routes/
        mod.rs
        health.rs
        products.rs
        lists.rs
    tests/
      common/
        mod.rs
      fixtures/
      health.rs
      products.rs
      lists.rs

  frontend/
    package.json
    vite.config.ts
    src/
    public/

  justfile
  compose.yaml
  Caddyfile
  tj-shop.service
  README.md
  docs/
  v0/
```

Decisions:

- `backend/` and `frontend/` are siblings.
- No Cargo workspace for v1.
- Keep root infra files at the root: `compose.yaml`, `Caddyfile`, `tj-shop.service`, `justfile`.
- Add a Rust workspace only when there is a second Rust deployable.

## Backend Architecture

Use one Rust crate in `backend/`.

`main.rs` is thin bootstrap:

- load `.env` for local development
- parse `Config`
- initialize tracing
- create `PgPool`
- run SQLx migrations
- build router
- bind listener
- serve with graceful shutdown

`lib.rs` wires the app:

- module aggregation
- `pub fn router(state: AppState) -> Router`
- middleware stacking
- route merging

`state.rs`:

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: std::sync::Arc<crate::config::Config>,
}
```

Use `PgPool` directly. Do not add a `Db` wrapper or repository traits until there is real pressure.

## Configuration

Use hand-rolled env parsing with `dotenvy` in local development.

Initial config:

- `DATABASE_URL`
- `LISTEN_ADDR`, default `127.0.0.1:3000`
- `DB_POOL_MAX`, default `10`
- `APP_ENV`, default `dev`
- `RUST_LOG`

Skip `figment`, `config`, and similar frameworks for v1.

## API Routes

Use `/api/...` for application routes. Keep operational routes outside `/api`.

Initial route surface:

```text
GET   /healthz
GET   /api/products
POST  /api/products/refresh
GET   /api/lists
POST  /api/lists
GET   /api/lists/:id
PUT   /api/lists/:id/items
PATCH /api/lists/:id/items/:item_id
```

Use `/healthz` for the first slice because it is operational and should never be caught by frontend routing.

## Error Handling

Use `thiserror` for API errors, `anyhow` for internal context, and one `IntoResponse` implementation.

Shape:

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

Log real internal errors with `tracing::error!`. Return sanitized JSON, initially a flat body like:

```json
{ "error": "internal error" }
```

## Database And Migrations

Use SQLx migrations as the source of truth.

- migrations live in `backend/migrations/`
- run at startup with `sqlx::migrate!("./migrations")`
- run manually with `sqlx migrate run`
- tests run migrations through `#[sqlx::test]`

Postgres conventions:

- native `uuid`, not `char(36)`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`
- add an `updated_at` trigger helper in the first real schema migration if enough tables need it

Likely first domain tables:

- `products`
- `shopping_lists`
- `shopping_list_items`

Use direct SQLx queries. Inline SQL is acceptable in route modules for v1. Extract to helper functions or query files only when reuse or size justifies it.

## Testing And Agentic TDD

Backend development should be test-driven by the coding agent.

Default workflow:

```text
1. Write or update a failing API or repository test.
2. Run the narrow test and confirm the expected failure.
3. Implement the smallest backend change.
4. Run the narrow test until green.
5. Run backend checks.
6. Update docs when behavior or architecture changes.
```

Use `#[sqlx::test]` for database-backed integration tests:

- fresh database per test
- migrations run automatically
- test receives a `PgPool`
- exercise the router with `tower::ServiceExt::oneshot`
- no real TCP listener in tests
- no database mocking
- fixtures live in `backend/tests/fixtures/`

Agent-specific rules:

- Do not weaken a test just to make it pass.
- Do not bundle a large untested feature behind one broad test.
- Schema work may need a brief design pass, but implementation still lands through tests.
- Prefer narrow commands while iterating and a full check before handoff.

## justfile

Use a root `justfile` as the command contract for humans and agents.

Initial recipes:

```just
default:
    @just --list

db-up:
    docker compose up -d db

db-down:
    docker compose down

db-reset:
    docker compose down -v
    docker compose up -d db

backend-run:
    cd backend && cargo run

backend-test:
    cd backend && cargo test

backend-check:
    cd backend && cargo fmt --check
    cd backend && cargo clippy --all-targets -- -D warnings
    cd backend && cargo test

frontend-dev:
    cd frontend && npm run dev

frontend-build:
    cd frontend && npm run build

check:
    just backend-check
    just frontend-build
```

`justfile` is preferred over `Makefile` because this repo needs a command runner, not a dependency graph.

## Local Development

Run three local processes:

```text
repo root: just db-up
backend/: cargo run
frontend/: npm run dev
```

`compose.yaml` is for local Postgres only:

```yaml
services:
  db:
    image: postgres:16
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: tj_shop
      POSTGRES_PASSWORD: tj_shop
      POSTGRES_DB: tj_shop
    volumes:
      - db_data:/var/lib/postgresql/data

volumes:
  db_data:
```

Use Vite proxying so frontend dev requests to `/api/*` forward to the Rust backend. That keeps development close to production and minimizes CORS.

## VPS Production

Use native services, not Docker.

Topology:

```text
Internet -> Caddy :443
  /api/* -> Rust backend on 127.0.0.1:3000
  /*     -> frontend static files in /var/www/tj-shop

Rust backend -> Postgres on 127.0.0.1:5432
```

Caddy responsibilities:

- TLS
- compression
- static frontend serving
- SPA fallback for frontend routes
- reverse proxy for `/api/*`

Important routing rule:

- SPA fallback must never catch `/api/*`.

Rust responsibilities:

- bind to loopback
- expose JSON API
- run migrations on startup
- log to stdout/stderr for journald

systemd responsibilities:

- load `/etc/tj-shop.env`
- restart on failure
- start after Postgres

## Shared VPS Postgres

One native Postgres install can serve multiple small apps.

Rules:

- one database per app
- one role per app
- no app connects as `postgres`
- each app owns only its database
- each app runs only its migrations
- keep pool sizes conservative, usually `5` or `10`

Example:

```text
postgres server
  database: tj_shop
    role: tj_shop
  database: another_app
    role: another_app
```

No PgBouncer for v1.

## Dependencies

Initial backend dependencies:

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower = "0.5"
tower-http = { version = "0.6", features = [
  "trace", "cors", "timeout", "request-id", "normalize-path", "sensitive-headers"
] }

sqlx = { version = "0.8", features = [
  "runtime-tokio", "tls-rustls-ring-native-roots", "postgres", "macros", "migrate",
  "uuid", "time", "json",
] }

thiserror = "2"
anyhow = "1"

serde = { version = "1", features = ["derive"] }
serde_json = "1"

dotenvy = "0.15"

tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

uuid = { version = "1", features = ["v7", "serde"] }
time = { version = "0.3", features = ["serde"] }

[dev-dependencies]
http-body-util = "0.1"
tower = { version = "0.5", features = ["util"] }
serde_json = "1"
```

Deferred dependencies:

- OpenAPI tooling
- auth libraries
- Redis
- background job crates
- heavy validation frameworks
- clean-architecture/service-trait scaffolding

## First Implementation Cycle

Start with a walking skeleton.

Acceptance criteria:

- `backend/` crate exists.
- `compose.yaml` starts local Postgres.
- `backend/.env.sample` documents local config.
- `GET /healthz` returns JSON.
- API integration test for `/healthz` uses `#[sqlx::test]`.
- router is built from library code, not duplicated in tests.
- migrations directory exists and startup migration flow is wired.
- `just backend-check` passes.

Suggested `/healthz` response:

```json
{ "status": "ok" }
```

## Second Implementation Cycle

Begin the first domain slice: products.

Before coding, resolve product source of truth:

- seed from existing v0 JSON into Postgres
- manual import command
- `POST /api/products/refresh` from TJ GraphQL
- scheduled refresh later

Then implement test-first:

1. product migration
2. product DTO/model
3. failing `GET /api/products` test
4. minimal implementation
5. fixture or seed strategy

## Deferred Decisions

Do not block the first `/healthz` cycle on these:

- SQLx offline mode and committed `.sqlx/`
- product data refresh strategy
- auth/write protection
- OpenAPI
- background jobs
- deployment build method: build on VPS vs cross-compile and rsync
- backups/deploy runbook

## Source References

- `docs/claude/reconciled-architecture-2026-05-15.md`
- `docs/codex/arch-init/reconciled-v1-architecture-seed-2026-05-15.md`
- `docs/claude/recommended-layout-2026-05-15.md`
- `docs/codex/arch-init/architecture-seed-axum-sqlx-postgres-2026-05-15.md`
