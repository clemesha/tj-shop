default:
    @just --list

# --- database (dev only) ---

db-up:
    docker compose up -d db

db-down:
    docker compose down

db-reset:
    docker compose down -v
    docker compose up -d db

# --- backend ---

backend-run:
    cd backend && cargo run

backend-test:
    cd backend && cargo test

backend-check:
    cd backend && cargo fmt --check
    cd backend && cargo clippy --all-targets -- -D warnings
    cd backend && cargo test

# --- frontend (placeholder until frontend/ exists) ---

frontend-dev:
    cd frontend && npm run dev

frontend-build:
    cd frontend && npm run build

# --- combined ---

check:
    just backend-check
