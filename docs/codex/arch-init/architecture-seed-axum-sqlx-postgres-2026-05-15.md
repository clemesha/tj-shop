# TJ Shop v1 Architecture Seed: Axum + SQLx + Postgres

Date: 2026-05-15

## Goal

Seed v1 with a boring, high-quality Rust web architecture:

- Axum for HTTP routing, extractors, middleware, and server bootstrap.
- SQLx for direct SQL, compile-time checked queries where practical, and migrations.
- PostgreSQL as the durable app database.
- A pure JSON API consumed by a TypeScript/React frontend served from the same repo/VPS.
- A simple module layout that can grow without becoming a framework inside the app.
- Fast local iteration, with production deployment to a minimal VPS.

This repo is currently mostly archived v0 static/App Script work, so treat this as greenfield guidance for the v1 backend.

Out of scope for this note:

- React, TypeScript, bundling, routing, state management, or frontend file layout.
- Server-rendered HTML.
- Frontend implementation details. Deployment shape matters here only where it affects backend routing, CORS, and static asset handoff.

## Reference Apps Reviewed

### 1. `davidpdrsn/realworld-axum-sqlx`

Source: <https://github.com/davidpdrsn/realworld-axum-sqlx>

Why it matters:

- Fork of Launchbadge's RealWorld Axum/SQLx app, updated by an Axum maintainer.
- Uses Axum 0.7-era APIs, `axum::serve`, `State`, graceful shutdown, and Tower HTTP middleware.
- Keeps `main.rs` and `lib.rs` split so helper binaries can share app code later.
- Uses a single shared SQLx `PgPool`.
- Runs embedded migrations at startup with `sqlx::migrate!()`.
- Organizes HTTP by route/domain modules plus shared HTTP error/extractor/types modules.

Borrow:

- `main.rs` should do process bootstrap only: load `.env`, initialize logging, parse config, connect DB, run migrations, then call into library code.
- `lib.rs` should expose app modules and router construction.
- App state should be one cloneable struct containing config and `PgPool`.
- Routes should live in domain-oriented modules rather than a single giant route file.
- Add graceful shutdown from day one.

Do not blindly copy:

- The RealWorld schema/domain is unrelated.
- The older commentary-heavy style is useful for learning, but v1 should keep comments sparse.
- License is AGPL on the original Launchbadge repo; use as architecture reference, not source copy.

### 2. `launchbadge/realworld-axum-sqlx`

Source: <https://github.com/launchbadge/realworld-axum-sqlx>

Why it matters:

- Written by SQLx maintainers as a best-practice exploration for Axum + SQLx + PostgreSQL.
- README explicitly recommends starting with `migrations/`, then `main.rs`, then module traversal.
- Uses `sqlx-cli` and `sqlx db setup` for local schema setup.
- Embeds migrations in the binary and applies them on startup.

Borrow:

- Treat migrations as first-class architecture, not an afterthought.
- Keep database schema comments useful and intentional.
- Prefer environment-based configuration with `.env` only for local development.

Do not blindly copy:

- It uses older Axum APIs in the upstream repo. Prefer the maintained fork for modern syntax.
- It chooses `mod.rs` module style. That is defensible, but not required.

### 3. `koskeller/axum-postgres-template`

Source: <https://github.com/koskeller/axum-postgres-template>

Why it matters:

- Smaller template inspired by Zero To Production and RealWorld Axum/SQLx.
- Has a clean `Db` wrapper around `PgPool`.
- Centralizes router construction, app state, middleware, and migration execution.
- Documents SQLx offline mode via `cargo sqlx prepare`.

Borrow:

- A small `Db` wrapper is useful if we want to hang `migrate()` or health checks off it.
- Add `tracing`, request IDs, CORS, timeout, and path normalization deliberately.
- Consider SQLx offline mode once queries are stable enough for CI.

Do not blindly copy:

- Keep middleware minimal at first; add only what v1 needs.
- Avoid making the template's generic API error model too broad before we know the app's response shape.

### 4. Shuttle Axum/Postgres Example

Source: <https://docs.shuttle.dev/examples/axum-postgres>

Why it matters:

- Very small, official-style example of Axum `State`, SQLx `PgPool`, and `sqlx::migrate!()`.
- Good baseline for the minimum viable pattern: state + routes + SQL + migrations.

Borrow:

- The simplicity. A tiny first vertical slice is better than scaffolding every layer upfront.

Do not blindly copy:

- It is Shuttle-specific and not structured as a normal self-hosted binary.
- It maps DB errors directly into response strings; v1 should have a typed HTTP error.

### 5. `b-palaniappan/axum-sqlx`

Source: <https://github.com/b-palaniappan/axum-sqlx>

Why it matters:

- Shows a broader Axum/SQLx app with CI services for Postgres and Redis, migrations, tracing, OpenAPI, auth, caching, and integration tests.
- Useful for CI and test wiring ideas.

Borrow:

- CI can start a Postgres service, set `DATABASE_URL`, run `sqlx migrate run`, build, and test.
- Integration tests should exercise the actual Axum router against a real migrated database.

Do not blindly copy:

- Too much for TJ Shop v1: Redis, MFA, gRPC, OpenTelemetry, OpenAPI, and custom auth should wait until there is a real requirement.
- Avoid stuffing all bootstrap, OpenAPI, static serving, and route wiring into one huge `main.rs`.
- Do not copy its large single-file shape. Static asset serving may matter on the VPS, but it should be isolated from API modules.

## Recommended v1 Shape

Use two top-level application directories: `backend/` and `frontend/`. The Rust backend owns API code, SQLx migrations, and backend tests. The React frontend owns TypeScript, bundling, routes, and UI code.

```text
backend/
  Cargo.toml
  Cargo.lock
  src/
    main.rs
    lib.rs
    config.rs
    state.rs
    db.rs
    error.rs
    http/
      mod.rs
      health.rs
      products.rs
      lists.rs
    domain/
      mod.rs
      product.rs
      shopping_list.rs
    repos/
      mod.rs
      products.rs
      lists.rs
  migrations/
    20260515000000_init.sql
  tests/
    api/
      health.rs
      products.rs
      lists.rs
frontend/
  package.json
  src/
  public/
docs/
  codex/
deploy/
  Caddyfile
  tj-shop.service
docker-compose.yml
README.md
```

This is intentionally conservative. It gives us enough separation for HTTP, domain types, and database access without introducing a formal clean-architecture/repository abstraction for every operation.

The repo can still contain the React app. Keep the Rust backend boundaries clean by treating the built frontend as a deployment artifact, not as part of the API architecture.

Keep the root for orchestration, docs, local services, and VPS deployment files. Keep Rust and frontend toolchains inside their own directories.

## Bootstrap Pattern

`main.rs`:

- Load `.env` with `dotenvy::dotenv().ok()`.
- Initialize `tracing_subscriber`.
- Parse config from env.
- Build one `PgPool`.
- Run `sqlx::migrate!("./migrations").run(&pool)`.
- Build the router from library code.
- Bind a `TcpListener`.
- Serve with graceful shutdown.

`lib.rs`:

- Expose `config`, `db`, `error`, `http`, and app state.
- Provide `build_router(state: AppState) -> Router`.
- Keep server runtime concerns in `main.rs`, router composition in the library.

## App State

Use one cloneable app state:

```rust
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
}
```

Use `State<AppState>` in handlers. Do not create a new database connection per request.

If the app grows, convert `db: PgPool` to a lightweight `Db` wrapper:

```rust
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}
```

Keep the inner pool accessible to repo functions without hiding SQLx behind a large trait hierarchy.

## Database/Migrations

Use SQLx migrations as the source of truth:

- Keep all schema in `migrations/`.
- Use forward-only migrations unless there is a clear need for reversible migrations.
- Use `sqlx migrate add <name>` for new migration files.
- Run `sqlx migrate run` locally and in CI.
- Run embedded migrations on app startup for small self-managed deployments.

Initial tables likely needed:

- `products`: TJ product catalog cache plus manual product rows.
- `shopping_lists`: list metadata.
- `shopping_list_items`: products, quantities, checked state, ordering/category grouping.
- Optional later: `product_price_snapshots`, `stores`, `users` or `households`.

## SQLx Usage

Prefer direct SQL:

- Use `query!` / `query_as!` for stable queries that should be compile-time checked.
- Use `query_file!` / `query_file_as!` if queries get large.
- Use `FromRow` with `query_as` when the query is dynamic or compile-time checking is not worth the setup yet.
- Consider checked-in `.sqlx/` offline metadata once CI is in place and the query surface stabilizes.

Avoid early ORM-style abstraction. TJ Shop's data model is relational and small enough that readable SQL will be clearer.

## HTTP Surface

The backend API should serve pure JSON. Use `Json<T>` responses, `Json<T>` request bodies, and explicit API DTOs at the HTTP boundary. Do not design HTML templates, server-rendered pages, or HTMX fragments in this backend pass.

Start with a narrow JSON API:

```text
GET  /healthz
GET  /api/products
POST /api/products/refresh
GET  /api/lists
POST /api/lists
GET  /api/lists/:id
PUT  /api/lists/:id/items
PATCH /api/lists/:id/items/:item_id
```

Keep handler modules aligned with resources:

- `http/products.rs`
- `http/lists.rs`
- `http/health.rs`

Handlers should parse/validate HTTP input, call repo/domain functions, and map results into response DTOs. They should not contain large SQL strings inline once the query becomes meaningful.

Use `/api/...` for application endpoints and reserve non-API routes for operational endpoints such as `/healthz`. The React app can live in the same repo and deployment, but it should still communicate with the backend through the JSON contract.

## Same-Repo VPS Deployment

Serving frontend and backend from the same repo/VPS changes deployment, not the core API design.

Preferred production shape:

```text
Browser
  -> https://tj-shop.example.com/
       reverse proxy or Axum static service -> built React assets
  -> https://tj-shop.example.com/api/...
       reverse proxy or Axum router -> JSON API
```

Reasonable options:

- Put Caddy or Nginx in front. Let it serve `frontend/dist` directly and reverse-proxy `/api/*` plus `/healthz` to the Axum process on localhost.
- Or let Axum serve the built `frontend/dist` with `tower_http::services::ServeDir` and fallback to `index.html` for React routes, while `/api/*` remains the JSON API.

The reverse-proxy option is the boring default for a small VPS because TLS, compression, cache headers, access logs, and static file serving are simpler outside the Rust app. Axum static serving is still acceptable if a single process is more valuable than that separation.

Either way:

- Keep API routes under `/api`.
- Keep React routes outside `/api`.
- Add an SPA fallback only for frontend routes, never for `/api` misses.
- In production, same-origin requests mean CORS can usually be disabled.
- In local development, enable CORS only for the Vite/dev-server origin, or use the frontend dev server's proxy to avoid CORS.
- Build and deploy the frontend as static files; do not make Rust responsible for TypeScript compilation.

## Local Development

Optimize for quick local iteration:

```text
Postgres
  -> local Docker/Podman Compose service
Backend
  -> cd backend && cargo run
Frontend
  -> cd frontend && npm run dev
```

Local defaults:

- Run Postgres with a root `docker-compose.yml`.
- Put backend config in `backend/.env` or pass env vars from the shell.
- Use `DATABASE_URL` for SQLx and app startup.
- Let the frontend dev server proxy `/api` to the Axum port, or enable narrow CORS for the dev origin.
- Keep test setup close to production by running migrations against a real Postgres database.

Root-level helper commands should make the agent workflow predictable. A `justfile` or `Makefile` can wrap:

```text
backend-test
backend-check
frontend-test
dev-db-up
dev-db-reset
```

Do not require the local workflow to mimic the VPS exactly. The important match is the contract: same migrations, same env-driven config, same JSON API, same built frontend artifact.

Environment split:

- Local development uses containerized Postgres for quick setup, teardown, reset, and test database workflows.
- The minimal VPS uses native services: system Postgres, an Axum binary managed by `systemd`, and Caddy/Nginx as the reverse proxy/static file server.
- Do not plan on Docker being required in production unless deployment constraints change.

Shared VPS Postgres:

- It is fine for multiple small apps to share one native Postgres server on the VPS.
- Give each app its own database and Postgres role.
- Do not connect app code as the `postgres` superuser.
- Keep each app's migrations scoped to that app's database.
- Use conservative SQLx pool sizes per app so the combined connection count stays comfortably below the Postgres limit.

Example:

```text
postgres server
  database: tj_shop_prod
    role: tj_shop_app
  database: another_app_prod
    role: another_app
```

## Error Handling

Create one API error enum:

```rust
pub enum ApiError {
    BadRequest(String),
    NotFound,
    Conflict(String),
    Internal(anyhow::Error),
}
```

Implement `IntoResponse` once. Use `thiserror` for typed expected errors and `anyhow` for internal bootstrap/application errors.

Do not expose raw SQLx error strings in HTTP responses.

## Middleware

Start with:

- `TraceLayer::new_for_http()`
- request timeout
- sensitive header redaction if auth or tokens are introduced
- CORS for local development only if the React dev server calls Axum from a different origin

Add request IDs after the first API pass if logs become hard to follow.

## Testing

Development should proceed in an agent-driven TDD style. For backend feature work, start with an API or repository test unless the change is purely mechanical.

Agent workflow:

```text
1. Write or update a failing test.
2. Run the narrow test and confirm the failure is expected.
3. Implement the smallest backend change.
4. Run the narrow test until green.
5. Run broader checks: fmt, clippy, and tests.
6. Update docs if behavior or architecture changed.
```

Use three layers:

- Unit tests for pure domain functions.
- Repository tests against a migrated Postgres test database.
- API tests that call the Axum router directly with `tower::ServiceExt`.

CI should:

- Start Postgres.
- Set `DATABASE_URL`.
- Run migrations.
- Run `cargo fmt --check`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo test`.

First vertical slice:

- Scaffold `backend/` as the Rust crate.
- Add `/healthz` or `/health` as a JSON endpoint.
- Add config and router construction separate from `main.rs`.
- Add Postgres connection config.
- Add the first empty migration.
- Add API integration test support.
- Add root local Postgres Compose config.
- Add root helper commands for backend checks.

## Dependencies To Start

```toml
[dependencies]
anyhow = "1"
axum = "0.8"
dotenvy = "0.15"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls-ring-native-roots", "postgres", "uuid", "time", "json", "macros", "migrate"] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower-http = { version = "0.6", features = ["trace", "timeout", "cors", "sensitive-headers"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1", features = ["serde", "v7"] }
time = { version = "0.3", features = ["serde"] }
```

Check versions at scaffold time, but this is the boring dependency set.

## Decisions For TJ Shop v1

- Keep v1 as one crate until there is a concrete reason for a workspace.
- Use `src/main.rs` as thin process bootstrap and `src/lib.rs` for router construction.
- Use one `AppState`.
- Use SQLx migrations in `migrations/` and embedded startup migrations.
- Prefer direct SQL over ORM/repository ceremony.
- Keep the Rust API pure JSON even though the React app ships from the same repo/VPS.
- Optimize for local iteration first: Docker/Podman Postgres, direct `cargo run`, and frontend dev server proxying `/api`.
- Prefer Caddy/Nginx serving built React assets and proxying `/api` to Axum for production.
- Allow Axum static serving only as a small deployment convenience, isolated from API modules.
- Keep VPS deployment minimal and native: one system Postgres instance, one Axum `systemd` service, one reverse proxy, and built frontend static files.
- On the VPS, share the native Postgres server across small apps by using one database and one role per app.
- Do not require Docker on the VPS for v1.
- Develop backend features test-first, driven by API/repository tests.
- Put server-rendered HTML, auth, Redis, OpenAPI, background jobs, and distributed tracing out of scope for initial v1.
- Build one vertical slice first: health check, one product table, one list table, and one list-items route.

## Sources

- `davidpdrsn/realworld-axum-sqlx`: <https://github.com/davidpdrsn/realworld-axum-sqlx>
- `launchbadge/realworld-axum-sqlx`: <https://github.com/launchbadge/realworld-axum-sqlx>
- `koskeller/axum-postgres-template`: <https://github.com/koskeller/axum-postgres-template>
- Shuttle Axum/Postgres example: <https://docs.shuttle.dev/examples/axum-postgres>
- SQLx README/docs: <https://github.com/launchbadge/sqlx>
- `b-palaniappan/axum-sqlx`: <https://github.com/b-palaniappan/axum-sqlx>
