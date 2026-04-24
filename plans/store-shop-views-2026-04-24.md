# TJ Shop — Store & Shop Views Plan

## Overview

Split the app into two views backed by Google Sheets as a simple database:

- **Store View** — Browse/search the full TJ catalog, select items + quantities, manage custom products. This is the "planning" interface.
- **Shop View** — The in-store shopping checklist. Minimal UI: photo, item name, quantity needed, checkbox to mark as grabbed, and a rough total at the bottom.

---

## Views

### Store View (Planning Mode)

What it does:
- Browse all ~2,700 products by category (what we have now)
- Search/filter products
- Tap +/- to set quantities for a shopping list
- **Add custom products** not in the TJ API (manual CRUD)
- "Save to list" pushes selections to the Google Sheet
- See which items are already on the current list

Custom product CRUD:
- Simple inline form: name, price, size, category, optional image URL
- Stored in a "Custom Products" tab in the Sheet
- Merged into the Store View alongside API products
- Edit/delete via a small icon on each custom item

### Shop View (In-Store Mode)

What it does:
- Shows only items on the current list (quantity > 0)
- Each item: photo, name, quantity badge, checkbox
- No prices visible (just the total estimate at bottom)
- Check off items as you shop
- Checked state persists (saved to Sheet or localStorage)
- Sorted by category to match store layout

UI pattern:
- Large tap targets for checkboxes (in-store, one-handed use)
- Checked items fade/move to bottom
- Progress indicator: "5 of 12 items"

---

## Data Model — Google Sheets

### Option A: Single Spreadsheet, Multiple Tabs

**Tab: `products_custom`** — Manually added products
| sku | name | price | size | category | image_url |
|-----|------|-------|------|----------|-----------|
| custom-001 | Spindrift Raspberry Lime | 7.49 | 12 Fl Oz | Water | (optional) |

**Tab: `lists`** — Shopping list metadata
| list_id | label | created | status |
|---------|-------|---------|--------|
| 2026-04 | April 2026 | 2026-04-24 | active |
| 2026-05 | May 2026 | 2026-05-01 | draft |

**Tab: `list_items`** — Items on each list
| list_id | sku | quantity | checked |
|---------|-----|----------|---------|
| 2026-04 | 048053 | 3 | TRUE |
| 2026-04 | custom-001 | 1 | FALSE |

### Option B: One Tab Per List

Each month gets its own tab (e.g. "April 2026") with columns: sku, name, quantity, checked. Simpler but harder to query across lists.

**Recommendation: Option A.** The multi-tab normalized approach is cleaner, extends to arbitrary list periods, and makes it easy to add features like "copy last month's list" or "most frequently bought items."

---

## List Periods — Extensibility

Start with monthly lists, but the `lists` table is generic:

```
list_id: string    — any unique key (e.g. "2026-04", "week-2026-04-21", "party-july4")
label: string      — display name ("April 2026", "Week of Apr 21", "July 4th BBQ")
created: date
status: string     — "active" | "draft" | "archived"
```

This naturally extends to:
- Weekly lists (`week-2026-04-21`)
- Event-based lists (`party-july4`)
- Recurring templates (`template-weekly-staples`)

The UI just needs a list picker dropdown. No schema changes needed to support new periods.

---

## Architecture Options

### Option 1: Static HTML + Google Sheets API (client-side)

How: The page calls Google Sheets API directly from the browser using an API key (read) or OAuth (write).

Pros:
- No server needed, stays on GitHub Pages
- Simple, everything in one HTML file

Cons:
- API key exposed in client code (read-only is fine, writes need OAuth)
- OAuth flow in-browser is clunky for a personal tool
- CORS and auth complexity

### Option 2: Static HTML + Apps Script Web App (middleware)

How: A Google Apps Script attached to the Sheet exposes a simple REST API (`doGet`/`doPost`). The HTML page calls that API.

Pros:
- No API keys in client code
- Apps Script has native Sheet access, no auth needed
- Free, no server to maintain
- Can deploy as "execute as me" so no OAuth for the client
- Still hosted on GitHub Pages

Cons:
- Apps Script has cold start latency (~1-3s)
- Apps Script URL is ugly (can alias)
- Deployment is separate from the main repo

### Option 3: Small Node server (e.g. on Cloudflare Workers or Fly.io)

How: Lightweight server proxies Sheets API calls, handles auth server-side.

Pros:
- Clean API, fast
- Can also proxy TJ API calls

Cons:
- Overkill for a personal shopping list
- Another thing to maintain/deploy

**Recommendation: Option 2 (Apps Script).** It's the sweet spot — no server cost, native Sheets integration, and the client stays a simple static page on GitHub Pages. The Apps Script acts as a thin API layer:

```
GET  /exec?action=getList&listId=2026-04     → list items
POST /exec?action=saveList                    → save selections
POST /exec?action=addCustomProduct            → CRUD for custom products
POST /exec?action=toggleChecked               → check/uncheck in Shop View
```

---

## Implementation Steps

### Phase 1: Google Sheet + Apps Script API
1. Create the Google Sheet with 3 tabs (`products_custom`, `lists`, `list_items`)
2. Write Apps Script with endpoints: getList, saveList, getCustomProducts, addCustomProduct, editCustomProduct, deleteCustomProduct, toggleChecked
3. Deploy as web app ("execute as me, anyone can access")
4. Test with curl

### Phase 2: Split the UI
1. Add a view toggle (Store / Shop) — could be a tab bar or URL hash (`#store` / `#shop`)
2. Store View: current UI + "Save List" button + custom product form
3. Shop View: filtered to current list items, checkbox UI, no prices, total at bottom

### Phase 3: List Management
1. List picker in the UI (dropdown or simple tabs)
2. "New List" button (creates a new entry in `lists` tab)
3. "Copy from previous" to bootstrap a new month from the last one

### Phase 4: Polish
1. Offline support (service worker cache for in-store use with spotty signal)
2. "Most bought" suggestions based on list history
3. Share a Shop View link with someone (read-only, no auth needed)

---

## Open Questions

- **Auth for writes:** Apps Script "execute as me" means anyone with the URL can write. Fine for personal use. If sharing, could add a simple token check.
- **Conflict handling:** If two people edit the list simultaneously (unlikely for personal use). Apps Script handles this via Sheets' built-in locking.
- **Product data freshness:** How often to re-fetch from TJ API? Could be manual ("refresh catalog" button) or a scheduled script.
- **Image fallbacks for custom products:** Allow pasting a URL, or use a generic placeholder.
