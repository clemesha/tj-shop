# Changelog

## 2026-04-22

- Researched Trader Joe's website; discovered public GraphQL API at `traderjoes.com/api/graphql` (Adobe Commerce/Magento 2, no auth required, ~2,700 products)
- Akamai CDN blocks curl/Python requests; Node.js `fetch` works with browser-like headers
- Fetched product data (name, price, image, size) for 3 items: Bananas, Black Cherry Vanilla Sparkling Water, Organic Caesar Salad Kit
- Downloaded product images locally from TJ CDN
- Built static HTML shopping list page with product images, prices, and total
- Created Google Sheet via workspace-mcp (google_workspace_mcp) with the same data, set to public read-only: https://docs.google.com/spreadsheets/d/1ouaydzEr1hX7_WqjJRJ2w5nExw-tJhz1X4IWIL32NHM/edit
- Deployed to GitHub Pages: https://clemesha.github.io/tj-shop/

### Trader Joe's API notes

- **Endpoint:** `POST https://www.traderjoes.com/api/graphql`
- **No auth** but requires browser-like headers (User-Agent, Origin, Referer); Akamai blocks curl/Python, Node.js fetch works
- **Search:** `products(search: "term", pageSize: 100, filter: { published: { eq: "1" } })`
- **Fields:** `item_title`, `sku`, `retail_price`, `primary_image`, `sales_size`, `sales_uom_description`, `nutrition`, `ingredients`, `allergens`, `category_hierarchy`
- **Images:** relative paths, prepend `https://www.traderjoes.com`
- **Prices:** per-store (pass `store_code`); default is Chicago South Loop (701)
- **Catalog:** ~2,718 products, paginate with `pageSize`/`currentPage`

### workspace-mcp setup notes

- Server: [taylorwilsdon/google_workspace_mcp](https://github.com/taylorwilsdon/google_workspace_mcp), installed at `/Users/agc/work/clemesha/mail-triage/google_workspace_mcp`
- Must source env vars from `~/.google_workspace_mcp/.env` when starting
- Start: `cd google_workspace_mcp && source ~/.google_workspace_mcp/.env && export GOOGLE_OAUTH_CLIENT_ID GOOGLE_OAUTH_CLIENT_SECRET OAUTHLIB_INSECURE_TRANSPORT && uv run main.py --transport streamable-http --tools sheets drive`
- Add to Claude Code: `claude mcp add --transport http workspace-mcp http://localhost:8000/mcp`
- Required enabling Google Sheets API and Google Drive API in GCP console (project 481258549660)
