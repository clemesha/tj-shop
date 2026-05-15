# Open questions

Genuine forks in the road, not bikeshedding. Each item has a "recommended
default if you don't want to decide" so v1 isn't blocked.

## 1. sqlx offline mode (`cargo sqlx prepare` + `.sqlx/`)

**Question:** do you want CI to build without a live Postgres?

- `sqlx::query!` macros check SQL against a real database at compile time.
  That requires `DATABASE_URL` to be set during `cargo build`.
- `cargo sqlx prepare` snapshots the query metadata to `.sqlx/*.json`,
  committed to git. Then `SQLX_OFFLINE=true cargo build` works without a
  database — needed for most CI setups and for `cargo install`.
- Cost: one extra step (`cargo sqlx prepare`) every time you change a query
  or migration. Forgetting it means broken CI.

**Recommendation:** turn it on from day one. The friction of `cargo sqlx
prepare` is small; the cost of bolting it on later (regenerating dozens of
files, untangling drift) is higher. Add a CI step that runs `cargo sqlx
prepare --check` to fail PRs that forgot.

**Decide:** confirm this and add `.sqlx/` to the commit list (not
`.gitignore`).

## 2. Workspace vs single crate

**Question:** one crate now, or carve out `tj-shop-core` / `tj-shop-api` up
front?

- Single crate (the recommended layout) is shorter and faster to compile
  for a small app.
- Workspace lets you reuse types (e.g. a `Product` struct) in a future CLI
  or worker without circular deps.
- Splitting later is mechanical but annoying (lots of `pub` to add).

**Recommendation:** single crate for v1. Split only when a second
deployable exists.

**Decide:** anything coming after v1 that would need to share code with
the API? (background jobs, a CLI, a worker process)

## 3. OpenAPI generation (utoipa vs aide vs hand-written vs none)

**Question:** auto-generated API docs from the start, or skip?

- **utoipa** — derive macros on handlers; widely used (clean_axum_demo,
  softwaremill template). Adds compile-time cost and macro noise.
- **aide** — newer, less invasive; tracks schema separately. Smaller
  community.
- **none** — write a Markdown API doc by hand. Honest for a 5-endpoint
  API.

**Recommendation:** none for v1. The frontend is in the same repo (or
about to be); types can be hand-mirrored in TypeScript. Revisit when (a)
external consumers appear, or (b) you have ≥10 endpoints.

**Decide:** confirm "no OpenAPI yet."

## 4. Date/time crate — chrono vs time

**Question:** `chrono = "0.4"` (recommended above) or `time = "0.3"`?

- chrono: wider ecosystem, slightly older API, has had soundness CVEs in
  the past.
- time: cleaner API, used by sqlx examples, slightly narrower ecosystem.

Both map to Postgres `timestamptz` cleanly via sqlx feature flags
(`chrono` or `time`).

**Recommendation:** chrono. It's what most Axum tutorials use and matches
serde_json date handling out of the box. Switching later is a sed.

**Decide:** any preference? Otherwise default to chrono.

## 5. Seed data / fixtures

**Question:** how does v1 get the TJ product catalog into the DB?

Options:
- A one-off `seed.sql` migration that bulk-inserts the v0 `products.json`
  scrape.
- A `cargo run --bin seed` binary that hits the TJ GraphQL API at
  build/deploy time.
- A `tj-shop-cli refresh` admin command run manually.

These have very different operational implications. v0 reads from a
static JSON file shipped with the frontend; v1's whole point is presumably
to centralize that.

**Decide:** what's the source of truth for product data in v1 — periodic
scrape of TJ GraphQL into Postgres, or per-request proxy, or pre-loaded
seed?

## 6. Database deployment target — RESOLVED

**Resolved:** Postgres runs on the same small VPS as the Rust backend,
on loopback (`127.0.0.1:5432`). Single instance, single replica, no
PgBouncer. See `recommended-layout-2026-05-15.md` → "Deployment topology".

Consequences baked in:
- `runtime-tokio-rustls` stays the default. The local loopback connection
  is plaintext; TLS is unnecessary inside the box.
- `PgPoolOptions::max_connections(10)`. Tune up only if you see
  `acquire_timeout` events.
- `sqlx::migrate!()` at startup is safe (single instance — no leader-lock
  problem).
- Backups are the host's problem (`pg_dump` + cron + offsite copy). Out
  of scope for this doc; flag for the deploy runbook.

## 7. Authentication scope for v1

**Question:** purely public API, or session/cookie-based admin?

The brief says "auth may eventually exist but is not in scope for v1." The
recommended layout reflects this. But: do you need *any* gating for v1
(e.g. a single shared admin token to write to the product catalog), or is
literally everything open?

**Recommendation:** if there's any write endpoint, gate it with a single
`Bearer <token>` checked against an env var. Five lines of middleware,
zero auth library needed. Otherwise, all public.

**Decide:** any v1 endpoint that should not be world-writable?

---

*Last updated: 2026-05-15*
