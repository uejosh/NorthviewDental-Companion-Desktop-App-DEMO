# Northview Office App

Front-desk desktop companion for the Northview Dental website (built from [docs/DEMO-DESCRIPTION.md](docs/DEMO-DESCRIPTION.md)
and the designs in `~/Downloads/NorthView Dental Destop App Plan/Designed/`). Native Rust + egui — no browser engine, no Node.js.

It has two modes, chosen from the link under the PIN box on the sign-in screen:

| Mode | Data | Sign-in |
| --- | --- | --- |
| **Website (live)** | Pulled from the WordPress site through the *Northview Office API* plugin; every action is written back and audited on the server. Nothing patient-related is saved on the PC. | Staff accounts and PINs managed on the website |
| **Demo data** | Fictional sample patients saved in `%LOCALAPPDATA%\NorthviewOffice` (`.bak` kept, damaged files never overwritten). *Simulate a website request* and *Reset demo* are available. | Any 4–6 digit PIN |

## Screens

* **Booking requests** — WPForms form 55 (*Contact & Appointment Request*) entries. Emergencies pinned, filters (Open/New/Contacted/Scheduled/Declined/All), search, Copy for Eaglesoft, Mark contacted / scheduled (with appointment date) / Decline, Reopen, activity log, avg first response. New requests trigger a desktop notification (no patient details) when the app is in the background.
* **Patient forms** — upcoming new patients (from scheduled requests with a date), form received/missing, Send form link, printable form view, Save as PDF (Typst, Letter), Mark attached in Eaglesoft.
* **Smile Savings Plan** — members table, renewals within 30 days highlighted, Renew / Send reminder / Mark paid, Add member with automatic pricing ($500 / $925 / $1,350 / $1,500, +$450 per extra member).
* **Review requests** — visits from the last 7 days (scheduled requests become visits automatically; *Log a visit* adds others), Text/Email, 180-day cooldown, message preview with opt-out line, sent log.
* Global search (Ctrl K), inbox search (Ctrl F), lock (Ctrl L or the sidebar button), auto-lock after 5 minutes.
* Motion: screens and the sign-in/lock screen fade and rise in (about ¼ s), the sidebar and filter highlights slide, list selection and hovers ease, request/form details cross-fade and confirmations slide up.

## Website side: `server/northview-office-api`

A WordPress plugin (installed and active on `nvdental-elwp.s-turl.com`) exposing `/wp-json/nvoffice/v1/…`:

`GET /staff` · `POST|DELETE /session` · `GET /sync` · `POST /requests/{id}/status|viewed|copied` ·
`POST /forms/{id}/attached|pdf` · `POST /forms/link` · `POST /members` · `POST /members/{id}/paid|renew|remind|archive` ·
`POST /visits` · `POST /reviews`

Security model:

* Every call needs the **device key** (`X-NVO-Device`), entered once per office PC, so the PIN endpoint is not open to the internet.
* Staff PINs are stored hashed (`wp_hash_password`); 5 wrong PINs lock that person out for 15 minutes.
* Sessions are random bearer tokens stored hashed; 30 minutes idle / 12 hours maximum. Locking the app ends the session and clears patient data from memory.
* Every view and change goes into the `wp_nvo_audit` table with the staff member, IP and time.

Setup on the website — **Tools → Northview Office App**:

1. Tick the staff who may use the app, give each a display name, role label and a 4–6 digit PIN. Tick *Manager* for the office manager.
2. Copy the *Site address* and *Device key* into the app on each office PC (Connection settings on the sign-in screen).
3. Optional: Google review link, Google profile URL, rating, and the new-patient form (see below).

To update the plugin: `server/northview-office-api.zip` → *Plugins → Add New → Upload*.

### Not switched on yet (needs practice decisions)

* **New-patient form**: the site deliberately removed DOB/insurance from the booking form and has no medical-history form. Collecting medical history online should wait for hosting covered by a HIPAA BAA. When ready, build the form in WPForms and choose it on the settings page; the app maps fields by label (name, date of birth, phone, email, address, first visit, conditions, medications, allergies, last dental visit, carrier, member ID, signature).
* **Texts and emails** (form links, renewal reminders, review requests) are recorded and audited but *simulated* until an SMS/email provider that signs a BAA is connected (`messaging_live`).
* **Review link / Google profile URL** are blank until entered on the settings page.

## Build, test, package

```sh
cargo run --release              # run
cargo test                       # business rules, website JSON contract, storage recovery, PDF
cargo run -- --screenshot shot.png --screen requests   # signin|requests|forms|savings|reviews (demo data)
python3 package.py               # release build + dist/ archive (Windows: ZIP with per-user installer)
```

Other options: `--data-dir <folder>` isolates settings/demo data, `--demo` forces demo mode,
`--no-motion` turns off transitions (screen fades, sliding highlights, toast slide; see `src/motion.rs`).

**GitHub Actions** (`.github/workflows/windows.yml`) builds the Windows ZIP on every push to `main`
(download it from the run's *Artifacts*), and publishes a GitHub Release when you push a tag:
`git tag v0.1.0 && git push origin v0.1.0`. It can also be started by hand from the Actions tab.

Locally, Windows builds must be made on Windows (`python3 package.py`) or with a MinGW cross toolchain
(`python3 package.py --target x86_64-pc-windows-gnu`). Release builds hide the console window.

## Layout

```
src/main.rs        window + command-line options
src/model.rs       data types shared with the website + business rules
src/sample.rs      fictional demo data
src/storage.rs     config + demo file (atomic save, .bak, damaged-file recovery)
src/api.rs         website client on a background thread
src/pdf.rs         Typst → Letter PDF (assets/form.typ)
src/notify.rs      background desktop notifications
src/theme.rs       brand palette, Poppins fonts, custom widgets
src/motion.rs      transition timings and helpers
build.rs           embeds assets/icon.ico + version info into the Windows .exe
packaging/linux/   install.sh / uninstall.sh (applications-menu launcher + icon)
src/ui/            sign-in, shell, inbox, forms, savings, reviews, dialogs
server/            WordPress plugin source + zip
```
