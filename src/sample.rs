//! Fictional sample data for demo mode. Times are relative to when the demo is seeded
//! so the inbox always looks current. No real patient information.

use crate::model::*;
use chrono::{DateTime, Duration, Local, NaiveDate, Utc};

pub const DEMO_REVIEW_LINK: &str = "g.page/r/northview-dental/review";

fn act(action: &str, by: &str, at: DateTime<Utc>, detail: &str) -> Activity {
    Activity { action: action.into(), detail: detail.into(), by: by.into(), at }
}

#[allow(clippy::too_many_arguments)]
fn req(
    id: u64, first: &str, last: &str, phone: &str, email: &str, dob: &str, ins: &str, reason: &str,
    new_patient: bool, preferred: &[&str], message: &str, received: DateTime<Utc>, status: Status,
) -> Request {
    Request {
        id,
        reference: format!("R-{id}"),
        first_name: first.into(),
        last_name: last.into(),
        phone: phone.into(),
        email: email.into(),
        address: String::new(),
        dob: dob.into(),
        insurance: ins.into(),
        reason: reason.into(),
        new_patient,
        emergency: reason.to_lowercase().contains("emergency"),
        preferred: preferred.iter().map(|s| s.to_string()).collect(),
        message: message.into(),
        received_at: received,
        status,
        first_action_at: None,
        appt_date: None,
        activity: vec![],
    }
}

pub fn seed(now: DateTime<Utc>) -> Data {
    let today: NaiveDate = now.with_timezone(&Local).date_naive();
    let m = |mins: i64| now - Duration::minutes(mins);
    let d = |days: i64| today + Duration::days(days);

    let tamara = req(1042, "Tamara", "Ellison", "(317) 555-0148", "tamara.ellison@example.com", "1988-03-14", "Delta Dental",
        "Dental emergency", true, &["Mon · Morning", "Tue · Morning"], "Cracked a molar last night, some pain when chewing.", m(24), Status::New);
    let jordan = req(1044, "Jordan", "Price", "(317) 555-0121", "jordan@example.com", "1990-05-17", "Anthem",
        "Routine cleaning & exam", true, &["Wednesday · Morning"], "", m(2), Status::New);
    let marcus = req(1039, "Marcus", "Bell", "(317) 555-0193", "mbell@example.com", "1975-11-02", "Cigna",
        "Dental implants consultation", true, &["Thursday · Afternoon"], "Would like to discuss options for a missing lower tooth.", m(3 * 60 + 5), Status::New);
    let mut priya = req(1031, "Priya", "Raman", "(317) 555-0112", "priya.r@example.com", "1993-08-21", "MetLife",
        "Routine cleaning & exam", false, &["Friday · Morning"], "Best to call after 4pm.", m(20 * 60), Status::Contacted);
    priya.first_action_at = Some(m(18 * 60));
    priya.activity = vec![act("status_contacted", "Front Desk", m(18 * 60), "Left voicemail")];

    let mut owen = req(1027, "Owen", "Fischer", "(317) 555-0177", "owen.f@example.com", "1982-01-30", "Guardian",
        "New patient visit", true, &["Tuesday · Morning"], "", m(2 * 24 * 60), Status::Scheduled);
    owen.first_action_at = Some(m(2 * 24 * 60 - 95));
    owen.appt_date = Some(d(2));
    owen.activity = vec![act("status_scheduled", "Front Desk", m(2 * 24 * 60 - 95), "")];

    let mut hana = req(1029, "Hana", "Sato", "(317) 555-0156", "hana.sato@example.com", "1996-06-09", "",
        "New patient visit", true, &["Monday · Afternoon"], "", m(30 * 60), Status::Scheduled);
    hana.first_action_at = Some(m(30 * 60 - 140));
    hana.appt_date = Some(d(1));
    hana.activity = vec![act("status_scheduled", "Office Manager", m(30 * 60 - 140), "")];

    let mut grace = req(1018, "Grace", "Okafor", "(317) 555-0164", "gokafor@example.com", "1987-12-12", "Aetna",
        "Routine cleaning & exam", false, &["Monday · Morning"], "", m(9 * 24 * 60), Status::Scheduled);
    grace.first_action_at = Some(m(9 * 24 * 60 - 200));
    grace.appt_date = Some(today);
    grace.activity = vec![act("status_scheduled", "Front Desk", m(9 * 24 * 60 - 200), "")];

    let mut spam = req(1035, "Test", "Test", "(000) 000-0000", "test@test.test", "", "",
        "Other", true, &[], "SEO services for your website!!", m(26 * 60), Status::Declined);
    spam.first_action_at = Some(m(25 * 60));
    spam.activity = vec![act("status_declined", "Front Desk", m(25 * 60), "Spam")];

    let forms = vec![
        PatientForm {
            id: 311,
            reference: "F-311".into(),
            name: "Marcus Bell".into(),
            dob: "1975-11-02".into(),
            phone: "(317) 555-0193".into(),
            email: "mbell@example.com".into(),
            address: "4120 Allisonville Rd, Indianapolis, IN 46205".into(),
            visit_date: d(1).format("%Y-%m-%d").to_string(),
            conditions: "High blood pressure (controlled)".into(),
            medications: "Lisinopril 10 mg daily".into(),
            allergies: "Penicillin".into(),
            last_dental: "About 3 years ago".into(),
            carrier: "Cigna Dental PPO".into(),
            member_id: "U4482 91307".into(),
            signature: "Marcus Bell".into(),
            submitted_at: m(2 * 60),
            attached_at: None,
            attached_by: String::new(),
        },
        PatientForm {
            id: 310,
            reference: "F-310".into(),
            name: "Owen Fischer".into(),
            dob: "1982-01-30".into(),
            phone: "(317) 555-0177".into(),
            email: "owen.f@example.com".into(),
            address: "8802 N Meridian St, Indianapolis, IN 46260".into(),
            visit_date: d(2).format("%Y-%m-%d").to_string(),
            conditions: "None".into(),
            medications: "None".into(),
            allergies: "None".into(),
            last_dental: "Within the last year".into(),
            carrier: "Guardian".into(),
            member_id: "GD-55120-03".into(),
            signature: "Owen Fischer".into(),
            submitted_at: m(26 * 60),
            attached_at: None,
            attached_by: String::new(),
        },
    ];

    let members = vec![
        Member { id: 501, name: "Hannah Whitley".into(), plan: "single".into(), household: vec!["Hannah Whitley".into()],
            enrolled: d(13) - Duration::days(365), renews: d(13), payment: "paid".into(), price: 500, last_reminder: None },
        Member { id: 502, name: "Derrick & Lisa Moore".into(), plan: "dual".into(), household: vec!["Derrick Moore".into(), "Lisa Moore".into()],
            enrolled: d(25) - Duration::days(365), renews: d(25), payment: "paid".into(), price: 925, last_reminder: None },
        Member { id: 503, name: "Nguyen family".into(), plan: "family3".into(), household: vec!["Linh Nguyen".into(), "Bao Nguyen".into(), "An Nguyen".into()],
            enrolled: d(165) - Duration::days(365), renews: d(165), payment: "paid".into(), price: 1350, last_reminder: None },
        Member { id: 504, name: "Alvarez family".into(), plan: "family4".into(), household: vec!["Rosa Alvarez".into(), "Luis Alvarez".into(), "Mateo Alvarez".into(), "Elena Alvarez".into()],
            enrolled: d(245) - Duration::days(365), renews: d(245), payment: "paid".into(), price: 1500, last_reminder: None },
        Member { id: 505, name: "Samuel Ortiz".into(), plan: "single".into(), household: vec!["Samuel Ortiz".into()],
            enrolled: d(-6), renews: add_year(d(-6)), payment: "due".into(), price: 500, last_reminder: None },
    ];

    let visits = vec![
        Visit { id: 701, name: "Priya Raman".into(), phone: "(317) 555-0112".into(), email: "priya.r@example.com".into(), visit_date: today, last_asked: Some(m(15)) },
        Visit { id: 702, name: "Grace Okafor".into(), phone: "(317) 555-0164".into(), email: "gokafor@example.com".into(), visit_date: today, last_asked: None },
        Visit { id: 703, name: "Hannah Whitley".into(), phone: "(317) 555-0130".into(), email: "hwhitley@example.com".into(), visit_date: d(-1), last_asked: Some(m(60 * 24 * 60)) },
        Visit { id: 704, name: "Ben Carter".into(), phone: "(317) 555-0185".into(), email: "bcarter@example.com".into(), visit_date: d(-1), last_asked: None },
    ];
    let review_log = vec![
        ReviewLog { id: 801, name: "Priya Raman".into(), channel: "text".into(), by: "Front Desk".into(), at: m(15) },
        ReviewLog { id: 802, name: "Hannah Whitley".into(), channel: "text".into(), by: "Front Desk".into(), at: m(60 * 24 * 60) },
    ];

    let upcoming = vec![Upcoming {
        name: "Tamara Ellison".into(),
        visit_date: d(1),
        phone: "(317) 555-0148".into(),
        email: "tamara.ellison@example.com".into(),
        request_id: 0,
        link_sent: false,
    }];

    Data {
        settings: SiteSettings { review_link: DEMO_REVIEW_LINK.into(), google_profile_url: "https://www.google.com/maps/search/Northview+Dental+Indianapolis".into(), ..Default::default() },
        requests: vec![tamara, jordan, marcus, priya, owen, hana, grace, spam],
        forms,
        members,
        visits,
        review_log,
        upcoming,
        next_id: 2000,
        simulated: 0,
    }
}

const SIM_PEOPLE: [(&str, &str, &str, bool); 6] = [
    ("Avery", "Collins", "Routine cleaning & exam", true),
    ("Noah", "Patel", "Cosmetic dentistry consultation", true),
    ("Maya", "Brooks", "Dental emergency", true),
    ("Ethan", "Kim", "Routine cleaning & exam", false),
    ("Sofia", "Ramirez", "Orthodontic consultation", true),
    ("Liam", "Turner", "Dental emergency", false),
];

/// A request as if a patient just submitted the website form. Every third one is an emergency.
pub fn simulated_request(data: &mut Data, now: DateTime<Utc>) -> Request {
    let n = data.simulated as usize;
    data.simulated += 1;
    let emergency = n % 3 == 2;
    let (first, last, mut reason, new_patient) = SIM_PEOPLE[n % SIM_PEOPLE.len()];
    if emergency {
        reason = "Dental emergency";
    } else if reason == "Dental emergency" {
        reason = "Routine cleaning & exam";
    }
    let id = data.requests.iter().map(|r| r.id).max().unwrap_or(1000).max(1044) + 1;
    let mut r = req(
        id, first, last, &format!("(317) 555-{:04}", 200 + n * 7), &format!("{}.{}@example.com", first.to_lowercase(), last.to_lowercase()),
        "1991-04-22", if n % 2 == 0 { "Delta Dental" } else { "" }, reason, new_patient, &["Thursday · Morning"],
        if emergency { "Swelling on the left side since this morning." } else { "" }, now, Status::New,
    );
    r.emergency = emergency;
    r
}
