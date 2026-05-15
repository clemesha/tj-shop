# Reference repos — Axum + SQLx + Postgres survey

Six repos surveyed. Each verified against the target stack by reading
`Cargo.toml`. Stars and last-push are May 2026 snapshots.

Ranked by relevance to "small JSON API, boring standard". The first two are
what you should crib from directly.

---

## 1. koskeller/axum-postgres-template — **minimum-viable, primary template**

- URL: https://github.com/koskeller/axum-postgres-template
- Last push: 2024-11-16. Stars: ~57.
- Stack: Axum 0.7.6, SQLx 0.8.0, tokio 1.x, tower-http 0.6, tracing, thiserror,
  anyhow, dotenvy, uuid v7. **Verified match.**

### What it is

A small, opinionated single-crate template explicitly inspired by Zero To
Production In Rust and launchbadge/realworld. No auth, no business logic — just
the bones: config, db pool, migrations-on-startup, structured logging,
request-id middleware, graceful CORS/timeout/path-normalize, a `/health_check`
route, and a working integration test against a real per-test database.

### Why study it

This is the closest match to what tj-shop v1 should look like on day one. It
gives you the floor of "production-shaped" without any feature you haven't
asked for. The integration test setup (random per-test DB, migration on
startup, full router exercised via `tower::ServiceExt::oneshot`) is exactly
the pattern to copy.

### Layout

```
src/
├── api_error.rs        # thiserror enum, IntoResponse impl
├── cfg.rs              # env -> Configuration struct, Arc-wrapped
├── db.rs               # Db { pool: PgPool }, .new(), .migrate()
├── lib.rs              # AppState, fn router(cfg, db) -> Router
├── main.rs             # bootstrap: load env, tracing, db, migrate, serve
├── middleware.rs       # request_id, cors, timeout, normalize_path layers
├── routes/
│   ├── mod.rs          # Router<AppState> aggregator
│   └── health_check.rs # single GET /health_check handler
└── telemetry.rs        # tracing_subscriber JSON layer + TraceLayer factory
migrations/
└── 20240126114845_example.sql
tests/api/
├── main.rs
├── helpers.rs          # TestApp { router, db }; create_test_db()
└── health_check.rs     # tokio::test using TestApp::new().await
```

### Patterns worth copying

- **AppState shape (`src/lib.rs`):** `#[derive(Clone)] struct AppState { db: Db,
  cfg: Config }` where `cfg: Arc<Configuration>` and `db: Db` wraps a `PgPool`
  (PgPool is already cheaply cloneable; no extra Arc needed). Pass via
  `.with_state(app_state)`.
- **Router-as-function (`src/lib.rs`):** Exporting `pub fn router(cfg, db) ->
  Router` from the lib crate is what makes integration tests trivial — both
  `main.rs` and the test harness call the same function. Do this.
- **Error type (`src/api_error.rs`):** `thiserror::Error` enum `ApiError` with
  `#[from] sqlx::Error`, `#[from] anyhow::Error`, plus
  `#[from] axum::extract::rejection::JsonRejection`. `IntoResponse` impl logs
  the *real* error via `tracing::error!`, then returns a sanitized
  `ApiErrorResp { message }` JSON body. The two-step "log internal, return
  generic" is the right move for a public API.
- **Config (`src/cfg.rs`):** Hand-rolled env reader, panics with explicit
  messages on missing/unparseable vars. Returns `Arc<Configuration>`. Boring,
  zero deps beyond `serde`/`std`. Good baseline. Skip the `config` crate.
- **Migrations on startup (`src/db.rs`):** `sqlx::migrate!("./migrations")
  .run(&self.pool).await?` called from `main.rs` before `axum::serve`. The
  comment "This integrates database migrations into the application binary to
  ensure the database is properly migrated during startup" is the right
  default for a small app.
- **Telemetry (`src/telemetry.rs`):** `tracing_subscriber::registry()` +
  `EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into())` +
  `fmt::layer().json()`. Pairs with a separate `trace_layer()` factory that
  returns a `tower_http::trace::TraceLayer` for HTTP-level spans. Two layers,
  one for stdout, one for the request span — clean separation.
- **Request IDs (`src/middleware.rs`):** `MakeRequestId` impl that emits
  `uuid::Uuid::now_v7()` for the `x-request-id` header. v7 UUIDs sort by time,
  which helps log correlation. Worth copying verbatim.
- **Test harness (`tests/api/helpers.rs`):** `TestApp::new()` creates a fresh
  random-named database per test, runs migrations, builds the *same*
  `router()` the app uses, and exposes `.request(req)` via
  `tower::ServiceExt::oneshot`. No real TCP socket. Fast.

### Things to skip / disagree with

- **The `JsonRejection` matching arms are verbose** — the `_ => "Unknown
  error"` is fine for v1; tighten later if you actually need per-variant
  messages.
- **`Config` returns `Arc<Configuration>` directly from `new()`** — fine, but
  it forces `Arc` on every test. Personal taste; not a bug.
- **The `set_dsn` method on `Config`** is a code smell (uses
  `Arc::get_mut`-style mutation on the config). Don't copy it; instead, let
  the test harness build a fresh `Configuration` per test.
- **Migration filename is a 14-digit timestamp** — that's what `sqlx migrate
  add` does by default, fine to keep.

**Label:** minimum-viable production template. **Use this as the starting
skeleton.**

---

## 2. launchbadge/sqlx — `examples/postgres/axum-social-with-tests` — **opinionated reference, primary for tests**

- URL: https://github.com/launchbadge/sqlx/tree/main/examples/postgres/axum-social-with-tests
- Lives inside the sqlx repo itself (~14k stars on parent; actively
  maintained, last activity within days).
- Stack: Axum 0.8, SQLx 0.8 (path dep on parent), thiserror 2.x, anyhow,
  validator, argon2. **Verified match — uses Axum 0.8.**

### What it is

The sqlx project's own example of how to build an Axum app with sqlx, and
critically, how to write integration tests with `#[sqlx::test]`. Small
("social network" toy: users, posts, comments) but every line is intentional.

### Why study it

This is the **authoritative source** for the `#[sqlx::test]` pattern. Every
test gets a fresh empty database (or one seeded from a fixture SQL file) via a
proc macro that injects a `PgPool` parameter into your async test fn. No
manual `create_test_db` like koskeller does — `#[sqlx::test]` does it for
you, including running your migrations.

Also: error type and router setup are clean and minimal in a way that the
launchbadge/realworld repo is not.

### Layout

```
src/
├── main.rs             # connect pool, run migrations, serve
├── lib.rs              # pub mod http; pub mod password;
├── password.rs
└── http/
    ├── mod.rs          # fn app(db: PgPool) -> Router; pub async fn serve
    ├── error.rs        # Error enum + IntoResponse
    ├── user.rs
    └── post/
        ├── mod.rs
        └── comment.rs
migrations/
├── 1_user.sql
├── 2_post.sql
└── 3_comment.sql
tests/
├── common.rs           # RequestBuilderExt::json, response_json helpers
├── user.rs             # #[sqlx::test] async fn test_create_user(db: PgPool)
├── post.rs
├── comment.rs
└── fixtures/           # *.sql files referenced via #[sqlx::test(fixtures(...))]
    ├── users.sql
    ├── posts.sql
    └── comments.sql
```

### Patterns worth copying

- **`fn app(db: PgPool) -> Router` in `src/http/mod.rs`:** State is just the
  `PgPool` directly via `.with_state(db)`, because there's no config to share.
  For tj-shop v1 you'll want `AppState` like koskeller, but the principle —
  *the entrypoint factory takes its dependencies as args and returns a
  Router* — is the same.
- **Error enum (`src/http/error.rs`):** `thiserror::Error` enum with five
  variants: `Sqlx(#[from] sqlx::Error)`, `Anyhow(#[from] anyhow::Error)`,
  `InvalidEntity(#[from] ValidationErrors)`, `UnprocessableEntity(String)`,
  `Conflict(String)`. `IntoResponse` uses `serde_with` to serialize the
  `Display` output. **This is the canonical minimal error type.** Copy the
  shape; you don't need `Conflict`/`UnprocessableEntity` on day one but they
  cost nothing.
- **`pub type Result<T, E = Error> = std::result::Result<T, E>;`** — the
  type alias trick (in `src/http/mod.rs`) makes handler signatures clean:
  `async fn create_user(...) -> Result<...>`. Copy this.
- **`#[sqlx::test]` (`tests/user.rs`):** Just `#[sqlx::test] async fn
  test_create_user(db: PgPool) { let mut app = http::app(db); ... }`. The
  macro provisions a fresh database, runs migrations from `./migrations/`,
  injects the `PgPool`, and tears the database down on success. This replaces
  the entire koskeller `helpers.rs` random-db dance.
- **Fixtures via `#[sqlx::test(fixtures("users", "posts"))]`** — seed SQL
  files referenced by name from `tests/fixtures/`. Use this for any test that
  needs pre-existing data.
- **`RequestBuilderExt::json` (`tests/common.rs`):** A tiny trait extension on
  `request::Builder` so tests read `Request::post("/v1/user").json(json!{{...}})`.
  Copy this verbatim.

### Things to skip / disagree with

- **`println!("API error: {self:?}")` in `error.rs`** — explicitly flagged
  in their own comment as "you wouldn't just print this." Use
  `tracing::error!` instead, as koskeller does.
- **`serde_with` + `DisplayFromStr`** for the error message — works, but if
  you find it weird, just inline `message: self.to_string()` like koskeller.

**Label:** opinionated reference, especially for tests. **Use this as the
authority on test setup.**

---

## 3. launchbadge/realworld-axum-sqlx — **opinionated reference, primary for design commentary**

- URL: https://github.com/launchbadge/realworld-axum-sqlx
- Last push: **2023-12-30. Stars: ~1.1k.** Not abandoned but not on the
  current Axum version either.
- Stack: **Axum 0.3** (very stale), SQLx 0.5, tokio 1, thiserror, anyhow.

### What it is

The Realworld spec implemented by a senior dev at Launchbadge (Austin
Bonander, also a sqlx maintainer). The point of the repo is not the code,
it's the **comments**: long inline explanations of why each pattern was
chosen. Treat it as a design doc with a working app stapled on.

### Why study it

The `src/http/mod.rs` doc-comments on `ApiContext` (their AppState
equivalent), the `src/http/error.rs` doc-comments on the `Error` enum, and
the `migrations/1_setup.sql` comments on schema conventions are still the
clearest written justifications for these patterns anywhere. **Read the
comments; don't copy the code.**

### Layout

```
src/
├── config.rs
├── main.rs
├── lib.rs
└── http/
    ├── mod.rs          # ApiContext, serve(), api_router()
    ├── error.rs        # Error enum + ResultExt
    ├── extractor.rs    # AuthUser, MaybeAuthUser
    ├── types.rs
    ├── users.rs
    ├── profiles.rs
    └── articles/
        ├── mod.rs
        ├── listing.rs
        └── comments.rs
migrations/
├── 1_setup.sql         # heavily commented; uuid-ossp, updated_at trigger
├── 2_user.sql
├── 3_follow.sql
└── 4_article.sql
```

### Patterns worth copying

- **The `Error` enum doc-comments (`src/http/error.rs`):** explains *why*
  `#[from] sqlx::Error` and `#[from] anyhow::Error` are both present, why the
  message is generic to the client, and what `ResultExt` is for (mapping
  Postgres constraint errors to 409/422). The pattern itself is identical to
  koskeller's; the comments are gold.
- **Schema conventions (`migrations/1_setup.sql`):** `create extension
  "uuid-ossp"` for `uuid_generate_v1mc()`; every table gets `created_at` and
  `updated_at`; a `trigger_updated_at(tablename regclass)` helper to avoid
  retyping the trigger. **Adopt these as table conventions for tj-shop.**
- **The migration README** (`migrations/README.md`) covers when to
  squash/never-edit, which is the standard advice.

### Things to skip / disagree with

- **Axum 0.3 APIs:** `AddExtensionLayer`, `axum::Server::bind`,
  `Router::nest_service` shape — all changed. Do not copy router/server
  bootstrap.
- **Storing config in `Extension`** instead of `State`. Axum 0.6+ added
  typed `State`; use that.
- **`ApiContext` vs `AppState` naming** — pick one (`AppState` is more
  common in 2025/26 Axum tutorials). The naming doesn't matter, the shape
  does.
- **Auth scaffolding (JWT, argon2, etc.)** — out of scope for v1; ignore.

**Label:** opinionated reference, design commentary only. **Read the
comments, then close the tab.**

---

## 4. sukjaelee/clean_axum_demo — **medium-real, cautionary**

- URL: https://github.com/sukjaelee/clean_axum_demo
- Last push: 2025-08-09. Stars: ~189.
- Stack: Axum 0.8.3, SQLx 0.8.3, thiserror, utoipa (Swagger), JWT (jsonwebtoken
  9), argon2, opentelemetry (optional feature). **Verified match.**

### What it is

A multi-domain app (auth, user, device, file) wired up using Clean
Architecture / DDD layering: every domain has `api/`, `domain/`, `dto/`,
`infra/` subdirectories with separate `handlers.rs`, `routes.rs`, `model.rs`,
`repository.rs` (trait), `service.rs` (trait), `impl_repository.rs`,
`impl_service.rs`. Authoritative for "what does this stack look like when
you over-engineer a medium app."

### Why study it

Two reasons. First, it's on **Axum 0.8 and SQLx 0.8**, so the dep versions
and APIs are current. Second, it's a useful counter-example: this is what
happens when you take the layering too far for a small app. Look at the file
count for `src/domains/user/` — six files for one domain. Do not do this for
tj-shop v1.

### Layout

```
src/
├── main.rs
├── lib.rs
├── app.rs              # create_router(state) -> Router
├── common/
│   ├── app_state.rs    # AppState holds Arc<dyn ...Trait> per service
│   ├── bootstrap.rs    # build_app_state, setup_tracing, shutdown_signal
│   ├── config.rs       # env -> Config struct, setup_database()
│   ├── error.rs        # AppError enum + IntoResponse
│   ├── jwt.rs
│   ├── dto.rs
│   ├── hash_util.rs
│   ├── multipart_helper.rs
│   ├── opentelemetry.rs
│   └── ts_format.rs
└── domains/
    ├── auth/{api,domain,dto,infra}/...
    ├── user/{api,domain,dto,infra}/...
    ├── device/{api,domain,dto,infra}/...
    └── file/{api,domain,dto,infra}/...
migrations/
└── (CHAR(36) UUID tables)
```

### Patterns worth copying

- **Graceful shutdown (`src/main.rs` + `src/common/bootstrap.rs`):**
  `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?`
  where `shutdown_signal` awaits `tokio::signal::ctrl_c()`. Standard, copy
  verbatim.
- **Pool retry on startup (`src/common/config.rs`, `setup_database`):**
  Retries the initial `PgPoolOptions::connect` 3× with a 1s delay before
  giving up. Useful when Docker Compose starts Postgres and the API in
  parallel.
- **Verbose tracing-subscriber config (`bootstrap.rs::setup_tracing`):** Adds
  `.with_file(true).with_line_number(true).with_target(true)` and
  `FmtSpan::CLOSE` for span-close events. Worth borrowing in dev; switch to
  JSON in prod.

### Things to skip / disagree with

- **`Arc<dyn ServiceTrait>` per domain in `AppState`** (`src/common/app_state.rs`).
  This is Java/Spring-style dependency injection, and it doesn't carry its
  weight here. The traits exist solely so you can hand-mock them in tests —
  but `#[sqlx::test]` against a real database is faster and catches more
  bugs. **Do not adopt.** For tj-shop, put the `PgPool` directly on
  `AppState` and call `sqlx::query!` from handlers (or from free functions
  in a `db/` module if you want a thin abstraction).
- **`{api,domain,dto,infra}` quad-tree per domain.** Massive overkill for a
  shopping-list app. Flatten to one `module_name.rs` per domain, or at most
  a `{handlers.rs, db.rs}` split.
- **Custom request/response body inspector middleware (`src/app.rs:make_request_response_inspecter`)**
  buffers every request and response body and regex-greps for `<script>`
  tags. This is a bad XSS-prevention pattern (do output escaping at the
  client/template layer instead) and it costs you streaming. **Skip.**
- **`CHAR(36)` UUIDs in Postgres.** Postgres has a native `uuid` type. Use
  that.
- **utoipa/Swagger.** Defer until you actually need it (see open-questions).

**Label:** medium-real, cautionary. **Cherry-pick graceful-shutdown +
tracing setup; ignore the architecture.**

---

## 5. sheroz/axum-rest-api-sample — **medium-real, alternate reference**

- URL: https://github.com/sheroz/axum-rest-api-sample
- Last push: 2026-03-23. Stars: ~128. **Most-recently-maintained on the
  list.**
- Stack: Axum 0.8, SQLx 0.8, thiserror 2, jsonwebtoken 10, redis 1.1, edition
  2024, rust-version 1.94. **Verified match.**

### What it is

A REST API skeleton with JWT auth, RBAC, accounts/users/transactions
domain, Redis-backed token revocation, integration test suite,
docker-compose, GitHub Actions CI, `clippy.toml`, `deny.toml`. Closest
thing to "what a junior senior would build in 2026 if asked for a
production-shaped Axum API."

### Layout

```
src/
├── main.rs
├── lib.rs
├── api/
│   ├── server.rs
│   ├── version.rs
│   ├── error.rs        # APIError, APIErrorEntry, APIErrorCode/Kind enums
│   ├── extractors.rs
│   ├── handlers/{account,auth,transaction,user}_handlers.rs
│   └── routes/{account,auth,transaction,user}_routes.rs
├── application/
│   ├── app.rs          # boot
│   ├── config.rs
│   ├── state.rs        # SharedState = Arc<AppState>
│   ├── constants.rs
│   ├── repository/{account,transaction,user}_repo.rs
│   ├── security/{auth,jwt,roles}.rs
│   └── service/{token,transaction}_service.rs
├── domain/models/{account,transaction,user}.rs
└── infrastructure/
    ├── database/{database.rs, postgres/{postgres.rs, options.rs, migrations/, queries/}}
    └── redis/connection.rs
tests/  (split by feature: auth_login_test.rs, auth_refresh_tests.rs, ...)
```

### Patterns worth copying

- **`SharedState = Arc<AppState>` (`src/application/state.rs`):** Single Arc
  wrap of the whole AppState, instead of relying on PgPool's internal Arc.
  Fine pattern, slightly safer if you later put a non-Clone field in
  AppState. **Decide later** — for v1, follow koskeller and put `PgPool` and
  `Config` as Clone fields directly on `AppState`; only Arc-wrap when you
  add something that demands it.
- **`From<sqlx::Error> for APIErrorEntry` (`src/api/error.rs`)** with
  `cfg!(debug_assertions)` branching: in debug builds, expose the real sqlx
  error; in release, log it with a trace_id and return a generic 500. Good
  idea for a public API; copy if you want.
- **Migrations are stored under `src/infrastructure/database/postgres/migrations/`**
  and loaded via `sqlx::migrate!("./src/infrastructure/.../migrations")`.
  **Do not** copy this; default `./migrations` is simpler and is what `sqlx
  migrate add` writes to.
- **Edition 2024, rust-version 1.94** pinned in `Cargo.toml` — adopt
  edition 2024 for v1.
- **Tests are split per feature, not per module** (`auth_login_test.rs`,
  `auth_refresh_tests.rs`, …) — clearer than one giant tests file.

### Things to skip / disagree with

- **The `APIError`/`APIErrorEntry`/`APIErrorCode`/`APIErrorKind` envelope**
  (`src/api/error.rs`) is an RFC 7807-ish nested structure with 12 optional
  fields per error entry. Way too much for v1. **Use koskeller's flat
  `{message}` shape instead.** Add structured error codes later if a
  frontend actually consumes them.
- **Six-layer `api/application/domain/infrastructure/security/...`
  layout.** For tj-shop's scale, keep it to ~3 top-level directories.
- **Redis dependency** — no use for tj-shop v1.

**Label:** medium-real, alternate reference. **Read the error type
discussion; mostly ignore the structure.**

---

## 6. tokio-rs/axum — `examples/sqlx-postgres/src/main.rs` — **minimum-viable, smoke-test**

- URL: https://github.com/tokio-rs/axum/blob/main/examples/sqlx-postgres/src/main.rs
- Maintained as part of the Axum repo (~20k stars on parent), updated
  whenever Axum is.
- Stack: Axum (current main), SQLx with `runtime-tokio-rustls`,`any`,
  `postgres`. **Verified match.**

### What it is

The official `cargo run -p example-sqlx-postgres` example. ~110 lines total.
Demonstrates two things: (a) `.with_state(pool)` extracting `State<PgPool>`
in a handler, and (b) writing a custom `FromRequestParts` extractor that
acquires a connection from the pool.

### Why study it

The reference for "what does sqlx-in-axum look like with no
opinions whatsoever." Whenever the bigger templates do something weird,
check this file to see what the un-opinionated baseline is.

### Layout

```
src/
└── main.rs       # ~110 lines, single file
Cargo.toml
```

### Patterns worth copying

- `PgPoolOptions::new().max_connections(5).acquire_timeout(Duration::from_secs(3))
  .connect(&dsn).await` — the `acquire_timeout` is worth copying; koskeller
  omits it.
- `State<PgPool>` extractor in the handler signature; no wrapper struct
  needed when the pool *is* the entire state.
- The `internal_error` helper that maps any `Error` to `(StatusCode, String)`
  — useful for "I haven't built my error type yet" prototyping; replace
  with `ApiError` once you have one.

### Things to skip / disagree with

- No config, no migrations, no logging, no `AppState`. It's a 110-line
  example — by design. Don't ship it.

**Label:** minimum-viable, smoke-test reference.

---

## Summary table

| Repo | Axum | SQLx | Updated | Use for |
| --- | --- | --- | --- | --- |
| koskeller/axum-postgres-template | 0.7 | 0.8 | 2024-11 | **Skeleton + middleware + test harness** |
| launchbadge/sqlx examples/.../axum-social-with-tests | 0.8 | 0.8 | active | **Error type + `#[sqlx::test]` pattern** |
| launchbadge/realworld-axum-sqlx | 0.3 | 0.5 | 2023-12 | Design commentary in code comments only |
| sukjaelee/clean_axum_demo | 0.8 | 0.8 | 2025-08 | Graceful shutdown; cautionary on layering |
| sheroz/axum-rest-api-sample | 0.8 | 0.8 | 2026-03 | Sanity check on current dep versions |
| tokio-rs/axum examples/sqlx-postgres | latest | latest | active | Smoke-test baseline |

---

*Last updated: 2026-05-15*
