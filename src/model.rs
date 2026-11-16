//! Data types shared with the website API, plus the business rules
//! (plan pricing, renewals, review cooldown, emergency pinning, response times).
//! Nothing in here touches the UI, the network or the disk.

use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub const REVIEW_COOLDOWN_DAYS: i64 = 180;
pub const RENEWAL_WINDOW_DAYS: i64 = 30;
pub const AUTO_LOCK_SECS: u64 = 5 * 60;
pub const EXTRA_MEMBER_PRICE: u32 = 450;

/// (key, label, people included, price per year)
pub const PLANS: [(&str, &str, usize, u32); 4] = [
    ("single", "Single", 1, 500),
    ("dual", "Dual", 2, 925),
    ("family3", "Family of 3", 3, 1350),
    ("family4", "Family of 4", 4, 1500),
];

pub fn plan_label(key: &str) -> &'static str {
    PLANS.iter().find(|p| p.0 == key).map(|p| p.1).unwrap_or("Single")
}

/// Number of people a plan covers before the extra-member charge.
pub fn plan_size(key: &str) -> usize {
    PLANS.iter().find(|p| p.0 == key).map(|p| p.2).unwrap_or(1)
}

/// Yearly price including any members beyond the plan size.
pub fn plan_price(key: &str, people: usize) -> u32 {
    let (_, _, size, price) = PLANS.iter().copied().find(|p| p.0 == key).unwrap_or(PLANS[0]);
    price + people.saturating_sub(size) as u32 * EXTRA_MEMBER_PRICE
}

/// Smallest plan that fits a household (used to suggest a plan in the add-member form).
pub fn suggested_plan(people: usize) -> &'static str {
    match people {
        0 | 1 => "single",
        2 => "dual",
        3 => "family3",
        _ => "family4",
    }
}

// ---------------------------------------------------------------- requests

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    New,
    Contacted,
    Scheduled,
    Declined,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::New => "New",
            Status::Contacted => "Contacted",
            Status::Scheduled => "Scheduled",
            Status::Declined => "Declined",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Status::New => "new",
            Status::Contacted => "contacted",
            Status::Scheduled => "scheduled",
            Status::Declined => "declined",
        }
    }
    pub fn is_open(self) -> bool {
        matches!(self, Status::New | Status::Contacted)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Activity {
    pub action: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub by: String,
    pub at: DateTime<Utc>,
}

impl Activity {
    pub fn title(&self) -> String {
        let a = match self.action.as_str() {
            "viewed" => return format!("Viewed by {}", self.by),
            "copied" => "Details copied for Eaglesoft",
            "status_contacted" => "Marked contacted",
            "status_scheduled" => "Marked scheduled",
            "status_declined" => "Declined",
            "status_new" => "Reopened",
            "form_link_sent" => "Form link sent",
            "received" => "Request received from website",
            other => other,
        };
        if self.by.is_empty() {
            a.to_string()
        } else {
            format!("{a} by {}", self.by)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Request {
    pub id: u64,
    #[serde(rename = "ref", default)]
    pub reference: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub dob: String,
    #[serde(default)]
    pub insurance: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub new_patient: bool,
    #[serde(default)]
    pub emergency: bool,
    #[serde(default)]
    pub preferred: Vec<String>,
    #[serde(default)]
    pub message: String,
    pub received_at: DateTime<Utc>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub first_action_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub appt_date: Option<NaiveDate>,
    #[serde(default)]
    pub activity: Vec<Activity>,
}

impl Request {
    pub fn name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim()).trim().to_string()
    }
    pub fn initials(&self) -> String {
        initials(&self.name())
    }
    pub fn patient_kind(&self) -> &'static str {
        if self.new_patient { "New patient" } else { "Returning" }
    }
    /// Emergencies that still need action are pinned above everything else.
    pub fn pinned(&self) -> bool {
        self.emergency && self.status.is_open()
    }
    pub fn matches(&self, q: &str) -> bool {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let digits: String = q.chars().filter(|c| c.is_ascii_digit()).collect();
        self.name().to_lowercase().contains(&q)
            || self.reason.to_lowercase().contains(&q)
            || self.email.to_lowercase().contains(&q)
            || (digits.len() >= 3 && digits_of(&self.phone).contains(&digits))
    }

    /// Text block the front desk pastes into Eaglesoft.
    pub fn eaglesoft_text(&self, today: NaiveDate) -> String {
        let mut s = String::new();
        let mut line = |k: &str, v: &str| {
            if !v.trim().is_empty() {
                s.push_str(&format!("{k:<13}{}\n", v.trim()));
            }
        };
        line("Name:", &self.name());
        line("Patient:", self.patient_kind());
        line("Phone:", &self.phone);
        line("Email:", &self.email);
        line("Address:", &self.address);
        let dob = match parse_date(&self.dob) {
            Some(d) => format!("{} (age {})", d.format("%m/%d/%Y"), age(d, today)),
            None => self.dob.clone(),
        };
        line("DOB:", &dob);
        line("Insurance:", &self.insurance);
        line("Reason:", &self.reason);
        line("Preferred:", &self.preferred.join(", "));
        if let Some(d) = self.appt_date {
            line("Appointment:", &d.format("%a %m/%d/%Y").to_string());
        }
        line("Message:", &self.message);
        line("Website ref:", &self.reference);
        s
    }

    /// Newest-first activity with the original website submission always last (the first event).
    pub fn timeline(&self) -> Vec<Activity> {
        let mut v: Vec<Activity> = self.activity.iter().filter(|a| a.action != "received").cloned().collect();
        v.sort_by(|a, b| b.at.cmp(&a.at));
        v.push(Activity { action: "received".into(), detail: String::new(), by: String::new(), at: self.received_at });
        v
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    Open,
    New,
    Contacted,
    Scheduled,
    Declined,
    All,
}

impl Filter {
    pub const ALL: [Filter; 6] = [Filter::Open, Filter::New, Filter::Contacted, Filter::Scheduled, Filter::Declined, Filter::All];
    pub fn label(self) -> &'static str {
        match self {
            Filter::Open => "Open",
            Filter::New => "New",
            Filter::Contacted => "Contacted",
            Filter::Scheduled => "Scheduled",
            Filter::Declined => "Declined",
            Filter::All => "All",
        }
    }
    pub fn accepts(self, s: Status) -> bool {
        match self {
            Filter::Open => s.is_open(),
            Filter::New => s == Status::New,
            Filter::Contacted => s == Status::Contacted,
            Filter::Scheduled => s == Status::Scheduled,
            Filter::Declined => s == Status::Declined,
            Filter::All => true,
        }
    }
}

/// Filtered, searched and ordered list: (pinned emergencies, the rest newest first).
pub fn inbox<'a>(reqs: &'a [Request], f: Filter, q: &str) -> (Vec<&'a Request>, Vec<&'a Request>) {
    let mut pinned = Vec::new();
    let mut rest = Vec::new();
    for r in reqs.iter().filter(|r| f.accepts(r.status) && r.matches(q)) {
        if r.pinned() { pinned.push(r) } else { rest.push(r) }
    }
    pinned.sort_by(|a, b| b.received_at.cmp(&a.received_at));
    rest.sort_by(|a, b| b.received_at.cmp(&a.received_at));
    (pinned, rest)
}

pub fn count_status(reqs: &[Request], f: Filter) -> usize {
    reqs.iter().filter(|r| f.accepts(r.status)).count()
}

/// Average time from arrival to first action over the last 30 days.
pub fn avg_first_response(reqs: &[Request], now: DateTime<Utc>) -> Option<Duration> {
    let window = now - Duration::days(30);
    let v: Vec<i64> = reqs
        .iter()
        .filter_map(|r| r.first_action_at.filter(|t| *t >= window).map(|t| (t - r.received_at).num_seconds().max(0)))
        .collect();
    if v.is_empty() {
        None
    } else {
        Some(Duration::seconds(v.iter().sum::<i64>() / v.len() as i64))
    }
}

pub fn scheduled_this_week(reqs: &[Request], now: DateTime<Utc>) -> usize {
    let week = now - Duration::days(7);
    reqs.iter()
        .filter(|r| r.status == Status::Scheduled)
        .filter(|r| r.activity.iter().any(|a| a.action == "status_scheduled" && a.at >= week) || r.received_at >= week)
        .count()
}

// ---------------------------------------------------------------- patient forms

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct PatientForm {
    pub id: u64,
    #[serde(rename = "ref", default)]
    pub reference: String,
    pub name: String,
    #[serde(default)]
    pub dob: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub visit_date: String,
    #[serde(default)]
    pub conditions: String,
    #[serde(default)]
    pub medications: String,
    #[serde(default)]
    pub allergies: String,
    #[serde(default)]
    pub last_dental: String,
    #[serde(default)]
    pub carrier: String,
    #[serde(default)]
    pub member_id: String,
    #[serde(default)]
    pub signature: String,
    pub submitted_at: DateTime<Utc>,
    #[serde(default)]
    pub attached_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub attached_by: String,
}

impl PatientForm {
    pub fn attached(&self) -> bool {
        self.attached_at.is_some()
    }
    /// Allergies worth flagging (ignores "none", "n/a", "no known allergies" and blanks).
    pub fn has_allergy(&self) -> bool {
        let a = self.allergies.trim().to_lowercase();
        !(a.is_empty() || a == "none" || a == "n/a" || a == "na" || a == "no" || a.starts_with("no known") || a == "nka" || a == "nkda")
    }
    pub fn pdf_name(&self) -> String {
        let mut parts: Vec<&str> = self.name.split_whitespace().collect();
        let last = parts.pop().unwrap_or("Patient");
        let first = parts.join("-");
        let clean = |s: &str| s.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect::<String>();
        if first.is_empty() {
            format!("{}-new-patient-form.pdf", clean(last))
        } else {
            format!("{}-{}-new-patient-form.pdf", clean(last), clean(&first))
        }
    }
    pub fn visit(&self) -> Option<NaiveDate> {
        parse_date(&self.visit_date)
    }
}

/// An upcoming first visit that the front desk should have paperwork for.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Upcoming {
    pub name: String,
    pub visit_date: NaiveDate,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub request_id: u64,
    #[serde(default)]
    pub link_sent: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpcomingRow {
    pub up: Upcoming,
    pub form_received: bool,
}

/// New patients visiting today or within the next two days, with whether their form arrived.
pub fn upcoming_new_patients(data: &Data, today: NaiveDate) -> Vec<UpcomingRow> {
    let until = today + Duration::days(2);
    let mut v: Vec<Upcoming> = data
        .requests
        .iter()
        .filter(|r| r.status == Status::Scheduled && r.new_patient)
        .filter_map(|r| {
            let d = r.appt_date?;
            Some(Upcoming {
                name: r.name(),
                visit_date: d,
                phone: r.phone.clone(),
                email: r.email.clone(),
                request_id: r.id,
                link_sent: r.activity.iter().any(|a| a.action == "form_link_sent"),
            })
        })
        .collect();
    for u in &data.upcoming {
        if !v.iter().any(|x| x.name.eq_ignore_ascii_case(&u.name)) {
            v.push(u.clone());
        }
    }
    v.retain(|u| u.visit_date >= today && u.visit_date <= until);
    v.sort_by(|a, b| a.visit_date.cmp(&b.visit_date).then(a.name.cmp(&b.name)));
    v.into_iter()
        .map(|up| {
            let received = data.forms.iter().any(|f| {
                f.name.trim().eq_ignore_ascii_case(up.name.trim())
                    || (!up.email.is_empty() && f.email.trim().eq_ignore_ascii_case(up.email.trim()))
            });
            UpcomingRow { up, form_received: received }
        })
        .collect()
}

// ---------------------------------------------------------------- savings plan

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Member {
    pub id: u64,
    pub name: String,
    pub plan: String,
    #[serde(default)]
    pub household: Vec<String>,
    pub enrolled: NaiveDate,
    pub renews: NaiveDate,
    pub payment: String,
    #[serde(default)]
    pub price: u32,
    #[serde(default)]
    pub last_reminder: Option<DateTime<Utc>>,
}

impl Member {
    pub fn people(&self) -> usize {
        self.household.len().max(1)
    }
    pub fn yearly(&self) -> u32 {
        plan_price(&self.plan, self.people())
    }
    pub fn days_to_renewal(&self, today: NaiveDate) -> i64 {
        (self.renews - today).num_days()
    }
    pub fn renewing_soon(&self, today: NaiveDate) -> bool {
        self.days_to_renewal(today) <= RENEWAL_WINDOW_DAYS
    }
    pub fn payment_due(&self) -> bool {
        self.payment == "due"
    }
    pub fn household_text(&self) -> String {
        self.household.join(", ")
    }
    pub fn matches(&self, q: &str) -> bool {
        let q = q.trim().to_lowercase();
        q.is_empty() || self.name.to_lowercase().contains(&q) || self.household.iter().any(|h| h.to_lowercase().contains(&q))
    }
}

pub fn add_year(d: NaiveDate) -> NaiveDate {
    d.checked_add_months(Months::new(12)).unwrap_or(d + Duration::days(365))
}

pub fn renewal_countdown(days: i64) -> String {
    match days {
        d if d < -1 => format!("{} days overdue", -d),
        -1 => "1 day overdue".into(),
        0 => "today".into(),
        1 => "tomorrow".into(),
        d => format!("in {d} days"),
    }
}

// ---------------------------------------------------------------- reviews

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Visit {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    pub visit_date: NaiveDate,
    #[serde(default)]
    pub last_asked: Option<DateTime<Utc>>,
}

impl Visit {
    pub fn asked_recently(&self, now: DateTime<Utc>) -> bool {
        self.last_asked.is_some_and(|t| now - t < Duration::days(REVIEW_COOLDOWN_DAYS))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReviewLog {
    pub id: u64,
    pub name: String,
    pub channel: String,
    #[serde(default)]
    pub by: String,
    pub at: DateTime<Utc>,
}

pub fn review_message(first_name: &str, link: &str, email: bool) -> String {
    let link = if link.trim().is_empty() { "g.page/r/northview-dental/review" } else { link.trim() };
    if email {
        format!(
            "Hi {first_name},\n\nThank you for visiting Northview Dental today! If you have a moment, we'd be grateful for a quick Google review:\n{link}\n\nThank you,\nThe Northview Dental team\n\nTo stop receiving these emails, reply STOP."
        )
    } else {
        format!(
            "Hi {first_name}, thanks for visiting Northview Dental! If you have a moment, we'd be grateful for a quick Google review: {link}\nReply STOP to opt out."
        )
    }
}

/// GSM-7 segments: 160 chars in one message, 153 per part when split.
pub fn sms_segments(chars: usize) -> usize {
    if chars <= 160 { 1 } else { chars.div_ceil(153) }
}

// ---------------------------------------------------------------- site settings and snapshot

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SiteSettings {
    #[serde(default)]
    pub review_link: String,
    #[serde(default)]
    pub google_profile_url: String,
    #[serde(default = "default_rating")]
    pub google_rating: String,
    #[serde(default)]
    pub practice_phone: String,
    #[serde(default)]
    pub patient_form_url: String,
    #[serde(default)]
    pub patient_forms_on: bool,
    #[serde(default)]
    pub messaging_live: bool,
}

fn default_rating() -> String {
    "5.0".into()
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            review_link: String::new(),
            google_profile_url: String::new(),
            google_rating: default_rating(),
            practice_phone: "317.818.7777".into(),
            patient_form_url: String::new(),
            patient_forms_on: true,
            messaging_live: false,
        }
    }
}

/// Everything the screens show. In live mode this is replaced by each website sync;
/// in demo mode it is edited locally and saved to the demo file.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct Data {
    #[serde(default)]
    pub settings: SiteSettings,
    #[serde(default)]
    pub requests: Vec<Request>,
    #[serde(default)]
    pub forms: Vec<PatientForm>,
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default)]
    pub visits: Vec<Visit>,
    #[serde(default)]
    pub review_log: Vec<ReviewLog>,
    /// First visits booked directly in Eaglesoft (demo data only).
    #[serde(default)]
    pub upcoming: Vec<Upcoming>,
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub simulated: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub server_time: DateTime<Utc>,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub settings: SiteSettings,
    #[serde(default)]
    pub requests: Vec<Request>,
    #[serde(default)]
    pub forms: Vec<PatientForm>,
    #[serde(default)]
    pub members: Vec<Member>,
    #[serde(default)]
    pub visits: Vec<Visit>,
    #[serde(default)]
    pub review_log: Vec<ReviewLog>,
}

impl From<Snapshot> for Data {
    fn from(s: Snapshot) -> Self {
        Data {
            settings: s.settings,
            requests: s.requests,
            forms: s.forms,
            members: s.members,
            visits: s.visits,
            review_log: s.review_log,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------- actions

#[derive(Clone, Debug, PartialEq)]
pub enum MemberOp {
    Paid,
    Renew,
    Remind,
}

impl MemberOp {
    pub fn key(&self) -> &'static str {
        match self {
            MemberOp::Paid => "paid",
            MemberOp::Renew => "renew",
            MemberOp::Remind => "remind",
        }
    }
}

/// Every change staff can make. Demo mode applies it locally; live mode posts it to the website.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    SetStatus { id: u64, status: Status, note: String, appt: Option<NaiveDate> },
    Viewed(u64),
    Copied(u64),
    FormAttached(u64),
    FormPdf(u64),
    FormLink { request_id: u64, name: String },
    MemberAdd { name: String, plan: String, enrolled: NaiveDate, household: Vec<String>, paid: bool },
    Member { id: u64, op: MemberOp },
    VisitAdd { name: String, phone: String, email: String, date: NaiveDate },
    Review { visit_id: u64, channel: String },
}

impl Data {
    fn bump(&mut self) -> u64 {
        let max = self
            .requests.iter().map(|r| r.id)
            .chain(self.forms.iter().map(|f| f.id))
            .chain(self.members.iter().map(|m| m.id))
            .chain(self.visits.iter().map(|v| v.id))
            .chain(self.review_log.iter().map(|l| l.id))
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max) + 1;
        self.next_id
    }

    /// Apply an action locally (demo mode). Mirrors the website plugin's behaviour.
    pub fn apply(&mut self, act: &Act, user: &str, now: DateTime<Utc>) -> Result<(), String> {
        let log = |action: &str, detail: String| Activity { action: action.into(), detail, by: user.into(), at: now };
        match act {
            Act::SetStatus { id, status, note, appt } => {
                let r = self.requests.iter_mut().find(|r| r.id == *id).ok_or("Request not found")?;
                r.status = *status;
                if *status != Status::New && r.first_action_at.is_none() {
                    r.first_action_at = Some(now);
                }
                if appt.is_some() {
                    r.appt_date = *appt;
                }
                let mut d = note.clone();
                if let Some(a) = appt {
                    if !d.is_empty() {
                        d.push_str(" · ");
                    }
                    d.push_str(&format!("Appointment {}", a.format("%Y-%m-%d")));
                }
                r.activity.push(log(&format!("status_{}", status.key()), d));
                if let (Status::Scheduled, Some(a)) = (*status, *appt) {
                    let (name, phone, email, rid) = (r.name(), r.phone.clone(), r.email.clone(), r.id);
                    let _ = rid;
                    if !self.visits.iter().any(|v| v.name == name && v.visit_date == a) {
                        let id = self.bump();
                        self.visits.push(Visit { id, name, phone, email, visit_date: a, last_asked: None });
                    }
                }
            }
            Act::Viewed(id) | Act::Copied(id) => {
                let r = self.requests.iter_mut().find(|r| r.id == *id).ok_or("Request not found")?;
                let action = if matches!(act, Act::Viewed(_)) { "viewed" } else { "copied" };
                r.activity.push(log(action, String::new()));
            }
            Act::FormAttached(id) => {
                let f = self.forms.iter_mut().find(|f| f.id == *id).ok_or("Form not found")?;
                f.attached_at = Some(now);
                f.attached_by = user.into();
            }
            Act::FormPdf(_) => {}
            Act::FormLink { request_id, name } => {
                if let Some(r) = self.requests.iter_mut().find(|r| r.id == *request_id) {
                    r.activity.push(log("form_link_sent", "simulated".into()));
                } else if let Some(u) = self.upcoming.iter_mut().find(|u| &u.name == name) {
                    u.link_sent = true;
                }
            }
            Act::MemberAdd { name, plan, enrolled, household, paid } => {
                if name.trim().is_empty() {
                    return Err("Membership name is required".into());
                }
                let id = self.bump();
                let household = if household.is_empty() { vec![name.trim().to_string()] } else { household.clone() };
                let price = plan_price(plan, household.len());
                self.members.push(Member {
                    id,
                    name: name.trim().into(),
                    plan: plan.clone(),
                    household,
                    enrolled: *enrolled,
                    renews: add_year(*enrolled),
                    payment: if *paid { "paid".into() } else { "due".into() },
                    price,
                    last_reminder: None,
                });
            }
            Act::Member { id, op } => {
                let m = self.members.iter_mut().find(|m| m.id == *id).ok_or("Membership not found")?;
                match op {
                    MemberOp::Paid => m.payment = "paid".into(),
                    MemberOp::Renew => {
                        m.renews = add_year(m.renews);
                        m.payment = "due".into();
                    }
                    MemberOp::Remind => m.last_reminder = Some(now),
                }
            }
            Act::VisitAdd { name, phone, email, date } => {
                if name.trim().is_empty() {
                    return Err("Patient name is required".into());
                }
                let id = self.bump();
                self.visits.push(Visit { id, name: name.trim().into(), phone: phone.clone(), email: email.clone(), visit_date: *date, last_asked: None });
            }
            Act::Review { visit_id, channel } => {
                let v = self.visits.iter().find(|v| v.id == *visit_id).ok_or("Visit not found")?.clone();
                if v.asked_recently(now) {
                    return Err("This patient was asked in the last 180 days".into());
                }
                let id = self.bump();
                self.review_log.insert(0, ReviewLog { id, name: v.name.clone(), channel: channel.clone(), by: user.into(), at: now });
                for x in self.visits.iter_mut() {
                    let same = x.id == v.id
                        || (!v.phone.is_empty() && digits_of(&x.phone) == digits_of(&v.phone))
                        || (!v.email.is_empty() && x.email.eq_ignore_ascii_case(&v.email));
                    if same {
                        x.last_asked = Some(now);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn members_sorted(&self) -> Vec<&Member> {
        let mut v: Vec<&Member> = self.members.iter().collect();
        v.sort_by(|a, b| a.renews.cmp(&b.renews).then(a.name.cmp(&b.name)));
        v
    }

    pub fn recent_visits(&self, today: NaiveDate) -> Vec<&Visit> {
        let from = today - Duration::days(7);
        let mut v: Vec<&Visit> = self.visits.iter().filter(|v| v.visit_date >= from && v.visit_date <= today).collect();
        v.sort_by(|a, b| b.visit_date.cmp(&a.visit_date).then(a.name.cmp(&b.name)));
        v
    }

    pub fn forms_to_attach(&self) -> usize {
        self.forms.iter().filter(|f| !f.attached()).count()
    }
}

// ---------------------------------------------------------------- small helpers

pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().filter(|p| p.chars().next().is_some_and(|c| c.is_alphabetic())).collect();
    let mut s = String::new();
    if let Some(f) = parts.first() {
        s.extend(f.chars().next().map(|c| c.to_ascii_uppercase()));
    }
    if parts.len() > 1 {
        s.extend(parts[parts.len() - 1].chars().next().map(|c| c.to_ascii_uppercase()));
    }
    if s.is_empty() { "?".into() } else { s }
}

pub fn digits_of(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Accepts the date formats WPForms and staff commonly produce.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    for f in ["%Y-%m-%d", "%m/%d/%Y", "%m-%d-%Y", "%b %d, %Y", "%B %d, %Y", "%d/%m/%Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, f) {
            return Some(d);
        }
    }
    None
}

pub fn age(dob: NaiveDate, today: NaiveDate) -> i32 {
    let mut a = today.year() - dob.year();
    if (today.month(), today.day()) < (dob.month(), dob.day()) {
        a -= 1;
    }
    a
}

pub fn ago(t: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let s = (now - t).num_seconds().max(0);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86_399 => format!("{} h ago", s / 3600),
        86_400..=172_799 => "Yesterday".into(),
        _ => format!("{} days ago", s / 86_400),
    }
}

pub fn fmt_duration(d: Duration) -> String {
    let m = d.num_minutes();
    if m < 60 {
        format!("{m} min")
    } else if m < 48 * 60 {
        format!("{:.1} h", m as f64 / 60.0)
    } else {
        format!("{:.1} d", m as f64 / 1440.0)
    }
}

pub fn money(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("${out}")
}
