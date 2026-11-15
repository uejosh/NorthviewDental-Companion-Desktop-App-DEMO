# Northview Dental Office App — Demo Description

**Version:** 0.1.0 (prototype) · **Prepared by:** Sturl Digital Services · **Platform:** Windows desktop 

> This is a working prototype built to show Northview Dental how the phase 1 Office App will look and behave. It runs entirely on sample data. No real patient information is used, and it does not yet connect to the website, a server or Eaglesoft.

---

## 1. What the app is

The Office App is a desktop companion for the Northview Dental front desk. It handles the jobs that fall **between** the new website and Eaglesoft:

- appointment requests that arrive from the website
- new-patient paperwork completed online
- the in-house Smile Savings Plan
- asking happy patients for Google reviews

It does **not** replace Eaglesoft. Scheduling, charting, imaging, billing and insurance claims stay in Eaglesoft exactly as today. The app's job is to make sure nothing from the website gets lost, and to make it quick to carry information across into Eaglesoft.

### Who uses it

| Role | Typical use |
| --- | --- |
| Front desk | Works the booking inbox, chases missing forms, sends review requests |
| Office manager | Same as front desk, plus oversight of the Savings Plan and response times |

---

## 2. Signing in and staying secure

![Sign-in screen](Designed/screens/01-sign-in.png)

- Each staff member signs in with their name and a PIN. In the demo, either user works with any 4–6 digit PIN.
- The app **locks automatically after 5 minutes without activity** and returns to the sign-in screen. This matters at a front desk where the screen may face patients.
- A **Lock** button in the sidebar lets staff lock the app immediately when stepping away.
- Every action is recorded against the signed-in user's name. This is the basis of the audit trail required for HIPAA.

---

## 3. Booking requests

![Booking requests inbox](Designed/screens/02-booking-requests.png)

The booking inbox collects every appointment request submitted through the website's **Request an appointment** form.

### At a glance

Four summary tiles across the top show:

- **New**: requests nobody has acted on yet
- **Awaiting reply**: patients who were contacted but haven't been booked yet
- **Scheduled**: requests that have been booked in Eaglesoft
- **Avg. first response**: the average time between a request arriving and the first action on it, a simple measure of front-desk responsiveness

The sidebar shows a count badge for new requests, so staff can see work waiting from any screen.

### The list

- Requests are listed newest first.
- **Emergency requests are always pinned to the top** and tagged in red, regardless of when they arrived.
- Each card shows the patient's name, how long ago they asked, the status, whether they are a new or returning patient, and the reason for the visit.
- **Filters:** Open (new + contacted, the default), New, Contacted, Scheduled, Declined, All.
- **Search** by name, phone number or reason. Results update as you type.

![Searching the inbox](Designed/screens/02c-inbox-search.png)

### The request detail

Selecting a request opens the full details on the right:

- reason for visit, new or returning patient, phone (clickable to call), email, date of birth with calculated age, preferred days and times, insurance carrier and the patient's message
- a red **"Emergency request. Call this patient first."** banner for emergencies

### Actions

| Button | What it does |
| --- | --- |
| **Copy details for Eaglesoft** | Copies the patient's details to the clipboard in a tidy block, ready to paste when creating the patient or appointment in Eaglesoft |
| **Mark contacted** | Records that the patient was called or messaged (for example, left a voicemail) |
| **Mark scheduled** | Records that the appointment is booked in Eaglesoft and closes the request |
| **Decline** | Closes a request that won't be booked (duplicate, spam, outside services offered) |

### Activity log

Every status change is listed under **Activity** with the action, the staff member who took it and the time. The original website submission is always the first entry.

### Demo feature: simulate a website request

![A new request arriving in the inbox](Designed/screens/02b-inbox-new-request.png)

**Simulate a website request** creates a new incoming request as if a patient had just submitted the website form. The request arrives about 5 seconds after the click and appears at the top of the inbox. If you switch to another window during that time, it also shows a **Windows desktop notification**, just as a real request would. Every few clicks it simulates an emergency, to show emergency pinning. In the live app this button does not exist; requests arrive from the website automatically.

---

## 4. Patient forms

![Patient forms](Designed/screens/03-patient-forms.png)

This module replaces the downloadable PDF form on the current website. Patients complete their new-patient form online before their visit, or on an office tablet on arrival, and sign on screen.

### Upcoming new patients

A checklist at the top lists new patients with upcoming visits:

- 🟢 green dot = form received
- 🟠 amber dot = form missing, with a **Send form link** button that texts and emails the patient a link to complete it (simulated in the demo)

This lets the front desk chase missing paperwork the day before rather than at check-in.

### Received forms

- Each completed form shows the patient, when it was submitted and the visit date.
- Status is either **To attach** (still needs to go into Eaglesoft) or **In Eaglesoft**.
- The sidebar badge counts forms still waiting to be attached.

### The form view

The selected form is laid out as a clean, printable document with the practice name, address and phone at the top:

- **Patient:** name, date of birth, phone, email, address, first visit date
- **Medical history:** conditions, medications, allergies (allergies are **highlighted in red** when present) and last dental visit
- **Insurance:** carrier and member ID
- **Signature:** the patient's name with the time it was signed electronically

### Actions

| Button | What it does |
| --- | --- |
| **Save as PDF** | Saves the form as a Letter-size PDF (named like `Bell-Marcus-new-patient-form.pdf`) ready to attach in Eaglesoft's document centre. The PDF is generated directly by the app and contains only the form, not the app's menus. |
| **Mark attached in Eaglesoft** | Records that the PDF has been added to the patient's record, and clears it from the to-do count |

---

## 5. Smile Savings Plan

![Smile Savings Plan](Designed/screens/04-savings-plan.png)

A register of members of the practice's in-house membership plan for patients without dental insurance. Each membership runs **one year from the enrolment date**.

### At a glance

- **Memberships**: total active memberships
- **People covered**: everyone included across all memberships
- **Renewing within 30 days**: memberships that need attention soon
- **Payment due**: memberships with an outstanding payment
- **Annual plan value**: the total yearly value of all memberships

### Members table

Sorted so the **soonest renewals come first**. Each row shows:

- membership name and the household members it covers
- plan and yearly price
- number of people
- enrolment date
- renewal date with a countdown ("in 13 days"); rows due within 30 days are **highlighted** with the countdown in amber
- payment status (Paid or Due)

### Actions

| Button | When it appears | What it does |
| --- | --- | --- |
| **Add member** | Always | Opens the new-membership form: name, plan, enrolment date and household members (one per line) |
| **Mark paid** | Payment is due | Records the payment |
| **Renew** | Within 30 days of renewal | Extends the membership by one year and marks the new year's payment as due |
| **Send reminder** | Within 30 days of renewal | Sends the member a renewal reminder (simulated in the demo) |

### Pricing

Prices match the plan published on the new website:

| Plan | Price per year |
| --- | --- |
| Single | $500 |
| Dual | $925 |
| Family of 3 | $1,350 |
| Family of 4 | $1,500 |
| Each additional member | $450 |

The app works out each membership's price automatically, including extra members beyond the plan size.

---

## 6. Google review requests

![Review requests](Designed/screens/05-review-requests.png)

Helps the practice keep a steady flow of new Google reviews, which protects its 5.0 rating and feeds the reviews section on the new website.

### How it works

- Recent visits are listed with each patient's phone, email, visit date and when they were last asked for a review.
- Staff send a request in one click by **Text** or **Email**.
- **Patients asked in the last 180 days are skipped automatically.** The buttons are replaced with "Asked recently", so nobody is pestered.
- A **message preview** shows exactly what patients receive, including the review link and an opt-out line ("Reply STOP to opt out").
- A **Sent** log records each request with the patient, channel, staff member and time.
- Summary tiles show requests sent and the current Google rating.

---

## 7. Notifications, badges and feedback

- **Desktop notifications** for new booking requests when the app is in the background or minimised. They say only "New appointment request" or "Emergency appointment request", with no patient details, because Windows can show them on the lock screen. While staff are working in the app, the sidebar badge and an on-screen message are used instead.
- **Sidebar badges** show what needs attention: new requests, forms to attach and memberships renewing soon.
- **Confirmation messages** appear briefly at the bottom of the screen after every action (for example "Copied. Paste into Eaglesoft." or "Membership added").

---

## 8. Suggested demo walkthrough (about 10 minutes)

1. **Sign in** as Front Desk. Point out the auto-lock.
2. **Booking requests:** open Tamara Ellison's emergency at the top. Show the red banner and the details. Click **Copy details for Eaglesoft**, then **Mark contacted**, and show the activity log.
3. Click **Simulate a website request**, then switch to another window (or minimise the app) and wait for the desktop notification. Return to the app and mark the new request **scheduled**.
4. **Patient forms:** show the upcoming new patient with a missing form and click **Send form link**. Open Marcus Bell's form, point out the highlighted allergy, then **Save as PDF** and **Mark attached in Eaglesoft**.
5. **Savings plan:** show the two memberships renewing soon. **Renew** one and **Send reminder** to the other. Add a new family membership.
6. **Review requests:** send a request by text and show that the patient now reads "Asked recently".
7. Click **Lock** to finish, and explain how real logins and the audit log will work.
8. Click **Reset demo** before the next run to restore the sample data.

---

## 9. Under the hood

| Area | Prototype |
| --- | --- |
| Language | Rust (2021 edition), compiled to a single native executable |
| Framework | `eframe`/`egui` 0.31, a native immediate-mode UI drawn with OpenGL (glow). No web browser engine, no Node.js and no embedded web runtime. Built for Windows; the same code also builds on macOS and Linux |
| Interface | Custom widgets and colour palette in `theme.rs`, using the practice's brand colours and bundled fonts. The Northview logo and sign-in background are embedded in the executable |
| Data | Fictional sample data seeded on first launch. Changes are saved straight away to a versioned data file in the user's app-data folder (`%LOCALAPPDATA%\NorthviewOffice` on Windows), and the previous copy is kept as a `.bak`. If the file is damaged, the app keeps it and offers to restore the backup rather than overwriting it. **Reset demo** reseeds the sample file |
| Business rules | Plan pricing, renewal dates, review-request cooldown, emergency pinning and response-time averages live in `model.rs`, kept separate from the UI and covered by unit tests |
| PDF export | Typst, embedded as a library (`typst` + `typst-pdf`, through `typst-as-lib`). The form layout lives in one template, `form.typ`, which the app fills with the patient's details. Typst handles text wrapping, page breaks, Letter page size, the logo and the red allergy highlight. The template and fonts are built into the executable, so no print dialog, browser or extra files are needed |
| Notifications | Windows desktop notifications via `notify-rust`, sent from one background thread so the screen never stalls. They appear only when the app is in the background; while staff are using it, the in-app confirmation message and sidebar badge are used instead. Notifications contain **no patient details** ("New appointment request" or "Emergency appointment request"), because Windows can show them on the lock screen and keeps them in the notification centre. The installer registers the app's own notification ID, so notifications show the Northview name and icon. Clicking a notification does not open the request yet (see section 10) |
| Clipboard | **Copy details for Eaglesoft** uses egui's built-in clipboard support |
| Security | No web content is rendered, so there is no script injection surface. Auto-lock is driven by the app's own input tracking. Phone numbers, email addresses and review links open in the system's default apps. Release builds hide the console window (`windows_subsystem = "windows"`) |
| Testing | `cargo test` runs the business-rule tests and headless egui click/render tests. `--screenshot <file.png>` renders the app to an image and exits; `--data-dir <folder>` isolates test data |
| Packaging | `cargo build --release` (LTO, stripped), then `python3 package.py` produces a Windows ZIP with a per-user PowerShell installer. The installer creates a desktop shortcut and registers the notification ID (`SturlDigital.NorthviewOffice`) under the current user, so no administrator account is needed |

### Project layout

```
nvdental-office-app/
├── Cargo.toml          Dependencies and release profile
├── package.py          Builds the Windows ZIP and per-user installer
├── src/
│   ├── main.rs         Window setup, command-line options (--screenshot, --data-dir)
│   ├── ui.rs           Sign-in, sidebar, auto-lock and all four screens
│   ├── theme.rs        Brand colours, fonts, buttons, badges and tiles
│   ├── model.rs        Requests, forms, memberships, reviews and their rules
│   ├── sample.rs       Fictional sample data, plan prices and placeholder review link
│   ├── storage.rs      Versioned save file, .bak backups and recovery
│   ├── pdf.rs          Fills form.typ and compiles it to a Letter-size PDF
│   ├── notify.rs       Background notification worker
│   └── tests.rs        Rule tests and headless UI tests
├── assets/
│   ├── form.typ        New-patient form layout (Typst template)
│   ├── fonts/
│   ├── NorthViewDental-Logo-R.png
│   └── Sign-in_screen_bg.jpg
└── docs/
    ├── DEMO-DESCRIPTION.md
    └── screenshots/
```

### Build and run

```sh
cargo run --release
cargo test
cargo build --release
python3 package.py
```

---

## 10. What the demo does not do yet

| Demo behaviour | In the production app |
| --- | --- |
| Sample patients stored on the computer | Real data held on a secure server (hosting covered by a HIPAA Business Associate Agreement), with nothing sensitive left on the PC |
| Any PIN signs in | Individual staff accounts and roles, checked by the server |
| Activity shown per request | Full audit log of who viewed or changed every record |
| "Simulate a website request" button | Requests arrive automatically from the website's booking form |
| Texts and emails are simulated | Sent through an SMS/email provider that signs a BAA |
| Clicking a desktop notification does nothing | Clicking it brings the app forward and opens the new request (planned using `tauri-winrt-notification`, the library `notify-rust` uses on Windows) |
| Placeholder Google review link | The practice's real review link |
| No connection to Eaglesoft | Same in phase 1: staff copy details across. Read-only Eaglesoft features may come in a later phase, subject to Patterson's approval |

---

*Prototype prepared by Sturl Digital Services for Northview Dental, Indianapolis. All patient names and details shown are fictional.*
