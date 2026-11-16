//! Modal dialogs: connection settings, request status changes, add member, log visit,
//! reset demo, and the damaged-data recovery screen.

use super::App;
use crate::model::*;
use crate::storage::Config;
use crate::theme::{self, c, txt, Btn, W};
use chrono::{Local, NaiveDate};
use eframe::egui::{self, vec2, Id};

pub enum Dialog {
    Connection { live: bool, url: String, key: String },
    Contacted { id: u64, note: String },
    Scheduled { id: u64, date: NaiveDate, with_date: bool },
    Decline { id: u64, reason: String },
    AddMember { name: String, plan: String, enrolled: NaiveDate, household: String, paid: bool, plan_touched: bool },
    LogVisit { name: String, phone: String, email: String, date: NaiveDate },
    ResetDemo,
}

impl Dialog {
    pub fn connection(cfg: &Config) -> Self {
        Dialog::Connection { live: cfg.mode == "live", url: cfg.site_url.clone(), key: cfg.device_key.clone() }
    }
    pub fn add_member() -> Self {
        Dialog::AddMember {
            name: String::new(),
            plan: "single".into(),
            enrolled: Local::now().date_naive(),
            household: String::new(),
            paid: true,
            plan_touched: false,
        }
    }
}

enum Out {
    Keep,
    Close,
}

fn frame(ctx: &egui::Context, id: &str, title: &str, sub: &str, body: impl FnOnce(&mut egui::Ui) -> Out) -> Out {
    let mut out = Out::Keep;
    let resp = egui::Modal::new(Id::new(id))
        .frame(egui::Frame::new().fill(c::WHITE).corner_radius(16).inner_margin(28).stroke(egui::Stroke::new(1.0, c::LINE)))
        .backdrop_color(egui::Color32::from_rgba_unmultiplied(15, 23, 42, 120))
        .show(ctx, |ui| {
            ui.set_width(440.0);
            ui.label(txt(title, 20.0, W::Semi, c::INK));
            if !sub.is_empty() {
                ui.add(egui::Label::new(txt(sub, 13.0, W::Regular, c::MUTED)).wrap());
            }
            ui.add_space(14.0);
            out = body(ui);
        });
    if resp.should_close() {
        return Out::Close;
    }
    out
}

fn buttons(ui: &mut egui::Ui, ok: &str, ok_kind: Btn, enabled: bool) -> (bool, bool) {
    ui.add_space(18.0);
    let mut res = (false, false);
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if theme::button_sized(ui, ok, ok_kind, None, None, enabled).clicked() {
                res.0 = true;
            }
            if theme::button(ui, "Cancel", Btn::Outline, None).clicked() {
                res.1 = true;
            }
        });
    });
    res
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut d) = app.dialog.take() else { return };
    let mut close = false;
    let mut todo: Option<(Act, String)> = None;
    let mut after: Option<Box<dyn FnOnce(&mut App)>> = None;
    let name_of = |id: u64| app.data.requests.iter().find(|r| r.id == id).map(|r| r.name()).unwrap_or_default();

    let out = match &mut d {
        Dialog::Connection { live, url, key } => frame(
            ctx,
            "dlg-conn",
            "Connection settings",
            "Connect this computer to the practice website, or use sample data for training and demos.",
            |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(live, true, txt("Website (live)", 13.5, W::Medium, c::INK));
                    ui.add_space(12.0);
                    ui.radio_value(live, false, txt("Demo data", 13.5, W::Medium, c::INK));
                });
                ui.add_space(10.0);
                if *live {
                    theme::field(ui, "Website address", url, "https://example.com", 440.0);
                    ui.add_space(8.0);
                    ui.label(txt("Device key", 12.5, W::Semi, c::INK));
                    ui.add(egui::TextEdit::singleline(key).password(true).hint_text("nvo_…").font(theme::font(13.5, W::Regular)).desired_width(440.0).margin(vec2(10.0, 8.0)));
                    ui.add(egui::Label::new(txt("Find both on the website under Tools → Northview Office App. Patient data from the website is never saved on this computer.", 12.0, W::Regular, c::MUTED)).wrap());
                } else {
                    ui.add(egui::Label::new(txt("Fictional patients stored on this computer. Any 4–6 digit PIN signs in.", 12.5, W::Regular, c::MUTED)).wrap());
                }
                let valid = !*live || (url.starts_with("https://") || url.starts_with("http://localhost")) && key.trim().len() > 10;
                let (ok, cancel) = buttons(ui, "Save and connect", Btn::Primary, valid);
                if ok {
                    let (l, u, k) = (*live, url.trim().trim_end_matches('/').to_string(), key.trim().to_string());
                    after = Some(Box::new(move |app: &mut App| {
                        app.cfg.mode = if l { "live".into() } else { "demo".into() };
                        if l {
                            app.cfg.site_url = u;
                            app.cfg.device_key = k;
                        }
                        if let Err(e) = app.storage.save_config(&app.cfg) {
                            app.toast(e, false);
                        }
                        app.connect();
                    }));
                    Out::Close
                } else if cancel {
                    Out::Close
                } else {
                    Out::Keep
                }
            },
        ),
        Dialog::Contacted { id, note } => {
            let id = *id;
            let who = name_of(id);
            frame(ctx, "dlg-contacted", "Mark contacted", &format!("Record how you reached {who}."), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for q in ["Called — spoke with patient", "Left voicemail", "Sent a text", "Sent an email"] {
                        let sel = note == q;
                        if ui.selectable_label(sel, txt(q, 12.5, W::Medium, if sel { c::BLUE } else { c::INK })).clicked() {
                            *note = q.to_string();
                        }
                    }
                });
                ui.add_space(8.0);
                theme::field(ui, "Note (optional)", note, "e.g. will call back Thursday", 440.0);
                let (ok, cancel) = buttons(ui, "Mark contacted", Btn::Primary, true);
                if ok {
                    todo = Some((Act::SetStatus { id, status: Status::Contacted, note: note.trim().to_string(), appt: None }, format!("{who} marked contacted")));
                    Out::Close
                } else if cancel {
                    Out::Close
                } else {
                    Out::Keep
                }
            })
        }
        Dialog::Scheduled { id, date, with_date } => {
            let id = *id;
            let who = name_of(id);
            frame(ctx, "dlg-sched", "Mark scheduled", &format!("Confirm {who}'s appointment is booked in Eaglesoft."), |ui| {
                ui.checkbox(with_date, txt("Add the appointment date", 13.5, W::Medium, c::INK));
                if *with_date {
                    ui.add_space(4.0);
                    ui.add(egui_extras::DatePickerButton::new(date).id_salt("appt-date").calendar_week(false));
                    ui.add(egui::Label::new(txt("The date lets the app chase the new-patient form and ask for a review after the visit.", 12.0, W::Regular, c::MUTED)).wrap());
                }
                let (ok, cancel) = buttons(ui, "Mark scheduled", Btn::Primary, true);
                if ok {
                    let appt = if *with_date { Some(*date) } else { None };
                    todo = Some((Act::SetStatus { id, status: Status::Scheduled, note: String::new(), appt }, format!("{who} marked scheduled")));
                    Out::Close
                } else if cancel {
                    Out::Close
                } else {
                    Out::Keep
                }
            })
        }
        Dialog::Decline { id, reason } => {
            let id = *id;
            let who = name_of(id);
            frame(ctx, "dlg-decline", "Decline request", &format!("Close {who}'s request without booking."), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for q in ["Duplicate", "Spam", "Service not offered", "Patient cancelled"] {
                        let sel = reason == q;
                        if ui.selectable_label(sel, txt(q, 12.5, W::Medium, if sel { c::BLUE } else { c::INK })).clicked() {
                            *reason = q.to_string();
                        }
                    }
                });
                ui.add_space(8.0);
                theme::field(ui, "Reason", reason, "Why is this request closed?", 440.0);
                let (ok, cancel) = buttons(ui, "Decline request", Btn::Danger, !reason.trim().is_empty());
                if ok {
                    todo = Some((Act::SetStatus { id, status: Status::Declined, note: reason.trim().to_string(), appt: None }, format!("{who}'s request declined")));
                    Out::Close
                } else if cancel {
                    Out::Close
                } else {
                    Out::Keep
                }
            })
        }
        Dialog::AddMember { name, plan, enrolled, household, paid, plan_touched } => frame(
            ctx,
            "dlg-member",
            "Add member",
            "Each membership runs one year from the enrolment date.",
            |ui| {
                theme::field(ui, "Membership name", name, "e.g. Moore family", 440.0);
                ui.add_space(8.0);
                ui.label(txt("Household members (one per line)", 12.5, W::Semi, c::INK));
                ui.add(egui::TextEdit::multiline(household).desired_rows(4).desired_width(440.0).font(theme::font(13.5, W::Regular)).hint_text("Leave blank for a single member"));
                let people: Vec<String> = household.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                let count = people.len().max(1);
                if !*plan_touched {
                    *plan = suggested_plan(count).to_string();
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(txt("Plan", 12.5, W::Semi, c::INK));
                        egui::ComboBox::from_id_salt("plan")
                            .selected_text(plan_label(plan))
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for (k, l, _, p) in PLANS {
                                    if ui.selectable_label(plan == k, format!("{l} — {}/yr", money(p))).clicked() {
                                        *plan = k.to_string();
                                        *plan_touched = true;
                                    }
                                }
                            });
                    });
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.label(txt("Enrolled", 12.5, W::Semi, c::INK));
                        ui.add(egui_extras::DatePickerButton::new(enrolled).id_salt("enrolled").calendar_week(false));
                    });
                });
                ui.add_space(6.0);
                ui.checkbox(paid, txt("First year paid", 13.0, W::Medium, c::INK));
                ui.add_space(6.0);
                // a plan covers at least its own size, even before every name is typed in
                let covered = count.max(plan_size(plan));
                ui.label(txt(
                    format!("{} {} · {} / yr · renews {}", covered, if covered == 1 { "person" } else { "people" }, money(plan_price(plan, count)), add_year(*enrolled).format("%b %-d, %Y")),
                    13.0,
                    W::Semi,
                    c::BLUE,
                ));
                let (ok, cancel) = buttons(ui, "Add member", Btn::Primary, !name.trim().is_empty());
                if ok {
                    todo = Some((
                        Act::MemberAdd { name: name.trim().into(), plan: plan.clone(), enrolled: *enrolled, household: people, paid: *paid },
                        "Membership added".into(),
                    ));
                    Out::Close
                } else if cancel {
                    Out::Close
                } else {
                    Out::Keep
                }
            },
        ),
        Dialog::LogVisit { name, phone, email, date } => frame(ctx, "dlg-visit", "Log a visit", "Add a recent visit so the patient can be asked for a review.", |ui| {
            theme::field(ui, "Patient name", name, "First and last name", 440.0);
            ui.add_space(6.0);
            theme::field(ui, "Mobile phone", phone, "(317) 555-0100", 440.0);
            ui.add_space(6.0);
            theme::field(ui, "Email", email, "name@example.com", 440.0);
            ui.add_space(6.0);
            ui.label(txt("Visit date", 12.5, W::Semi, c::INK));
            ui.add(egui_extras::DatePickerButton::new(date).id_salt("visit-date").calendar_week(false));
            let (ok, cancel) = buttons(ui, "Add visit", Btn::Primary, !name.trim().is_empty() && (!phone.trim().is_empty() || !email.trim().is_empty()));
            if ok {
                todo = Some((Act::VisitAdd { name: name.trim().into(), phone: phone.trim().into(), email: email.trim().into(), date: *date }, "Visit added".into()));
                Out::Close
            } else if cancel {
                Out::Close
            } else {
                Out::Keep
            }
        }),
        Dialog::ResetDemo => frame(ctx, "dlg-reset", "Reset demo?", "This restores the original sample patients and clears every change made during the demo.", |ui| {
            let (ok, cancel) = buttons(ui, "Reset demo", Btn::Danger, true);
            if ok {
                after = Some(Box::new(|app: &mut App| app.reset_demo()));
                Out::Close
            } else if cancel {
                Out::Close
            } else {
                Out::Keep
            }
        }),
    };
    if matches!(out, Out::Close) {
        close = true;
    }
    if !close {
        app.dialog = Some(d);
    }
    if let Some((a, m)) = todo {
        app.act(a, &m);
    }
    if let Some(f) = after {
        f(app);
    }
}

pub fn damaged(app: &mut App, ctx: &egui::Context, msg: &str) {
    egui::CentralPanel::default().frame(egui::Frame::new().fill(c::BG)).show(ctx, |ui| {
        let r = egui::Rect::from_center_size(ui.max_rect().center(), vec2(520.0, 320.0));
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(r));
        egui::Frame::new().fill(c::WHITE).corner_radius(16).inner_margin(32).stroke(egui::Stroke::new(1.0, c::LINE)).show(&mut ui, |ui| {
            ui.label(txt("The demo data file is damaged", 20.0, W::Semi, c::INK));
            ui.add_space(6.0);
            ui.add(egui::Label::new(txt(msg, 13.0, W::Regular, c::MUTED)).wrap());
            ui.add(egui::Label::new(txt(format!("The file has been left untouched at {}", app.storage.demo_path().display()), 12.0, W::Regular, c::FAINT)).wrap());
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if app.storage.has_backup() && theme::button(ui, "Restore backup", Btn::Primary, None).clicked() {
                    match app.storage.restore_backup() {
                        Ok(d) => {
                            app.data = d;
                            app.phase = super::Phase::SignIn;
                            app.toast("Backup restored", true);
                        }
                        Err(e) => app.toast(e, false),
                    }
                }
                if theme::button(ui, "Start fresh with sample data", Btn::Outline, None).clicked() {
                    app.storage.set_aside_damaged();
                    app.reset_demo();
                    app.phase = super::Phase::SignIn;
                }
            });
        });
    });
}
