# tj-shop

A personal Trader Joe's shopping app. Built for one real user's real
shopping habits.

## What this app is

Two views, one focus:

- **Shop view** — a rich shopping list. Browse the products I care about,
  build a list for the next trip, check items off as I go.
- **Store view** — a rich per-store interface. Flexible CRUD over the
  products I actually buy at each store I actually shop at.

The scope is **Trader Joe's only**. Not a generic grocery app. Not a
multi-chain product database. TJ across multiple TJ stores (Pacific
Beach and La Jolla today), modeling TJ's reality cleanly.

## Build philosophy

These tenets steer every decision. When in doubt, check them.

**Build what I'd actually use first.** A real shopping flow that fits
real habits beats a feature-complete sketch. The first version should
be immediately useful for the way I shop today — not a sketch that
becomes useful three iterations from now.

**Curated, not comprehensive.** Per-store data is *what I actually buy
at that store*, not a mirror of TJ's full inventory. The Store view is
where I maintain that curated set — manual entries, price overrides,
notes. The app earns its keep by managing the subset I care about, not
by being a comprehensive TJ database.

**Do one thing really well.** TJ is the scope. No Safeway, no Whole
Foods, no general grocery abstraction. The schema's job is to model TJ
cleanly, not be a universal grocery framework.

**Catalog vs curation, clearly separated.** TJ's catalog (name, SKU,
image, size, category) is national and immovable. Per-store data
(price, availability, my notes) is mine. Both deserve their own table.

**UX over abstraction.** Convenience for real-world usage patterns
drives feature shape. If a "clean abstraction" makes the UI clunkier,
the abstraction loses.

**Add generality only when earned.** When two concrete needs share a
pattern, extract. Until then, the simpler shape wins. Don't design for
hypothetical second use cases.

## Where things live

- **Architecture and scaffolding decisions:** `docs/arch-plan-claude-2026-05-15.md` —
  the v1 master plan (Rust + Axum + SQLx + Postgres + React/Vite).
- **Backend agent rules:** `backend/CLAUDE.md` — TDD discipline,
  when-to-skip-a-test guidance, what-not-to-build.
- **Legacy app:** `v0/` — archived first version (vanilla JS, Google
  Sheets backend). Reference only; not maintained.
