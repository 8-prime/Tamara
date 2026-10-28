# Tamara

Tamara helps teachers manage classrooms and follow students' learning development. This first feature implements the supplied Bremen mathematics development overview directly, using a Rust/Axum backend, SQLite, and a Svelte 5 frontend.

## Run locally

Prerequisites: Rust 1.96+, Node.js 22.12+ (or 24+), and pnpm 11.8.

```sh
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend build
cargo run
```

Open **http://127.0.0.1:3000**. SQLite is created at `data/tamara.sqlite`; migrations run automatically. Run commands from the repository root.

On an empty database, the server prints a one-time **setup token** to its terminal. Choose “Konto erstellen” and enter it to create the first administrator. The setup token stops working as soon as the first account exists. You can supply `TAMARA_SETUP_TOKEN` (at least 24 characters) instead of generating a new token on each start before setup.

The administrator opens **Registrierungscodes** to issue invitations. Each token works once, expires after seven days, can be revoked, and is shown in full only at creation. Give the code to the intended teacher. Tokens are stored as SHA-256 hashes. Registrations cannot proceed without a valid token. New teachers are regular users, with access only to their own or explicitly shared classrooms.

For development with frontend hot reload, run `cargo run` and `pnpm --dir frontend dev` in two terminals. Vite proxies `/api` to the Rust server. For a single command, use `./scripts/dev.sh`.

## Teaching workflow

1. Create a classroom with a name and school year.
2. Add students, one name per line. Student IDs keep children with identical names distinct.
3. Create a named grading period. Start empty or copy any period in the same classroom. Copying takes an independent snapshot of both affirmative and negative assessments; unassessed entries stay unassessed.
4. Edit in **Einzelkind**, **Klassenübersicht**, or **Nach Kompetenz**. All three modes update the same records and save automatically. In the competency view, select students and apply a status to the selection, or grade individual students directly. Unselected students are unchanged.
5. Filter by competency section, development level, or search text. The classroom matrix paginates competencies and scrolls horizontally.
6. Export a student's PDF from the individual view. The original 20-page document is retained, affirmative assessments get crosses beside their statements, and the cover receives the student name, school year, and grading-period name.

Statuses are **Erreicht** (`yes`), **Noch nicht** (`no`), and **Nicht beurteilt** (no assessment row). The latter two remain distinct in SQLite and both leave the printed form blank. The reference form provides no separate mark for a negative or unassessed result.

The classroom creator is its owner. Under **Gemeinsam arbeiten**, they can share with another registered teacher by email or revoke access. Collaborators can edit students, periods, and assessments. Only the owner can change membership or delete students. Deleting a student requires confirmation and removes that student's assessments across every period in that classroom. Renaming a student applies across all periods.

## The mathematics form

The PDF at the repository root is embedded in the Rust executable, together with `backend/criteria.json`. The checked-in catalog contains **354 criteria** across **10 sections**, on pages **5–16**, retaining the source wording and levels Basis–10.

Each criterion includes an ID, domain, section, level, and a mark location. Coordinates use **PDF points from the top-left corner**, pages are **1-based**, and the exporter converts the y coordinate to the PDF bottom-left coordinate system. Crosses are drawn in the gap between the strand symbol and statement. The `math_criteria` SQLite table also stores this mapping. The source SHA-256 is recorded in the catalog and checked by a Rust test. The original template is never overwritten.

The form is intentionally specific to this feature. Core account, classroom, and period tables are separate from `math_criteria` and `math_assessments` so future tools can have their own data models.

To regenerate the catalog after an intentional change to the template:

```sh
python3 -m venv .venv
.venv/bin/pip install 'PyMuPDF==2.0'
.venv/bin/python scripts/extract_criteria.py
```

Review the generated labels, levels, and rendered cross positions before changing a deployed catalog. Existing IDs are page/row based: changing the template's row order requires an explicit data migration. The extractor is a development utility; the server has no Python dependency. DejaVu Sans is embedded for cover text; its license is included in `backend/assets`. PDF text uses that font's glyph coverage; full complex-script shaping and unsupported glyph fallback are not implemented.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | `sqlite:data/tamara.sqlite` | SQLite file; foreign keys, WAL and a busy timeout are enabled. |
| `TAMARA_BIND` | `127.0.0.1:3000` | HTTP bind address. |
| `TAMARA_SETUP_TOKEN` | Generated and printed on first run | Initial admin setup only; at least 24 characters. |
| `TAMARA_SECURE_COOKIES` | `false` | Set to `true` behind HTTPS. |
| `TAMARA_FRONTEND_DIR` | `frontend/dist` | Built frontend directory. |
| `RUST_LOG` | `tamara=info,tower_http=info` | Logging filter. |

Passwords use Argon2id. Sessions use random opaque tokens, stored hashed, with HttpOnly/SameSite=Strict cookies and seven-day expiry. State-changing API calls require `X-Tamara-Request: 1`; there is no cross-origin API access. Sharing checks also guard PDF exports. Bulk grading and period copies are transactional; invitation redemption is serialized with an immediate transaction. Login attempts are throttled per email in memory.

For deployment, use HTTPS, secure cookies, a private terminal/log for initial setup, and persistent storage for the SQLite database. Back up SQLite using its online backup API or stop the server before copying the database and its WAL files together. Password reset, email delivery, bulk PDF archives, moving students between classes, and live synchronization between browsers are outside this first version. Concurrent edits to the same assessment use the last saved value; reloading a classroom retrieves other teachers' changes.

## Checks

```sh
pnpm --dir frontend check
pnpm --dir frontend build
cargo fmt --all --check
cargo test
```

The Rust suite exercises the real Axum router without requiring a network listener: invitation issuance/revocation/expiry and concurrent redemption, admin permissions, cookies and authentication, classroom isolation and sharing revocation, tri-state grading, atomic bulk updates, independent period copies, student management, and PDF export. The PDF tests check the template hash, mapping, page count, and the pages receiving overlays.

The workflow test writes `output/pdf/qa-all-criteria.pdf`, a synthetic fully marked export for visual review. `scripts/smoke-test.py` additionally exercises a real HTTP server, restart persistence, and PDF validation when local sockets and Poppler's `pdfinfo` are available. It uses a disposable database and cleans up its server.
