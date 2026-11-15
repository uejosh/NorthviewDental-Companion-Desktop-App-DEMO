//! Patient forms: upcoming new patients (form received / missing), received forms,
//! a printable form view, Save as PDF and Mark attached in Eaglesoft.

use super::{fmt_stamp, header, split, App};
use crate::model::*;
use crate::pdf;
use crate::theme::{self, c, font, txt, Btn, W};
use chrono::Utc;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, Rect, Sense, Stroke, StrokeKind};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let today = app.today();
    let now = Utc::now();
    header(ui, "Patient forms", "New-patient forms completed online or on the office tablet, signed on screen.", |ui| {
        if theme::button(ui, "Copy tablet link", Btn::Outline, Some("🔗")).clicked() {
            let url = if app.data.settings.patient_form_url.is_empty() {
                if app.live() { String::new() } else { "https://nvdindy.com/new-patient-form/".into() }
            } else {
                app.data.settings.patient_form_url.clone()
            };
            if url.is_empty() {
                app.toast("The new-patient form page isn't set on the website yet", false);
            } else {
                ui.ctx().copy_text(url);
                app.toast("Tablet link copied", true);
            }
        }
    });

    // upcoming new patients
    let ups = upcoming_new_patients(&app.data, today);
    let missing = ups.iter().filter(|u| !u.form_received).count();
    let card_w = ui.available_width();
    let per_row = 3usize;
    let rows = ups.len().div_ceil(per_row).max(1);
    let card_h = 62.0 + rows as f32 * 72.0 + 12.0;
    let (cr, _) = ui.allocate_exact_size(vec2(card_w, card_h), Sense::hover());
    theme::card_paint(ui.painter(), cr);
    let p = ui.painter().clone();
    let t = p.text(pos2(cr.min.x + 20.0, cr.min.y + 20.0), Align2::LEFT_TOP, "Upcoming new patients", font(15.5, W::Semi), c::INK);
    p.text(
        pos2(t.max.x + 12.0, t.center().y),
        Align2::LEFT_CENTER,
        format!("Next 2 days · {}", if missing == 0 { "all forms in".to_string() } else { format!("{missing} form{} missing", if missing == 1 { "" } else { "s" }) }),
        font(12.5, W::Regular),
        c::FAINT,
    );
    if ups.is_empty() {
        p.text(pos2(cr.min.x + 20.0, cr.min.y + 72.0), Align2::LEFT_TOP, "No new patients booked for the next two days. Booking requests marked scheduled with a date show up here.", font(13.0, W::Regular), c::MUTED);
    }
    let gap = 14.0;
    let iw = (card_w - 40.0 - gap * (per_row as f32 - 1.0)) / per_row as f32;
    for (i, u) in ups.iter().enumerate() {
        let (row, col) = (i / per_row, i % per_row);
        let ir = Rect::from_min_size(pos2(cr.min.x + 20.0 + col as f32 * (iw + gap), cr.min.y + 58.0 + row as f32 * 72.0), vec2(iw, 62.0));
        let miss = !u.form_received;
        p.rect_filled(ir, 10, if miss { c::AMBER_ROW } else { c::BG });
        if miss {
            p.rect_stroke(ir, 10, Stroke::new(1.0, c::AMBER_LINE), StrokeKind::Inside);
        }
        theme::dot(&p, pos2(ir.min.x + 18.0, ir.center().y), 4.5, if miss { c::AMBER_DOT } else { c::GREEN });
        theme::text_at(&p, pos2(ir.min.x + 34.0, ir.min.y + 12.0), &u.up.name, font(13.5, W::Semi), c::INK, iw - 190.0);
        let when = if u.up.visit_date == today { "Visit today".to_string() } else { format!("Visit {}", u.up.visit_date.format("%a, %b %-d")) };
        p.text(pos2(ir.min.x + 34.0, ir.min.y + 33.0), Align2::LEFT_TOP, when, font(12.0, W::Regular), if miss { c::AMBER } else { c::MUTED });
        if miss {
            let br = Rect::from_min_max(pos2(ir.max.x - 160.0, ir.min.y + 14.0), pos2(ir.max.x - 12.0, ir.max.y - 14.0));
            let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(br).layout(egui::Layout::right_to_left(egui::Align::Center)));
            if u.up.link_sent {
                bu.label(txt("Link sent ✔", 12.5, W::Medium, c::AMBER));
            } else if theme::button(&mut bu, "Send form link", Btn::SmallPrimary, Some("✉")).on_hover_text(format!("Texts and emails {} a link to the form", u.up.name)).clicked() {
                let msg = format!("Form link sent to {}{}", u.up.name, if app.data.settings.messaging_live { "" } else { " (simulated)" });
                app.act(Act::FormLink { request_id: u.up.request_id, name: u.up.name.clone() }, &msg);
            }
        } else {
            p.text(pos2(ir.max.x - 14.0, ir.center().y), Align2::RIGHT_CENTER, "Form received", font(12.5, W::Medium), c::GREEN);
        }
    }
    ui.add_space(20.0);

    if app.live() && !app.data.settings.patient_forms_on {
        super::signin::notice(
            ui,
            "ℹ",
            "The online new-patient form isn't connected on the website yet. Medical history should only be collected once hosting is covered by a HIPAA Business Associate Agreement; then choose the form under Tools → Northview Office App on the website.",
            c::MUTED,
            c::WHITE,
        );
        return;
    }

    let mut forms = app.data.forms.clone();
    forms.sort_by(|a, b| a.attached().cmp(&b.attached()).then(b.submitted_at.cmp(&a.submitted_at)));
    if app.sel_form.is_none_or(|id| !forms.iter().any(|f| f.id == id)) {
        app.sel_form = forms.first().map(|f| f.id);
    }
    let list_w = (ui.available_width() * 0.34).clamp(320.0, 420.0);
    let [(mut lu, lr), (mut ru, rr)] = split(ui, list_w, 16.0);

    // list
    theme::card_paint(lu.painter(), lr);
    let lp = lu.painter().clone();
    let band = Rect::from_min_size(lr.min + vec2(1.0, 1.0), vec2(lr.width() - 2.0, 34.0));
    lp.rect_filled(band, egui::CornerRadius { nw: 12, ne: 12, sw: 0, se: 0 }, c::BG);
    theme::hline(&lp, lr.min.x, lr.max.x, band.max.y);
    lp.text(pos2(band.min.x + 16.0, band.center().y), Align2::LEFT_CENTER, "RECEIVED FORMS", font(10.5, W::Semi), c::FAINT);
    lp.text(pos2(band.max.x - 16.0, band.center().y), Align2::RIGHT_CENTER, format!("{} to attach", app.data.forms_to_attach()), font(11.0, W::Regular), c::FAINT);
    let inner = Rect::from_min_max(pos2(lr.min.x + 1.0, band.max.y + 1.0), lr.max - vec2(1.0, 1.0));
    let mut li = lu.new_child(egui::UiBuilder::new().max_rect(inner));
    li.set_clip_rect(inner);
    egui::ScrollArea::vertical().id_salt("forms-list").auto_shrink([false, false]).show(&mut li, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if forms.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| ui.label(txt("No forms received yet", 13.0, W::Regular, c::FAINT)));
        }
        for f in &forms {
            let (r, resp) = ui.allocate_exact_size(vec2(inner.width(), 78.0), Sense::click());
            let sel = app.sel_form == Some(f.id);
            let p = ui.painter();
            p.rect_filled(r, 0, if sel { c::BLUE_BG } else if resp.hovered() { Color32::from_rgb(249, 251, 253) } else { c::WHITE });
            if sel {
                p.rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0, c::BLUE);
            }
            theme::hline(p, r.min.x, r.max.x, r.max.y);
            theme::text_at(p, pos2(r.min.x + 16.0, r.min.y + 14.0), &f.name, font(14.5, W::Semi), c::INK, r.width() - 120.0);
            p.text(pos2(r.max.x - 16.0, r.min.y + 16.0), Align2::RIGHT_TOP, ago(f.submitted_at, now), font(12.0, W::Regular), c::FAINT);
            let (l, fg, bg) = if f.attached() { ("In Eaglesoft", c::GREEN, c::GREEN_BG) } else { ("To attach", c::AMBER, c::AMBER_BG) };
            let cw = theme::chip_at(p, pos2(r.min.x + 16.0, r.min.y + 44.0), l, fg, bg);
            let visit = f.visit().map(|d| format!("Visit {}", d.format("%b %-d, %Y"))).unwrap_or_else(|| f.visit_date.clone());
            theme::text_at(p, pos2(r.min.x + 24.0 + cw, r.min.y + 45.0), &format!("{visit} · {}", f.reference), font(12.5, W::Regular), c::MUTED, r.width() - cw - 40.0);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                app.sel_form = Some(f.id);
            }
        }
    });

    // viewer
    let Some(f) = app.sel_form.and_then(|id| forms.iter().find(|f| f.id == id)).cloned() else {
        super::inbox::empty(&mut ru, rr, "Select a form to view it");
        return;
    };
    viewer(app, &mut ru, rr, &f);
}

fn viewer(app: &mut App, ui: &mut egui::Ui, r: Rect, f: &PatientForm) {
    let today = app.today();
    theme::card_paint(ui.painter(), r);
    let p = ui.painter().clone();
    let hr = Rect::from_min_size(r.min, vec2(r.width(), 64.0));
    theme::text_at(&p, pos2(hr.min.x + 20.0, hr.min.y + 14.0), &f.pdf_name(), font(14.0, W::Semi), c::INK, r.width() - 440.0);
    p.text(pos2(hr.min.x + 20.0, hr.min.y + 36.0), Align2::LEFT_TOP, "Letter · signed electronically", font(11.5, W::Regular), c::FAINT);
    let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(hr.max.x - 430.0, hr.min.y + 12.0), pos2(hr.max.x - 16.0, hr.max.y - 12.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
    bu.spacing_mut().item_spacing.x = 10.0;
    if f.attached() {
        bu.label(txt(format!("✔ In Eaglesoft · {}{}", if f.attached_by.is_empty() { String::new() } else { format!("{} · ", f.attached_by) }, f.attached_at.map(fmt_stamp).unwrap_or_default()), 12.5, W::Medium, c::GREEN));
    } else if theme::button_sized(&mut bu, "Mark attached in Eaglesoft", Btn::Primary, Some("✔"), None, !app.busy).clicked() {
        app.act(Act::FormAttached(f.id), &format!("{}'s form marked attached in Eaglesoft", f.name));
    }
    if theme::button(&mut bu, "Save as PDF", Btn::Outline, Some("⬇")).clicked() {
        save_pdf(app, f);
    }
    theme::hline(&p, r.min.x, r.max.x, hr.max.y);

    let body = Rect::from_min_max(pos2(r.min.x + 1.0, hr.max.y + 1.0), r.max - vec2(1.0, 1.0));
    p.rect_filled(body, egui::CornerRadius { nw: 0, ne: 0, sw: 12, se: 12 }, Color32::from_rgb(237, 241, 245));
    let mut vu = ui.new_child(egui::UiBuilder::new().max_rect(body));
    vu.set_clip_rect(body);
    let jump = std::mem::take(&mut app.jump_medical);
    let mut clicked_jump = false;
    egui::ScrollArea::vertical().id_salt(("form-doc", f.id)).auto_shrink([false, false]).show(&mut vu, |ui| {
        let page_w = (body.width() - 80.0).min(660.0);
        let pad = (body.width() - page_w) / 2.0;
        ui.add_space(22.0);
        if f.has_allergy() {
            ui.horizontal(|ui| {
                ui.add_space(pad);
                egui::Frame::new().fill(c::RED_BG).stroke(Stroke::new(1.0, c::RED_LINE)).corner_radius(10).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
                    ui.set_width(page_w - 32.0);
                    ui.horizontal(|ui| {
                        ui.label(txt(format!("⚠  Allergy on file: {}", f.allergies), 13.5, W::Semi, c::RED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if theme::link(ui, "Jump to medical history ⬇", 12.5, c::RED).clicked() {
                                clicked_jump = true;
                            }
                        });
                    });
                });
            });
            ui.add_space(12.0);
        }
        ui.horizontal(|ui| {
            ui.add_space(pad);
            egui::Frame::new()
                .fill(c::WHITE)
                .corner_radius(4)
                .inner_margin(egui::Margin::symmetric(40, 36))
                .shadow(egui::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(18) })
                .show(ui, |ui| ui.vertical(|ui| {
                    ui.set_width(page_w - 80.0);
                    let w = page_w - 80.0;
                    ui.allocate_ui_with_layout(vec2(w, 40.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        ui.vertical(|ui| {
                            ui.label(txt("Northview Dental", 16.0, W::Semi, c::INK));
                            ui.label(txt(format!("{} · {}", pdf::PRACTICE_ADDRESS, app.data.settings.practice_phone), 10.5, W::Regular, c::MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.vertical(|ui| {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                                    ui.label(txt(&f.reference, 12.0, W::Semi, c::INK));
                                    ui.label(txt(format!("Submitted {}", fmt_stamp(f.submitted_at)), 10.5, W::Regular, c::MUTED));
                                });
                            });
                        });
                    });
                    ui.add_space(8.0);
                    let (lr, _) = ui.allocate_exact_size(vec2(w, 2.0), Sense::hover());
                    ui.painter().rect_filled(lr, 0, c::INK);
                    ui.add_space(18.0);
                    ui.label(txt("New Patient Form", 20.0, W::Semi, c::INK));
                    ui.add_space(10.0);
                    sec(ui, "Patient");
                    let dob = parse_date(&f.dob).map(|d| format!("{} · {} yrs", d.format("%b %-d, %Y"), age(d, today))).unwrap_or_else(|| f.dob.clone());
                    row(ui, w, "Name", &f.name, false);
                    row(ui, w, "Date of birth", &dob, false);
                    row(ui, w, "Phone", &f.phone, false);
                    row(ui, w, "Email", &f.email, false);
                    row(ui, w, "Address", &f.address, false);
                    row(ui, w, "First visit", &f.visit().map(|d| d.format("%b %-d, %Y").to_string()).unwrap_or_else(|| f.visit_date.clone()), false);
                    ui.add_space(14.0);
                    let med = sec(ui, "Medical history");
                    if jump {
                        med.scroll_to_me(Some(egui::Align::Min));
                    }
                    row(ui, w, "Conditions", &f.conditions, false);
                    row(ui, w, "Medications", &f.medications, false);
                    row(ui, w, "Allergies", &f.allergies, f.has_allergy());
                    row(ui, w, "Last dental visit", &f.last_dental, false);
                    ui.add_space(14.0);
                    sec(ui, "Insurance");
                    row(ui, w, "Carrier", &f.carrier, false);
                    row(ui, w, "Member ID", &f.member_id, false);
                    ui.add_space(14.0);
                    sec(ui, "Signature");
                    ui.add_space(6.0);
                    ui.label(txt(if f.signature.is_empty() { &f.name } else { &f.signature }, 20.0, W::Medium, c::INK).italics());
                    let (sl, _) = ui.allocate_exact_size(vec2(260.0, 1.0), Sense::hover());
                    ui.painter().rect_filled(sl, 0, c::MUTED);
                    ui.label(txt(format!("Signed electronically · {}", fmt_stamp(f.submitted_at)), 10.5, W::Regular, c::MUTED));
                }));
        });
        ui.add_space(30.0);
    });
    if clicked_jump {
        app.jump_medical = true;
    }
}

fn sec(ui: &mut egui::Ui, s: &str) -> egui::Response {
    ui.label(txt(s.to_uppercase(), 10.5, W::Semi, c::BLUE).extra_letter_spacing(0.8))
}

fn row(ui: &mut egui::Ui, w: f32, k: &str, v: &str, warn: bool) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 30.0), Sense::hover());
    let p = ui.painter();
    if warn {
        p.rect_filled(r.expand2(vec2(4.0, -2.0)), 6, c::RED_BG);
    }
    p.text(pos2(r.min.x + 4.0, r.center().y), Align2::LEFT_CENTER, k, font(12.5, W::Regular), if warn { c::RED } else { c::MUTED });
    let val = if v.trim().is_empty() { "—".to_string() } else if warn { format!("⚠ {v}") } else { v.to_string() };
    theme::text_at(p, pos2(r.min.x + 150.0, r.center().y - 9.0), &val, font(13.0, if warn { W::Semi } else { W::Medium }), if warn { c::RED } else { c::INK }, w - 154.0);
    if !warn {
        theme::hline(p, r.min.x, r.max.x, r.max.y);
    }
}

fn save_pdf(app: &mut App, f: &PatientForm) {
    let name = f.pdf_name();
    let start = dirs::document_dir().or_else(dirs::home_dir).unwrap_or_default();
    let Some(path) = rfd::FileDialog::new().set_title("Save new-patient form").set_directory(start).set_file_name(&name).add_filter("PDF", &["pdf"]).save_file() else {
        return;
    };
    match pdf::render(f, &app.data.settings.practice_phone, app.today()).and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| format!("Could not save the PDF: {e}"))) {
        Ok(()) => {
            app.act(Act::FormPdf(f.id), "");
            app.toast(format!("Saved {}", path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(name)), true);
        }
        Err(e) => app.toast(e, false),
    }
}
