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

## Docker

```sh
docker compose up --build -d
docker compose logs tamara
```

Open **http://localhost:3000**. On a fresh database, the logs contain the setup token for creating the first administrator. The named `tamara-data` volume preserves the SQLite database when the container is replaced. `docker compose down` keeps that volume; `docker compose down --volumes` deletes it.

The image builds the frontend with pnpm and the backend with Cargo in separate build stages. Its runtime contains the Rust executable, compiled frontend, and embedded-font license. A single Rust process serves the API, HTML, JavaScript, and CSS; no Node server or reverse proxy is required. Frontend routes return the app shell, while unknown API routes and missing bundles remain 404 responses.

Without Compose:

```sh
docker build -t tamara:local .
docker run -d --name tamara -p 3000:3000 -v tamara-data:/data tamara:local
docker logs tamara
```

The container listens on `0.0.0.0:3000`, uses `/data/tamara.sqlite`, and runs as UID/GID `10001:10001`. When using a host directory instead of a named volume, make it writable by that UID. Configuration variables below can be supplied with `docker run --env`. `docker stop` sends SIGTERM and the server finishes in-flight requests before exiting.

## Teaching workflow

1. Create a classroom with a name and school year.
2. Add students, one name per line. Student IDs keep children with identical names distinct.
3. Create a named grading period. Start empty or copy any period in the same classroom. Copying takes an independent snapshot of both affirmative and negative assessments; unassessed entries stay unassessed.
4. Edit in **Einzelkind**, **Klassenübersicht**, or **Nach Kompetenz**. All three modes update the same records and save automatically. In the competency view, select students and apply a status to the selection, or grade individual students directly. Unselected students are unchanged.
5. Filter by competency section, school grade, development level, or search text. The grade filter follows the overlapping ranges on PDF page 3: grade 1 = stages 01–04, grade 2 = 03–06, grade 3 = 05–08, grade 4 = 07–10. The exact-stage filter narrows within that range; choose all grades to include Basis. Changing topics preserves the selected grade. Topics are grouped by domain in the dropdown. All three editing views retain the PDF’s blue topic hierarchy, competency symbols, source order, and light/dark gray target-stage bands. The classroom matrix paginates competencies and scrolls horizontally.
6. Export a student's PDF from the individual view, or choose **Alle PDFs als ZIP** beside the period selector to export the whole class for that period. The ZIP includes one PDF for every child, including children without assessments. Numbered, sanitized filenames keep duplicate names distinct. The roster and assessments are read together as a snapshot. The original 20-page document is retained, affirmative assessments get crosses beside their statements, and the cover receives the student name, school year, and grading-period name.

Each **Erreicht** assessment stores an `achieved_on` observation date independently of its last edit time. It defaults to the teacher’s local date in the UI (UTC date for API callers that omit it), appears in all three editing views, and can be adjusted for earlier observations. Saving an already achieved competency preserves its date; copying a period preserves dates as an independent snapshot. Switching to **Noch nicht** or **Nicht beurteilt** clears the reached date in that period. Existing affirmative assessments are backfilled from their last saved timestamp, the only historical date available.

Statuses are **Erreicht** (`yes`), **Noch nicht** (`no`), and **Nicht beurteilt** (no assessment row). The latter two remain distinct in SQLite and both leave the printed form blank. The reference form provides no separate mark for a negative or unassessed result.

The classroom creator is its owner. Under **Gemeinsam arbeiten**, they can share with another registered teacher by email or revoke access. Collaborators can edit students, periods, and assessments. Only the owner can change membership or delete students. Deleting a student requires confirmation and removes that student's assessments across every period in that classroom. Renaming a student applies across all periods.

## The mathematics form

The PDF at the repository root is embedded in the Rust executable, together with `backend/criteria.json`. The checked-in catalog contains **354 criteria** across **10 sections**, on pages **5–16**, retaining the source wording and levels Basis–10.

Each criterion includes an ID, domain, section, level, symbol, source background band (`white`, `light`, or `dark`), and a mark location. Gray target stages follow the PDF (03, 05, 07, 09); their backgrounds are extracted from the actual PDF drawings. Coordinates use **PDF points from the top-left corner**, pages are **1-based**, and the exporter converts the y coordinate to the PDF bottom-left coordinate system. Crosses are drawn in the gap between the strand symbol and statement. The `math_criteria` SQLite table also stores this mapping. The source SHA-256 is recorded in the catalog and checked by a Rust test. The original template is never overwritten.

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

For deployment, use HTTPS, secure cookies, a private terminal/log for initial setup, and persistent storage for the SQLite database. Back up SQLite using its online backup API or stop the server before copying the database and its WAL files together. Password reset, email delivery, moving students between classes, and live synchronization between browsers are outside this first version. Concurrent edits to the same assessment use the last saved value; reloading a classroom retrieves other teachers' changes.

## Checks

```sh
pnpm --dir frontend check
pnpm --dir frontend build
cargo fmt --all --check
cargo test
```

The Rust suite exercises the real Axum router without requiring a network listener: invitation issuance/revocation/expiry and concurrent redemption, admin permissions, cookies and authentication, classroom isolation and sharing revocation, tri-state grading, atomic bulk updates, independent period copies, student management, and PDF/period ZIP export, including duplicate names, safe filenames, unassessed students, selected-period marks, and sharing revocation. It also verifies HTML and asset serving, frontend route fallback, HEAD requests, and separate 404 behavior for API routes and assets. The PDF tests check the template hash, mapping, page count, and the pages receiving overlays.

The workflow test writes `output/pdf/qa-all-criteria.pdf`, a synthetic fully marked export for visual review. `scripts/smoke-test.py` additionally exercises a real HTTP server, restart persistence, and PDF validation when local sockets and Poppler's `pdfinfo` are available. It uses a disposable database and cleans up its server.

After building the image, run `python3 scripts/container-smoke-test.py tamara:local` to verify the compiled frontend, HTTP API, non-root runtime, graceful shutdown, and account/session/classroom persistence across container replacement. This uses and removes an isolated test container and volume. CI builds the Docker image and runs this check on every push.
