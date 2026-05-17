# backend/ — agent rules

This crate is developed **test-first** by the coding agent. The rules
below override any default coding instincts.

## TDD discipline

- **Tests before implementation.** Never write a handler/repo/util and
  its test in the same turn. The test goes in first, gets reviewed,
  fails for the right reason, then the implementation lands.
- **Never weaken a test to make it pass.** If a test seems wrong,
  escalate ("I think the test is wrong because X") instead of editing
  it to match buggy behavior.
- **Body assertions are required.** Don't assert only on status codes —
  assert on the response body shape and the relevant fields.
- **No DB mocking.** Integration tests use `#[sqlx::test]` against real
  Postgres. The test framework gives each test a fresh database with
  migrations applied and hands the test a `PgPool`.
- **Exercise the router via `tower::ServiceExt::oneshot`.** No real TCP
  listener in tests. Build the same `Router` the binary builds, in-process.
- **Schema/migration design is the only TDD carve-out.** Exploratory
  iteration on table shape is fine; the moment the shape stabilizes,
  the migration is locked in by a passing integration test.
- **Done = `just backend-test` is green.** A task is not finished until
  the full test suite passes. Before handoff, also run `just backend-check`
  (fmt + clippy + tests).

## Workflow per endpoint

1. User writes a 3–5 line spec: path, method, request shape, response
   shape, error cases.
2. Agent writes failing integration test(s) — happy path + 1–2 error
   cases. No implementation.
3. User reviews the test. **Gate point** — agent stops here.
4. Agent writes migration (if needed) and handler — minimum code to
   pass.
5. Agent runs the narrow test, then the full suite. Reports green.
6. Optional refactor pass with the tests as a safety net.

## When to skip a test

TDD discipline above is the default. It is not a license to write tests
that have no signal value. Before writing a test, ask: **what realistic
change would make this fail?** If the only answer is "deleting the code
under test," the compiler already covers it — skip the test.

Worth writing:

- Endpoints with branches, validation, or error paths.
- SQL queries — the integration test proves the schema, the query, and
  the wiring all line up.
- Anything touching money, auth, or data integrity.
- Tests that pin the request/response contract — the test *is* the API
  documentation.

Not worth writing:

- One-liner handlers returning a constant. (One smoke test for the
  pattern is enough; don't repeat it per constant.)
- Internal types/helpers with no branching. They get exercised
  transitively by the endpoint tests above.
- Glue that the framework or type system already guarantees (e.g.,
  "axum routes the URL to the function I registered").
- Implementation-detail assertions ("calls `db.query` with exactly this
  string"). These are refactor-hostile and regression-neutral.

When in doubt, prefer one fat integration test that exercises the real
behavior over many narrow tests that pin internal structure.

## What lives where

- Route modules under `src/routes/`. Inline `sqlx::query!` /
  `query_as!` in handlers is fine. Extract a query into a `repos/`
  module only when the **same** query is used by **two** handlers.
- Domain types live in their route file until two files need the same
  one. Then promote to a flat `src/domain.rs`.
- `AppState` lives in `src/lib.rs`. Don't create a separate `state.rs`
  unless the struct grows past ~5 fields.
- Test fixtures live in `tests/fixtures/`. Helper code shared across
  tests lives in `tests/common/mod.rs`.

## What not to do

- No repository traits, no service layer, no clean-architecture
  scaffolding. Direct SQLx is fine.
- No `Db` wrapper around `PgPool` — it's already `Arc`'d.
- No ORM, no query builder. Raw SQL via the SQLx macros.
- No mock layer between handlers and the database.
- No premature feature flags or backwards-compatibility shims.
