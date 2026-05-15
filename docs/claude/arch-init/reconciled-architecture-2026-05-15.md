# Reconciled architecture — tj-shop v1 backend

Two independent agents seeded this architecture:
- `docs/claude/recommended-layout-2026-05-15.md` (this agent)
- `docs/codex/arch-init/architecture-seed-axum-sqlx-postgres-2026-05-15.md` (codex)

This doc captures (a) where both converged with high confidence,
(b) where they disagreed and the deliberate resolution, and (c) the
elements from each side worth absorbing into the final plan.

**When this doc and the two originals disagree, this doc wins.**

## Strong convergence (high confidence — both agents picked these independently)

- Stack: **Axum 0.8 + SQLx 0.8 + Postgres 16 + sqlx migrations + Tokio**.
- **Single crate**, no workspace.
- `main.rs` is thin bootstrap; `lib.rs` exposes router construction.
- One **`AppState`** struct, `Clone`, holds `PgPool` + `Arc<Config>`.
- **`sqlx::migrate!()` at startup**, idempotent, embedded.
- **`thiserror` enum + `anyhow`** for fall-through internal errors; one
  `IntoResponse` impl; flat `{"error": "..."}` JSON body.
- **Hand-rolled config** from env via `dotenvy`. No `config`/`figment`.
- **`tracing` + `tracing-subscriber` + `tower-http::TraceLayer`.**
- **Graceful shutdown** via `with_graceful_shutdown` from day one.
- **Test strategy: real Postgres via `#[sqlx::test]`**; router exercised
  via `tower::ServiceExt::oneshot`. No DB mocking.
- **TDD as the development cadence**, agent-driven.
- **Monorepo**: `backend/` + `frontend/` siblings.
- **Same-VPS deploy**: Caddy fronts, reverse-proxies `/api/*` to Rust on
  loopback. Pure JSON backend; Caddy serves the React build.
- **Containerized Postgres locally, native Postgres on VPS.**
- **Multi-app shared Postgres on the VPS**: one database + one role per
  app; app roles never connect as superuser.
- **Deferred from v1**: auth, OpenAPI, Redis, server-rendered HTML,
  background jobs, distributed tracing, custom DI / clean-architecture
  abstractions.

## Material differences and resolutions

### 1. URL paths: `/api/` prefix → codex wins

Codex prefixed app routes with `/api/`; I did not.

**Resolution: use `/api/` prefix.** It's load-bearing for the Caddy split
(`handle /api/* { reverse_proxy ... }` vs the SPA fallback). Without it,
`try_files {path} /index.html` would swallow API misses. Operational
endpoints (`/healthz`) stay **outside** `/api/`.

Final endpoint shape (provisional, domain-dependent):
```
GET   /healthz                            # operational, outside /api
GET   /api/products
POST  /api/products/refresh               # if syncing from TJ GraphQL (see Q5)
GET   /api/lists
POST  /api/lists
GET   /api/lists/:id
PUT   /api/lists/:id/items
PATCH /api/lists/:id/items/:item_id
```

### 2. Time crate: `time` → codex wins

I picked `chrono`; codex picked `time`.

**Resolution: `time`.** Cleaner API, fewer historical CVEs, matches
launchbadge/sqlx example code. The "wider chrono ecosystem" argument
doesn't pay off for a small backend; sqlx type mapping is identical.

### 3. SQLx TLS feature → codex wins

I had the legacy `runtime-tokio-rustls`; codex had the current
`tls-rustls-ring-native-roots` + separate `runtime-tokio`.

**Resolution: codex's feature names.** More current sqlx 0.8 idiom.

### 4. Backend module layout: mostly mine, with one codex tweak

Codex split `src/` into `http/`, `domain/`, `repos/`. I kept it flat
with `routes/` + inline SQL in handlers.

**Resolution: stay flat for v1.** Three dirs for ~8 endpoints is
premature. Inline `sqlx::query!` in route files; extract a `repos/`
module only when a query is shared by two handlers. A flat `domain.rs`
file (single file, not a directory) appears when DTOs need to be shared
across routes — not before.

**One codex tweak adopted: separate `state.rs`.** Move `AppState` out of
`lib.rs` into `src/state.rs`. `lib.rs` becomes a pure aggregator.
Trivial; slightly more discoverable.

### 5. Request IDs: day-one → mine wins

I said wire `tower-http`'s `SetRequestIdLayer` +
`PropagateRequestIdLayer` from the start; codex said add when logs get
hard to follow.

**Resolution: day-one.** Two layers, ten lines, immediate value the
first time any concurrent request shows up in logs. Retrofitting later
means revisiting every log statement that wants the correlation ID.

### 6. Infra files location: bare at root → mine wins (user-approved)

Codex puts `Caddyfile` + `tj-shop.service` in `deploy/`. I have them
plus `compose.yaml` as bare files at root.

**Resolution: bare at root.** User explicitly preferred this in earlier
conversation. Three small files don't earn a directory.

### 7. CI sketch: include from day one → codex wins

I deferred CI; codex sketched the pipeline.

**Resolution: include the sketch in the docs even if not wired up.**
Standard pipeline:
1. Spin up Postgres service.
2. Set `DATABASE_URL`.
3. `cargo fmt --check`.
4. `cargo clippy --all-targets -- -D warnings`.
5. `cargo test` (migrations run via `#[sqlx::test]`).

Shape is portable to GitHub Actions, GitLab CI, Forgejo Actions, or any
other runner. Wire it up when there's a CI provider; the shape is settled.

### 8. `justfile` for agent commands → codex wins

Codex mentioned a `justfile` / `Makefile` for repeatable commands; I
didn't. For agent-driven TDD this is high-value — the agent always
knows the canonical command names.

**Resolution: include `justfile` at repo root.** Initial set:

```just
default:
    @just --list

db-up:           docker compose up -d db
db-down:         docker compose down
db-reset:        docker compose down -v && docker compose up -d db

backend-run:     cd backend && cargo run
backend-test:    cd backend && cargo test
backend-check:   cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings

frontend-dev:    cd frontend && npm run dev
frontend-build:  cd frontend && npm run build
```

## Codex contributions to absorb

- **Initial schema sketch** (codex named the likely tables; this is the
  starting point for the first non-`/healthz` migration):
  - `products` — TJ product catalog cache + manual rows
  - `shopping_lists` — list metadata
  - `shopping_list_items` — products in lists, with quantities, checked
    state, ordering/category grouping
  - *Possible later:* `product_price_snapshots`, `stores`, `users` /
    `households`
- **`POST /api/products/refresh`** — codex's nod to one answer for the
  open seed-data question: a refresh endpoint that pulls from TJ
  GraphQL on demand. Not the only option (see Q5 below), but a viable
  shape.
- **`query_file!` / `query_file_as!`** as an escape hatch when inline
  SQL in a handler gets unwieldy.
- **`sensitive-headers`** feature on `tower-http` — added to deps below
  preemptively for when auth eventually shows up.

## Mine that codex didn't capture (preserve)

- **Concrete `IntoResponse` impl** with `Sqlx(#[from] sqlx::Error)` and
  `Anyhow(#[from] anyhow::Error)` variants — codex described in prose,
  the working code is in `docs/claude/recommended-layout-2026-05-15.md` →
  "Error type".
- **Memory-aware deploy mechanic** with explicit RAM cutoffs (build on
  box if VPS ≥ 4 GB; cross-compile + rsync via `cargo-zigbuild` if
  1–2 GB).
- **Connection-budget math** for shared Postgres: `max_connections=10`
  per app, ~10 apps fit under default 100.
- **Local ↔ VPS parity table**: what must match (Postgres major, env-var
  contract, auto-applied migrations) vs what intentionally diverges
  (Caddy/systemd absent locally, Postgres runtime supervisor, CORS
  dev-only).
- **TDD failure modes specific to agent-driven workflows**: tests-and-
  impl in the same turn, weakening the test to make it pass, schema-
  design carve-out, etc. Codex described the cadence well but did not
  call out the agent-specific traps.

## Unified backend subtree

```
backend/
├── Cargo.toml
├── Cargo.lock
├── .env.sample
├── .sqlx/                       # commit; from `cargo sqlx prepare`
├── migrations/
│   └── 20260515000000_init.sql
├── src/
│   ├── main.rs                  # thin bootstrap
│   ├── lib.rs                   # module aggregation + build_router(state)
│   ├── state.rs                 # AppState (codex's split)
│   ├── config.rs
│   ├── db.rs                    # PgPool factory + migrate wrapper
│   ├── error.rs                 # ApiError + IntoResponse
│   ├── telemetry.rs
│   ├── middleware.rs            # request-id + timeout + normalize-path + dev-only CORS
│   └── routes/
│       ├── mod.rs               # builds /api Router; merges siblings
│       ├── health.rs            # GET /healthz (mounted at root, not /api)
│       ├── products.rs
│       └── lists.rs
└── tests/                       # flat, not tests/api/
    ├── common/mod.rs
    ├── health.rs
    ├── products.rs
    └── lists.rs
```

## Unified Cargo.toml dependencies

```toml
[dependencies]
axum         = "0.8"
tokio        = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower        = "0.5"
tower-http   = { version = "0.6", features = [
  "trace", "cors", "timeout", "request-id", "normalize-path", "sensitive-headers"
] }

sqlx = { version = "0.8", features = [
  "runtime-tokio", "tls-rustls-ring-native-roots", "postgres", "macros", "migrate",
  "uuid", "time", "json",
] }

thiserror = "2"
anyhow    = "1"

serde       = { version = "1", features = ["derive"] }
serde_json  = "1"

dotenvy = "0.15"

tracing            = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

uuid = { version = "1", features = ["v7", "serde"] }
time = { version = "0.3", features = ["serde"] }

[dev-dependencies]
http-body-util = "0.1"
tower          = { version = "0.5", features = ["util"] }
serde_json     = "1"
```

Changes from the prior `docs/claude/recommended-layout-2026-05-15.md` version:
- `runtime-tokio` + `tls-rustls-ring-native-roots` replaces
  `runtime-tokio-rustls`.
- `time` replaces `chrono`.
- `sensitive-headers` added to `tower-http`.

## Open items (unresolved across both agents)

Carried over from `docs/claude/open-questions-2026-05-15.md` with this doc's view:

| # | Question | Status |
|---|----------|--------|
| 1 | sqlx offline mode (`.sqlx/` committed, `--check` in CI) | **Recommended on**; needs user confirm |
| 2 | Workspace split | Resolved: single crate |
| 3 | OpenAPI generation | Resolved: skip for v1 |
| 4 | chrono vs time | **Resolved: `time`** (this doc) |
| 5 | Product data source of truth | **Genuinely open — blocks first domain cycle** |
| 6 | Postgres deployment target | Resolved: native on VPS, shared across apps |
| 7 | Auth for v1 | Defer; gate any write endpoint with a Bearer-from-env if needed |
| 8 | `backend/CLAUDE.md` with TDD rules | **Not yet drafted; should land before first code** |

**Q5 (product data) options remaining:**
- One-off `seed.sql` migration with v0's `products.json` scrape baked in.
- `POST /api/products/refresh` endpoint pulling from TJ GraphQL on demand
  (codex's hint).
- Scheduled background sync from TJ GraphQL into Postgres.

This decision affects the *first non-`/healthz`* cycle — not the `/healthz`
slice — so it can wait one cycle.

## First cycle (both agents agree)

**`GET /healthz` returning a small JSON body.** Zero domain logic, but it
exercises every piece of plumbing: `cargo new`, deps, AppState, router
factory, test harness, `#[sqlx::test]`, compose Postgres, Caddyfile
routing, systemd unit. End state: green test, app boots clean, plumbing
proven before any domain code.

After `/healthz` lands, resolve Q5 and start the first real domain cycle.

---

*Last updated: 2026-05-15*
