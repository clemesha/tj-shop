#!/usr/bin/env node
//
// One-off: convert v0/products/all-categories.json into a Postgres seed
// migration for the v1 schema. Re-run when refreshing the seed.
//
//   node scripts/convert-v0-seed.js
//
// Output: backend/migrations/20260529130000_seed_catalog.sql

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const inputPath = path.join(root, 'v0', 'products', 'all-categories.json');
const outputPath = path.join(
  root,
  'backend',
  'migrations',
  '20260529130000_seed_catalog.sql',
);

const data = JSON.parse(fs.readFileSync(inputPath, 'utf-8'));

function sql(s) {
  if (s == null || s === '') return 'null';
  return `'${String(s).replace(/'/g, "''")}'`;
}

const categories = Object.entries(data).map(([tj_id, { name }], i) => ({
  tj_id,
  name,
  position: i,
}));

const products = Object.entries(data).flatMap(([tj_id, { items = [] }]) =>
  items
    .filter(item => item.sku && item.name)
    .map(item => ({
      sku: item.sku,
      name: item.name,
      size: item.size || null,
      image_url: item.image || null,
      tj_category_id: tj_id,
    })),
);

const out = [];

out.push('-- Seed: TJ catalog (categories + national product canon).');
out.push('-- Source: v0/products/all-categories.json (archived scrape).');
out.push('-- No prices: those are per-store, populated via store_products curation.');
out.push('-- Idempotent: ON CONFLICT DO NOTHING on (tj_id) and (sku).');
out.push('--');
out.push(`-- Categories: ${categories.length}`);
out.push(`-- Products:   ${products.length}`);
out.push('');

out.push('insert into categories (id, tj_id, name, position) values');
out.push(
  categories
    .map(
      c =>
        `    (gen_random_uuid(), ${sql(c.tj_id)}, ${sql(c.name)}, ${c.position})`,
    )
    .join(',\n'),
);
out.push('on conflict (tj_id) do nothing;');
out.push('');

out.push(
  'insert into products (id, sku, name, size, image_url, category_id, is_manual)',
);
out.push(
  'select gen_random_uuid(), v.sku, v.name, v.size, v.image_url, c.id, false',
);
out.push('from (values');
out.push(
  products
    .map(
      p =>
        `    (${sql(p.sku)}, ${sql(p.name)}, ${sql(p.size)}, ${sql(p.image_url)}, ${sql(p.tj_category_id)})`,
    )
    .join(',\n'),
);
out.push(') as v(sku, name, size, image_url, tj_category_id)');
out.push('join categories c on c.tj_id = v.tj_category_id');
out.push('on conflict (sku) do nothing;');
out.push('');

fs.writeFileSync(outputPath, out.join('\n'));
console.log(`wrote ${path.relative(root, outputPath)}`);
console.log(`  ${categories.length} categories`);
console.log(`  ${products.length} products`);
