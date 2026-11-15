//! Fills assets/form.typ with a patient's form and compiles it to a Letter-size PDF.
//! Template, logo and fonts are compiled into the executable.

use crate::model::{age, parse_date, PatientForm};
use chrono::{Local, NaiveDate};
use typst::foundations::{Dict, IntoValue};
use typst_layout::PagedDocument;
use typst_as_lib::TypstEngine;

static TEMPLATE: &str = include_str!("../assets/form.typ");
static LOGO: &[u8] = include_bytes!("../assets/NorthViewDental-Logo-R.png");
static FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Poppins-Regular.ttf"),
    include_bytes!("../assets/fonts/Poppins-Medium.ttf"),
    include_bytes!("../assets/fonts/Poppins-SemiBold.ttf"),
    include_bytes!("../assets/fonts/Poppins-Bold.ttf"),
];

pub const PRACTICE_ADDRESS: &str = "210 East 91st Street, Suite A · Indianapolis, IN 46240";

fn pretty_date(s: &str, today: NaiveDate, with_age: bool) -> String {
    match parse_date(s) {
        Some(d) if with_age => format!("{} · {} yrs", d.format("%b %-d, %Y"), age(d, today)),
        Some(d) => d.format("%a, %b %-d, %Y").to_string(),
        None => s.to_string(),
    }
}

pub fn form_json(f: &PatientForm, phone: &str, today: NaiveDate) -> serde_json::Value {
    serde_json::json!({
        "name": f.name,
        "ref": f.reference,
        "submitted": f.submitted_at.with_timezone(&Local).format("%b %-d, %Y, %-I:%M %p").to_string(),
        "dob": pretty_date(&f.dob, today, true),
        "phone": f.phone,
        "email": f.email,
        "address": f.address,
        "visit": pretty_date(&f.visit_date, today, false),
        "conditions": f.conditions,
        "medications": f.medications,
        "allergies": f.allergies,
        "allergy": f.has_allergy(),
        "last_dental": f.last_dental,
        "carrier": f.carrier,
        "member_id": f.member_id,
        "signature": if f.signature.is_empty() { f.name.clone() } else { f.signature.clone() },
        "practice_address": PRACTICE_ADDRESS,
        "practice_phone": phone,
    })
}

pub fn render(f: &PatientForm, phone: &str, today: NaiveDate) -> Result<Vec<u8>, String> {
    let engine = TypstEngine::builder()
        .main_file(TEMPLATE)
        .fonts(FONTS)
        .with_static_file_resolver([("logo.png", LOGO)])
        .build();
    let mut inputs = Dict::new();
    inputs.insert("data".into(), form_json(f, phone, today).to_string().into_value());
    let doc: PagedDocument = engine
        .compile_with_input(inputs)
        .output
        .map_err(|e| format!("Could not lay out the form: {e:?}"))?;
    typst_pdf::pdf(&doc, &Default::default()).map_err(|e| format!("Could not create the PDF: {e:?}"))
}
