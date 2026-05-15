# TJ Shop

## GitHub

Always use the `clemesha` GitHub account (personal). Before any git push, verify with `gh auth status` and switch if needed:

```
gh auth switch --user clemesha
```

## Trader Joe's API

- Endpoint: `POST https://www.traderjoes.com/api/graphql`
- Default store: Pacific Beach, San Diego (store_code: `21`). Also shops at La Jolla (store_code: `20`).
- Node.js `fetch` works; curl/Python get blocked by Akamai.
- Images: use TJ CDN URLs directly (e.g. `https://www.traderjoes.com/content/dam/trjo/products/...`), don't download locally.

## Codebase Principles

- Vanilla JS — no frameworks, no build step, no bundler.
- Single HTML file (index.html) with inline styles and scripts.
- Hash-based routing:
  - `/` (no hash) → Shop View, latest active list. This is the default shareable link.
  - `#shop/2026-04` → Shop View for a specific list.
  - `#store` → Store View (planning/browsing mode).
- Google Sheet as database, Apps Script as hidden API layer.
- Install dependencies locally, never globally.

## Apps Script API

- Web app URL: `https://script.google.com/macros/s/AKfycbx74Pzy2XRQP6Q2EDVRdegPJR9cWnNm1Ada2TkrhvrTsGefL8KUB_I6enBIkAZbHI50Cw/exec` (auto-rewritten by `deploy.sh` on every deploy)
- Google Sheet ID: `1V0d4ejGLPfGwOvKJzhjHxvC446m31R0YCeVnZQ9WNlk`
- GET endpoints (public): `getLists`, `getList`, `getCustomProducts`
- POST endpoints (token required): `saveListItems`, `toggleChecked`, `removeListItem`, `addCustomProduct`, `editCustomProduct`, `deleteCustomProduct`, `createList`
- Code lives in `apps-script/Code.gs`. To deploy: `cd apps-script && ./deploy.sh "<description>"`.
- `deploy.sh` works around the Apps Script API bug where `deployments.update` (and `clasp deploy -i`) resets Web App entry points: instead of bumping an existing deployment's version, it creates a *new* deployment each time and rewrites the Web app URL in `index.html` and `CLAUDE.md`. Old deployments are left in place for rollback — clean them up via `clasp deployments` + UI when sprawl gets annoying.
- `appsscript.json` must keep its `"webapp": { "executeAs": "USER_DEPLOYING", "access": "ANYONE_ANONYMOUS" }` block — that's what makes new deployments inherit ANYONE_ANONYMOUS access.

## Deploy

After pushing to main, always:
1. Trigger a Pages build: `gh api repos/clemesha/tj-shop/pages/builds -X POST`
2. Poll until built, then paste the live URL into chat: https://clemesha.github.io/tj-shop/

## TODO

### Fill product catalog gaps
TJ's API only covers ~2,720 of their in-store products. Known gaps include third-party brands with multiple flavors listed as a single item (e.g. Spindrift has 8+ flavors in-store but only 1 in the API). Plan:
1. **Create `products/manual.json`** — a simple file for manually adding products not in the API. Same format as API data (name, sku, price, size, image, category). Merge into page at build time.
2. **Cross-reference Instacart** — Instacart lists TJ products with prices/images. Diff against our catalog to surface missing items.
3. **Open Food Facts / UPC databases** — cross-reference known TJ UPCs to fill in community-contributed data.
4. **In-store photo capture** — snap shelf photos, OCR to extract product names/prices. Most effort but catches everything.
