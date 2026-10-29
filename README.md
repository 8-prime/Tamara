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

## Class workspaces and navigation

Choose the class once in the sidebar's **Klasse** switcher, then use the single work-area menu: **Lernentwicklung**, **Anwesenheit**, **Hausaufgaben**, **Notizen**, **Kinder**, and **Klasseneinstellungen**. Switching classes keeps the selected work area. Class creation is beside the switcher; account and administrator tools remain separate from class tools.

Work areas have URLs such as `/classes/{class-id}/homework?tab=open`. Refresh, direct links, and browser Back/Forward restore the class and work area. Homework's tab and subject filter are also reflected in its URL. The **Notizen** entry currently explains the planned quick per-pupil notes feature; note-taking will be implemented separately.

Manage the class roster under **Kinder**, and its subjects and sharing under **Klasseneinstellungen**. View modes, dates, grading periods, homework tabs, and subject filters remain within their respective pages.

## Teaching workflow

1. Create a classroom with a name and school year.
2. Add students, one name per line. Student IDs keep children with identical names distinct.
3. Create a named grading period. Start empty or copy any period in the same classroom. Copying takes an independent snapshot of both affirmative and negative assessments; unassessed entries stay unassessed.
4. Edit in **Einzelkind**, **Klassenübersicht**, or **Nach Kompetenz**. All three modes update the same records and save automatically. In the competency view, select students and apply a status to the selection, or grade individual students directly. Unselected students are unchanged.
5. Filter by competency section, school grade, development level, or search text. The grade filter follows the overlapping ranges on PDF page 3: grade 1 = stages 01–04, grade 2 = 03–06, grade 3 = 05–08, grade 4 = 07–10. The exact-stage filter narrows within that range; choose all grades to include Basis. Changing topics preserves the selected grade. Topics are grouped by domain in the dropdown. All three editing views retain the PDF’s blue topic hierarchy, competency symbols, source order, and light/dark gray target-stage bands. The classroom matrix paginates competencies and scrolls horizontally.
6. Export a student's PDF from the individual view, or choose **Alle PDFs als ZIP** beside the period selector to export the whole class for that period. The ZIP includes one PDF for every child, including children without assessments. Numbered, sanitized filenames keep duplicate names distinct. The roster and assessments are read together as a snapshot. The original 20-page document is retained, affirmative assessments get crosses beside their statements, and the cover receives the student name, school year, and grading-period name.

Each **Erreicht** assessment stores an `achieved_on` observation date independently of its last edit time. It defaults to the teacher’s local date in the UI (UTC date for API callers that omit it), appears in all three editing views, and can be adjusted for earlier observations. Saving an already achieved competency preserves its date; copying a period preserves dates as an independent snapshot. Switching to **Noch nicht** or **Nicht beurteilt** clears the reached date in that period. Existing affirmative assessments are backfilled from their last saved timestamp, the only historical date available.

Statuses are **Erreicht** (`yes`), **Noch nicht** (`no`), and **Nicht beurteilt** (no assessment row). The latter two remain distinct in SQLite and both leave the printed form blank. The reference form provides no separate mark for a negative or unassessed result.

The classroom creator is its owner. Under **Klasseneinstellungen → Freigaben verwalten**, they can share with another registered teacher by email or revoke access. Collaborators can edit students, periods, and assessments. Only the owner can change membership or delete students. Deleting a student requires confirmation and removes that student's assessments across every period in that classroom, attendance, and homework submission records. Renaming a student applies across all periods and features.

## Attendance

Select a class, then open **Anwesenheit**. Attendance is independent of mathematics grading periods and defaults to the teacher's local date.

- **Kind für Kind** shows each pupil with **Anwesend** and **Abwesend** buttons. Each choice saves before advancing. Teachers can skip a pupil, go back to correct a choice, and resume unrecorded pupils. The summary distinguishes present, absent, and still open.
- **Kalender & Liste** provides a month calendar and date input for historical entries and corrections. Each pupil has present and absent checkboxes. Selecting one replaces the other; removing the selected checkmark clears the entry. Changes save automatically. Both views use the same daily records.

Attendance stores one record per class, date, and student ID, including who last saved it and when. Unrecorded pupils are distinct from absent pupils. Duplicate names remain separate. Class owners and shared editors can read and edit attendance; access revocation applies immediately to subsequent requests. Renaming a pupil preserves records; deleting one removes their attendance across all dates along with their assessments. Concurrent edits use the last saved value; reload to retrieve another teacher's changes.

The API supports `GET /api/classrooms/{class}/attendance/{YYYY-MM-DD}` and `PATCH` at the same path with `{"changes":[{"student_id":"…","status":"present"}]}`. Status is `present`, `absent`, or `null` to clear. Dates are validated calendar dates; invalid batches are rejected atomically.

## Homework

1. Add the class's subjects under **Klasseneinstellungen → Fächer**. Subjects can be renamed; a subject used by any homework cannot be removed, preserving the history.
2. Open **Hausaufgaben** and choose **Hausaufgabe erstellen**. Select a subject, enter a title and due date, and optionally add a description and a PDF worksheet (up to 10 MB). Existing entries can be edited, and worksheets can be added, replaced, downloaded, or removed.
3. **Offen** shows active homework across all subjects, ordered by due date. Filter to a single subject as needed. Past-due homework stays open and is labelled **Überfällig**.
4. Open an assignment and check each pupil who has handed it in, or mark all remaining pupils at once. Changes save automatically. Duplicate names remain separate through pupil IDs.
5. The final submission automatically closes the assignment and moves it into **Verlauf**. **Manuell abschließen** also closes an assignment with outstanding submissions; those remain visible in its history. **Wieder öffnen** reopens a manually closed assignment with outstanding submissions.

The assigned pupil list is a snapshot of the roster when the homework is created. Adding pupils later does not change existing assignments. Renaming a pupil preserves the assignment; deleting one removes their submission and recalculates automatic completion for the remaining assigned pupils. Homework requires at least one pupil at creation. An assignment without remaining pupils can still be closed manually.

History retains assignment details, due dates, worksheets, and per-pupil submission dates. Correcting a submission in an automatically closed assignment reopens it if a pupil is now outstanding. Manual closures remain closed when submissions are corrected.

Class owners and shared editors can manage subjects, homework, worksheets, and submissions. Access checks also protect PDF downloads, and revoked sharing applies to subsequent requests. Submission batches and automatic closure are saved in one write transaction. Worksheets are private SQLite blobs rather than public files, so existing database backups include them.

The homework API is independent of mathematics grading periods:

- `GET/POST /api/classrooms/{class}/subjects`; `PATCH/DELETE /subjects/{subject}` under the same class.
- `GET/POST /api/classrooms/{class}/homework`; `GET/PATCH /homework/{homework}` under the same class. Creation/editing accepts `subject_id`, `title`, `description`, and `due_date` (`YYYY-MM-DD`).
- `PATCH …/homework/{homework}/submissions` accepts `{"changes":[{"student_id":"…","handed_in":true}]}`.
- `PATCH …/homework/{homework}/closure` accepts `{"closed":true}` (manual close) or `false` (reopen when submissions remain outstanding).
- `GET/PUT/DELETE …/homework/{homework}/worksheet`; upload raw PDF bytes with `PUT …/worksheet?name=worksheet.pdf`. The 10 MB body limit applies only to worksheet routes; other API requests retain their 1 MB limit.

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

For deployment, use HTTPS, secure cookies, a private terminal/log for initial setup, and persistent storage for the SQLite database. Password reset, email delivery, moving students between classes, and live synchronization between browsers are outside this first version. Concurrent edits to the same assessment use the last saved value; reloading a classroom retrieves other teachers' changes.

## Database backups

Administrators can open **Administration → Datenbank sichern** and select **Sicherung erstellen**. The server creates a ZIP containing a standalone `database.sqlite` snapshot of the entire database, including committed WAL data, using SQLite's `VACUUM INTO`. The running application can continue to be used during the snapshot. The completed archive is published atomically; errors are reported in the dialog and server logs.

Archives live in a private directory beside the database: for `data/tamara.sqlite`, this is `data/tamara.sqlite.backups/`, with separate `manual/` and `automatic/` subdirectories. On Unix these directories are created with mode `0700`. This location is outside the frontend's served directory and remains in the persistent data volume in Docker. `POST /api/admin/backups` requires an administrator session and the mutation header, returns `204` after completion, and exposes no filenames or paths. There is no backup listing or download API.

The backend creates one automatic backup per night at **02:00 UTC** while running, checking once a minute. On startup it catches up the latest missed nightly backup, rather than generating archives for every missed day. Automatic archives older than seven days are removed after a successful backup; at most seven are retained. Stable nightly filenames avoid duplicates on restart. Failures are logged and retried after a minute. Manual archives are never removed by automatic retention. Shutdown waits for an in-progress nightly backup to finish. Importing/restoring through the application is not implemented.

## Checks

```sh
pnpm --dir frontend check
pnpm --dir frontend build
cargo fmt --all --check
cargo test
```

The Rust suite exercises the real Axum router without requiring a network listener: invitation issuance/revocation/expiry and concurrent redemption, admin permissions, cookies and authentication, classroom isolation and sharing revocation, tri-state grading, atomic bulk updates, independent period copies, student management, and PDF/period ZIP export, including duplicate names, safe filenames, unassessed students, selected-period marks, and sharing revocation. Homework coverage includes subject validation, roster snapshots, duplicate names, atomic submission batches, concurrent final submissions, automatic/manual closure and reopening, historical corrections, pupil deletion, PDF upload limits/replacement/download/removal, and sharing revocation. It also verifies HTML and asset serving, frontend route fallback, HEAD requests, and separate 404 behavior for API routes and assets. The PDF tests check the template hash, mapping, page count, and the pages receiving overlays.

The workflow test writes `output/pdf/qa-all-criteria.pdf`, a synthetic fully marked export for visual review. `scripts/smoke-test.py` additionally exercises a real HTTP server, restart persistence, and PDF validation when local sockets and Poppler's `pdfinfo` are available. It uses a disposable database and cleans up its server.

After building the image, run `python3 scripts/container-smoke-test.py tamara:local` to verify the compiled frontend, HTTP API, non-root runtime, graceful shutdown, and account/session/classroom persistence across container replacement. This uses and removes an isolated test container and volume. CI builds the Docker image and runs this check on every push.
