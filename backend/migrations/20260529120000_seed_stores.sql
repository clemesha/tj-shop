-- Seed the two TJ stores we shop at.
-- Idempotent: re-running the migration leaves existing rows untouched.

insert into stores (id, tj_code, name, city, state)
values
    (gen_random_uuid(), '21', 'Pacific Beach', 'San Diego', 'CA'),
    (gen_random_uuid(), '20', 'La Jolla',      'La Jolla',  'CA')
on conflict (tj_code) do nothing;
