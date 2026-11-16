//! Business-rule tests, API contract tests and storage tests.

use crate::model::*;
use crate::sample;
use crate::storage::{Loaded, Storage};
use chrono::{Duration, NaiveDate, TimeZone, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 15, 14, 0, 0).unwrap()
}

#[test]
fn plan_prices_match_website() {
    assert_eq!(plan_price("single", 1), 500);
    assert_eq!((plan_size("single"), plan_size("dual"), plan_size("family3"), plan_size("family4")), (1, 2, 3, 4));
    assert_eq!(plan_price("dual", 2), 925);
    assert_eq!(plan_price("family3", 3), 1350);
    assert_eq!(plan_price("family4", 4), 1500);
    assert_eq!(plan_price("family4", 6), 1500 + 2 * 450);
    assert_eq!(plan_price("dual", 3), 925 + 450);
    assert_eq!(plan_price("single", 0), 500);
    assert_eq!(suggested_plan(1), "single");
    assert_eq!(suggested_plan(5), "family4");
}

#[test]
fn renewal_is_one_year_and_handles_leap_day() {
    let d = NaiveDate::from_ymd_opt(2028, 2, 29).unwrap();
    assert_eq!(add_year(d), NaiveDate::from_ymd_opt(2029, 2, 28).unwrap());
    let mut data = sample::seed(now());
    let today = now().date_naive();
    let m = data.members_sorted()[0].clone();
    assert!(m.renewing_soon(today));
    data.apply(&Act::Member { id: m.id, op: MemberOp::Renew }, "Front Desk", now()).unwrap();
    let after = data.members.iter().find(|x| x.id == m.id).unwrap();
    assert_eq!(after.renews, add_year(m.renews));
    assert!(after.payment_due());
    data.apply(&Act::Member { id: m.id, op: MemberOp::Paid }, "Front Desk", now()).unwrap();
    assert!(!data.members.iter().find(|x| x.id == m.id).unwrap().payment_due());
}

#[test]
fn members_sorted_by_soonest_renewal() {
    let data = sample::seed(now());
    let v = data.members_sorted();
    assert!(v.windows(2).all(|w| w[0].renews <= w[1].renews));
}

#[test]
fn emergencies_are_pinned_and_rest_newest_first() {
    let data = sample::seed(now());
    let (pinned, rest) = inbox(&data.requests, Filter::Open, "");
    assert!(pinned.iter().all(|r| r.emergency));
    assert_eq!(pinned.len(), 1);
    assert!(rest.windows(2).all(|w| w[0].received_at >= w[1].received_at));
    // a closed emergency is no longer pinned
    let mut reqs = data.requests.clone();
    reqs.iter_mut().find(|r| r.emergency).unwrap().status = Status::Scheduled;
    let (pinned, _) = inbox(&reqs, Filter::All, "");
    assert!(pinned.is_empty());
}

#[test]
fn filters_and_search() {
    let data = sample::seed(now());
    let open = count_status(&data.requests, Filter::Open);
    assert_eq!(open, count_status(&data.requests, Filter::New) + count_status(&data.requests, Filter::Contacted));
    let (p, r) = inbox(&data.requests, Filter::Open, "mar");
    let names: Vec<String> = p.iter().chain(r.iter()).map(|r| r.name()).collect();
    assert!(names.contains(&"Tamara Ellison".to_string()));
    assert!(names.contains(&"Marcus Bell".to_string()));
    let (p, r) = inbox(&data.requests, Filter::All, "555-0121");
    assert_eq!(p.len() + r.len(), 1);
    let (p, r) = inbox(&data.requests, Filter::All, "implant");
    assert_eq!(r.first().map(|r| r.first_name.as_str()), Some("Marcus"));
    assert!(p.is_empty());
}

#[test]
fn first_response_average() {
    let mut a = Request { id: 1, received_at: now() - Duration::hours(5), ..Default::default() };
    a.first_action_at = Some(now() - Duration::hours(3));
    let mut b = Request { id: 2, received_at: now() - Duration::hours(2), ..Default::default() };
    b.first_action_at = Some(now() - Duration::hours(1));
    let avg = avg_first_response(&[a, b], now()).unwrap();
    assert_eq!(avg.num_minutes(), 90);
    assert_eq!(fmt_duration(avg), "1.5 h");
    assert!(avg_first_response(&[], now()).is_none());
}

#[test]
fn status_change_records_activity_and_first_action() {
    let mut data = sample::seed(now());
    let id = 1044;
    data.apply(&Act::SetStatus { id, status: Status::Contacted, note: "Left voicemail".into(), appt: None }, "Front Desk", now()).unwrap();
    let r = data.requests.iter().find(|r| r.id == id).unwrap();
    assert_eq!(r.status, Status::Contacted);
    assert_eq!(r.first_action_at, Some(now()));
    let tl = r.timeline();
    assert_eq!(tl.first().unwrap().title(), "Marked contacted by Front Desk");
    assert_eq!(tl.last().unwrap().action, "received");
    // scheduling with a date creates a visit for review requests later
    let appt = now().date_naive() + Duration::days(1);
    data.apply(&Act::SetStatus { id, status: Status::Scheduled, note: String::new(), appt: Some(appt) }, "Front Desk", now()).unwrap();
    assert!(data.visits.iter().any(|v| v.name == "Jordan Price" && v.visit_date == appt));
    // first action time is kept from the first change
    assert_eq!(data.requests.iter().find(|r| r.id == id).unwrap().first_action_at, Some(now()));
}

#[test]
fn review_cooldown_blocks_repeat_requests() {
    let mut data = sample::seed(now());
    let v = data.visits.iter().find(|v| v.last_asked.is_none()).unwrap().clone();
    data.apply(&Act::Review { visit_id: v.id, channel: "text".into() }, "Front Desk", now()).unwrap();
    assert!(data.visits.iter().find(|x| x.id == v.id).unwrap().asked_recently(now()));
    assert!(data.apply(&Act::Review { visit_id: v.id, channel: "email".into() }, "Front Desk", now()).is_err());
    assert_eq!(data.review_log[0].name, v.name);
    let old = Visit { id: 1, name: "A".into(), phone: String::new(), email: String::new(), visit_date: now().date_naive(), last_asked: Some(now() - Duration::days(181)) };
    assert!(!old.asked_recently(now()));
}

#[test]
fn sms_message_has_opt_out_and_fits() {
    let m = review_message("Grace", sample::DEMO_REVIEW_LINK, false);
    assert!(m.contains("Reply STOP to opt out"));
    assert!(m.contains(sample::DEMO_REVIEW_LINK));
    assert_eq!(sms_segments(160), 1);
    assert_eq!(sms_segments(161), 2);
    assert_eq!(sms_segments(306), 2);
    assert_eq!(sms_segments(307), 3);
}

#[test]
fn eaglesoft_copy_block() {
    let data = sample::seed(now());
    let r = data.requests.iter().find(|r| r.id == 1042).unwrap();
    let s = r.eaglesoft_text(NaiveDate::from_ymd_opt(2026, 11, 15).unwrap());
    assert!(s.contains("Name:        Tamara Ellison"));
    assert!(s.contains("DOB:         03/14/1988 (age 38)"));
    assert!(s.contains("Dental emergency"));
    assert!(!s.contains("Address:"));
}

#[test]
fn upcoming_patients_and_form_matching() {
    let data = sample::seed(now());
    let today = now().with_timezone(&chrono::Local).date_naive();
    let ups = upcoming_new_patients(&data, today);
    let owen = ups.iter().find(|u| u.up.name == "Owen Fischer").unwrap();
    assert!(owen.form_received);
    let hana = ups.iter().find(|u| u.up.name == "Hana Sato").unwrap();
    assert!(!hana.form_received);
    assert!(ups.iter().all(|u| u.up.visit_date >= today && u.up.visit_date <= today + Duration::days(2)));
}

#[test]
fn allergy_flag_and_pdf_name() {
    let data = sample::seed(now());
    let marcus = data.forms.iter().find(|f| f.name == "Marcus Bell").unwrap();
    assert!(marcus.has_allergy());
    assert_eq!(marcus.pdf_name(), "Bell-Marcus-new-patient-form.pdf");
    let owen = data.forms.iter().find(|f| f.name == "Owen Fischer").unwrap();
    assert!(!owen.has_allergy());
}

#[test]
fn simulated_requests_include_emergencies() {
    let mut data = sample::seed(now());
    let rs: Vec<Request> = (0..3).map(|_| sample::simulated_request(&mut data, now())).collect();
    assert!(!rs[0].emergency && !rs[1].emergency && rs[2].emergency);
}

#[test]
fn helpers() {
    assert_eq!(initials("Derrick & Lisa Moore"), "DM");
    assert_eq!(initials("jordan price"), "JP");
    assert_eq!(money(4775), "$4,775");
    assert_eq!(money(500), "$500");
    assert_eq!(parse_date("05/17/1990"), NaiveDate::from_ymd_opt(1990, 5, 17));
    assert_eq!(age(NaiveDate::from_ymd_opt(1990, 5, 17).unwrap(), NaiveDate::from_ymd_opt(2026, 11, 15).unwrap()), 36);
    assert_eq!(renewal_countdown(13), "in 13 days");
    assert_eq!(ago(now() - Duration::minutes(24), now()), "24 min ago");
}

/// The website plugin's JSON (as returned by /wp-json/nvoffice/v1/sync) parses into our types.
#[test]
fn parses_website_snapshot() {
    let json = r#"{
      "server_time":"2026-11-15T10:02:52Z","today":"2026-11-15","host":"nvdental-elwp.s-turl.com","me":null,
      "settings":{"review_link":"","google_profile_url":"","google_rating":"5.0","practice_phone":"317.818.7777","patient_form_url":"","patient_forms_on":false,"messaging_live":false},
      "requests":[{"id":1,"ref":"R-1","first_name":"Zz Test","last_name":"Delete Me","phone":"(317) 555-0100","email":"test@example.com","address":"","dob":"","insurance":"","reason":"Dental Emergency","new_patient":true,"emergency":true,"preferred":["Monday · Morning"],"message":"x","received_at":"2026-11-15T10:02:38Z","status":"contacted","first_action_at":"2026-11-15T10:02:52Z","appt_date":null,"activity":[{"action":"status_contacted","detail":"Left voicemail","by":"Test Admin","at":"2026-11-15T10:02:52Z"}]}],
      "forms":[],"members":[{"id":1,"name":"Zz Test family","plan":"dual","household":["A","B","C"],"enrolled":"2025-11-25","renews":"2026-11-25","payment":"due","price":1375,"last_reminder":null}],
      "visits":[],"review_log":[]
    }"#;
    let s: Snapshot = serde_json::from_str(json).unwrap();
    let d: Data = s.into();
    assert_eq!(d.requests[0].status, Status::Contacted);
    assert_eq!(d.requests[0].reference, "R-1");
    assert!(!d.settings.patient_forms_on);
    assert_eq!(d.members[0].yearly(), 1375);
    assert_eq!(d.members[0].yearly(), d.members[0].price);
}

#[test]
fn storage_round_trip_backup_and_damage() {
    let dir = std::env::temp_dir().join(format!("nvo-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let st = Storage::new(Some(dir.clone()));
    assert!(matches!(st.load_demo(), Loaded::Missing));
    let mut d = sample::seed(now());
    st.save_demo(&d).unwrap();
    d.members.clear();
    st.save_demo(&d).unwrap();
    match st.load_demo() {
        Loaded::Ok(x) => assert!(x.members.is_empty()),
        _ => panic!("expected data"),
    }
    std::fs::write(st.demo_path(), "{ not json").unwrap();
    assert!(matches!(st.load_demo(), Loaded::Damaged(_)));
    let restored = st.restore_backup().unwrap();
    assert_eq!(restored.members.len(), 5);
    assert!(st.demo_path().with_extension("json.damaged").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pdf_renders_letter_size() {
    let data = sample::seed(now());
    let f = &data.forms[0];
    let bytes = crate::pdf::render(f, "317.818.7777", now().date_naive()).expect("pdf");
    assert!(bytes.starts_with(b"%PDF"));
    let s = String::from_utf8_lossy(&bytes);
    assert!(s.contains("612") && s.contains("792"), "Letter media box");
}


/// Writes a sample PDF when NVO_PDF_OUT is set (used to eyeball the layout).
#[test]
fn pdf_sample_to_file() {
    if let Ok(p) = std::env::var("NVO_PDF_OUT") {
        let data = sample::seed(now());
        let bytes = crate::pdf::render(&data.forms[0], "317.818.7777", now().date_naive()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
}
