# TJ Shop v1 Reconciled Architecture Seed

Date: 2026-05-15

This reconciles the architecture work in `docs/codex/arch-init/` and `docs/claude/`.
Use this as the working seed for scaffolding v1.

## Scope

TJ Shop v1 is a monorepo with:

- `backend/`: Rust JSON API using Axum, SQLx, PostgreSQL, Tokio, and SQLx migrations.
- `frontend/`: TypeScript/React app, likely Vite, consuming `/api/...`.
- Local development on a laptop with containerized Postgres.
- Production on a minimal VPS with native Postgres, Caddy, and a systemd-managed Rust binary.

The Rust backend remains a pure JSON API. It does not server-render HTML and does not compile the frontend.

## Repo Layout

Use two functional application directories and keep small infra files at the repo root:

```text
tj-shop/
  backend/
    Cargo.toml
    Cargo.lock
    .env.sample
    .sqlx/
    migrations/
    src/
      main.rs
      lib.rs
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
      health.rs
      products.rs
      lists.rs
      fixtures/

  frontend/
    package.json
    vite.config.ts
    src/
    public/

  Caddyfile
  compose.yaml
  tj-shop.service
  README.md
  docs/
    claude/
    codex/
  v0/
```

Notes:

- No Cargo workspace for v1. `backend/` is one standalone Rust crate.
- No `deploy/` directory yet. `Caddyfile`, `compose.yaml`, and `tj-shop.service` are one-file root concerns.
- Add a workspace only when there is a second Rust deployable, such as a CLI or worker.

## Backend Shape

Use a single binary crate with library-backed router construction.

`main.rs`:

- Load `backend/.env` with `dotenvy::dotenv().ok()` for local development.
- Build `Config::from_env()`.
- Initialize tracing.
- Build a `PgPool`.
- Run `sqlx::migrate!("./migrations").run(&pool)`.
- Build the router from `lib.rs`.
- Bind to `127.0.0.1:3000` by default.
- Serve with graceful shutdown.

`lib.rs`:

- Define `AppState`.
- Expose `pub fn router(state: AppState) -> Router`.
- Wire route modules and middleware in one place.

Use `routes/` rather than `http/` for endpoint modules. It reads better in this app because the API surface is small and resource-oriented.

## AppState

Use concrete state, not trait-heavy dependency injection:

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: std::sync::Arc<crate::config::Config>,
}
```

Do not introduce a `Db` wrapper initially. `PgPool` is already cheap to clone. Put pool construction and migration helpers in `db.rs` as free functions.

## Configuration

Use hand-rolled env parsing:

```rust
pub struct Config {
    pub database_url: String,
    pub listen_addr: std::net::SocketAddr,
    pub db_pool_max: u32,
    pub app_env: AppEnv,
}

pub enum AppEnv {
    Dev,
    Prod,
}
```

Expected initial env vars:

- `DATABASE_URL`
- `LISTEN_ADDR`, default `127.0.0.1:3000`
- `DB_POOL_MAX`, default `10`
- `APP_ENV`, default `dev`
- `RUST_LOG`, handled by tracing subscriber

Skip `figment`, `config`, and similar config frameworks for v1.

## API Surface

Use `/api/...` for application endpoints and keep frontend routes outside `/api`.

Initial API:

```text
GET  /health
GET  /api/products
POST /api/products/refresh
GET  /api/lists
POST /api/lists
GET  /api/lists/:id
PUT  /api/lists/:id/items
PATCH /api/lists/:id/items/:item_id
```

Default to `/health` for readability. If the app later moves into Kubernetes-style infrastructure, add `/livez` and `/readyz` then.

## Error Handling

Use a `thiserror` enum plus one `IntoResponse` implementation:

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

Log real internal errors with `tracing::error!`, but return sanitized JSON. A flat body like `{ "error": "..." }` is enough for v1.

## Database And Migrations

Use SQLx migrations as the schema source of truth:

- `backend/migrations/`
- `sqlx migrate add <name>`
- startup migrations with `sqlx::migrate!("./migrations")`
- `sqlx migrate run` for manual local checks

Initial tables will likely be:

- `products`
- `shopping_lists`
- `shopping_list_items`

Adopt simple Postgres conventions:

- Native `uuid` columns, not `char(36)`.
- `created_at timestamptz not null default now()`.
- `updated_at timestamptz not null default now()`.
- Add an `updated_at` trigger helper in the first migration if enough tables need it.

Use direct SQLx queries. Do not add repository traits or ORM-style abstractions until there is real pressure.

## Testing And Agent TDD

Backend development should be test-driven by the coding agent.

Agent workflow:

```text
1. Write or update a failing API or repository test.
2. Run the narrow test and confirm the expected failure.
3. Implement the smallest backend change.
4. Run the narrow test until green.
5. Run fmt, clippy, and the full backend test suite.
6. Update docs when behavior or architecture changes.
```

Use `#[sqlx::test]` for database-backed integration tests:

- Each test gets a fresh database.
- Migrations run automatically.
- The test receives a `PgPool`.
- Exercise the Axum router with `tower::ServiceExt::oneshot`, not a real TCP listener.
- Put SQL fixtures in `backend/tests/fixtures/`.

This is the strongest addition from the Claude notes and should replace hand-rolled random database helpers.

## Local Development

Run three local processes:

```text
repo root: docker compose up -d
backend/: cargo run
frontend/: npm run dev
```

Use `compose.yaml` only for local Postgres:

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

Frontend dev should proxy `/api/*` to `127.0.0.1:3000`. With that setup, the browser sees same-origin API calls in development and CORS should rarely matter.

## VPS Production

Use native services, not Docker:

```text
Internet -> Caddy :443
  /api/* -> Rust backend on 127.0.0.1:3000
  /*     -> frontend static files in /var/www/tj-shop

Rust backend -> Postgres on 127.0.0.1:5432
```

Caddy is the preferred front door:

- TLS termination via Let's Encrypt.
- Static React build serving.
- Reverse proxy for `/api/*`.
- Compression and cache headers outside the Rust app.

Rust should bind loopback only in production. Caddy is the public interface.

Run the Rust binary with systemd:

- `EnvironmentFile=/etc/tj-shop.env`
- `Restart=on-failure`
- `After=network.target postgresql.service`
- stdout/stderr to journald

## Shared VPS Postgres

Multiple small apps can share one native Postgres install.

Rules:

- One database per app.
- One role per app.
- App code never connects as the `postgres` superuser.
- Each app's SQLx migrations target only its own database.
- Keep SQLx pool sizes conservative, e.g. `max_connections(5)` or `10`.

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

Initial backend dependency set:

```toml
[dependencies]
anyhow = "1"
axum = "0.8"
dotenvy = "0.15"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = [
  "runtime-tokio-rustls",
  "postgres",
  "macros",
  "migrate",
  "uuid",
  "chrono",
  "json",
] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.6", features = [
  "trace",
  "cors",
  "timeout",
  "request-id",
  "normalize-path",
] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
http-body-util = "0.1"
```

Notes:

- Prefer `chrono` for v1 because it is common and well-supported. `time` would also be fine.
- Skip OpenAPI, Redis, JWT, Argon2, background jobs, and service/repository traits for the first scaffold.

## First Vertical Slice

Start with a walking skeleton:

1. Create `backend/` Rust crate.
2. Add `Config`, tracing, `PgPool`, migrations, and graceful shutdown.
3. Add `GET /health` returning JSON.
4. Add `#[sqlx::test]` integration test for `/health`.
5. Add `compose.yaml` for local Postgres.
6. Add `.env.sample`.
7. Add root helper commands later if repetition becomes annoying.

Then add the first real domain slice:

1. `products` migration.
2. Product model/DTO.
3. `GET /api/products` test.
4. Minimal implementation.
5. Product seed/import decision.

## Deferred Decisions

Keep these out of the first scaffold unless they become blockers:

- SQLx offline mode and committed `.sqlx/`.
- OpenAPI generation.
- Auth or write protection.
- Product catalog source of truth and refresh strategy.
- Background jobs.
- Whether deployment builds on the VPS or ships a locally/cross-compiled binary.

## What This Reconciles

From Codex notes, keep:

- Backend as pure JSON API.
- `frontend/` + `backend/` monorepo.
- Local containerized Postgres, native VPS services.
- Shared VPS Postgres with one database/role per app.
- Conservative API-first vertical slicing.

From Claude notes, keep:

- Caddy as the preferred production front door.
- Root `compose.yaml`, `Caddyfile`, and `tj-shop.service`.
- `#[sqlx::test]` as the integration testing default.
- `PgPool` directly in `AppState`.
- `routes/` layout, `telemetry.rs`, and `middleware.rs`.
- Request IDs, timeout, path normalization, and tracing as early middleware.
- Launchbadge-style schema conventions.

Do not copy:

- Heavy clean architecture/DDD layering.
- Repository traits for every table.
- Redis/auth/OpenAPI before requirements exist.
- Static asset serving from Rust unless Caddy becomes undesirable.
