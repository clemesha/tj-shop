# TJ Shop (v0)

Trader Joe's shopping list with live product data, images, and prices from the TJ GraphQL API.

> **Status:** archived as v0 on 2026-05-15. Work on v1 has begun at the repo root. The live URL https://clemesha.github.io/tj-shop/ will 404 until v1 ships a root `index.html` (or a redirect is added). v0 is still viewable at https://clemesha.github.io/tj-shop/v0/.

**Google Sheet:** https://docs.google.com/spreadsheets/d/1ouaydzEr1hX7_WqjJRJ2w5nExw-tJhz1X4IWIL32NHM/edit

## Fetching product data

```js
const resp = await fetch('https://www.traderjoes.com/api/graphql', {
  method: 'POST',
  headers: {
    'content-type': 'application/json',
    'user-agent': 'Mozilla/5.0 ...',
    'origin': 'https://www.traderjoes.com',
    'referer': 'https://www.traderjoes.com/home/products',
  },
  body: JSON.stringify({
    query: `{ products(search: "bananas", pageSize: 5, filter: { published: { eq: "1" } }) {
      items { item_title sku retail_price primary_image sales_size sales_uom_description }
    }}`
  })
});
```

Images: prepend `https://www.traderjoes.com` to `primary_image` paths. Prices are from the Chicago South Loop store (code 701).
