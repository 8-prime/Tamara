# Backend structure

`src/main.rs` starts the application. `src/app.rs` reads configuration, wires dependencies, starts the HTTP listener and backup scheduler, and handles shutdown.

The main boundaries are:

- `api/`: Axum routes, request extraction, input validation, cookies, response formatting, and HTTP error mapping. Protected handlers use `Authenticated` or `Administrator` extractors and call the auth module for classroom permissions. Handlers never run SQL or open transactions.
- `auth.rs`: password hashing and verification, opaque session tokens, login throttling, first-run setup, and administrator/owner/member policies. It accepts ordinary Rust values and a `Database`, so it can be used without HTTP. Throttling state is shared across cloned application states; permissions and sessions are read from storage on each request.
- `db/`: the private SQLite pool, connection settings, migrations, catalog initialization, queries, and transactions, grouped by feature. Repository methods accept shared records and commands, never Axum extractors or responses. They enforce relational invariants and own complete transactions, including invitation redemption, roster snapshots, period copies, batch writes, and homework reconciliation. `db/backup.rs` owns database snapshots and backup retention.
- `models.rs`: shared records and mutation commands. Account password hashes have a separate record type that cannot be serialized as a teacher response.
- `error.rs`: application error categories and safe messages. Only `api/error.rs` converts these into HTTP status codes and JSON.
- `validation.rs` and `util.rs`: shared validation and ID/time helpers. `pdf.rs` and `archive.rs` render exports from already-loaded data.

The request flow is `HTTP → API extractor/handler → auth policy + repository → SQLite`. Repositories return data or application errors; the API decides how to represent them over HTTP. `AppState` carries the repository, auth service, cookie configuration, and optional backup service. The pool is kept private inside `db`.

When adding a feature, put its handler and request types in `api/`, its queries and complete aggregate transactions in `db/`, and any shared records in `models.rs`. Keep authorization decisions in `auth.rs`, and check classroom access before loading feature data. Related writes belong in one repository method so callers cannot accidentally commit only part of an operation. A transaction must not cross back into a handler or span PDF rendering.

Run `cargo fmt --all --check`, `cargo test --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` from the repository root. The router tests preserve the existing HTTP contract; `tests/layers.rs` also exercises authentication and repository rollback directly without HTTP, and the auth unit tests cover shared throttling state.
