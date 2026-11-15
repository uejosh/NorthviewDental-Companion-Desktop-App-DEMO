//! Sign-in / lock screen: pick a staff member, enter a 4–6 digit PIN.

use super::{App, Dialog};
use crate::api::Cmd;
use crate::theme::{self, c, font, txt, Btn, W};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, Id, Key, Rect, Sense, Stroke, StrokeKind};

pub fn show(app: &mut App, ctx: &egui::Context) {
    // PIN entry from the keyboard (no visible text field: the boxes are drawn).
    if app.dialog.is_none() && !app.signing_in {
        ctx.input(|i| {
            for e in &i.events {
                match e {
                    egui::Event::Text(t) => {
                        for ch in t.chars().filter(|c| c.is_ascii_digit()) {
                            if app.pin.len() < 6 {
                                app.pin.push(ch);
                                app.signin_err = None;
                            }
                        }
                    }
                    egui::Event::Key { key: Key::Backspace, pressed: true, .. } => {
                        app.pin.pop();
                    }
                    _ => {}
                }
            }
        });
    }
    let submit = app.dialog.is_none() && ctx.input(|i| i.key_pressed(Key::Enter));

    egui::CentralPanel::default().frame(egui::Frame::new().fill(c::WHITE)).show(ctx, |ui| {
        let full = ui.max_rect();
        let left = Rect::from_min_size(full.min, vec2((full.width() * 0.39).clamp(420.0, 620.0), full.height()));
        hero(app, ui, left);

        let right = Rect::from_min_max(pos2(left.max.x, full.min.y), full.max);
        let col_w = 400.0;
        let t = app.phase_t;
        let col = Rect::from_center_size(right.center(), vec2(col_w, 560.0_f32.min(right.height() - 40.0))).translate(vec2(0.0, (1.0 - t) * crate::motion::RISE * 1.5));
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(col).layout(egui::Layout::top_down(egui::Align::Min)));
        ui.set_opacity(t);
        let ui = &mut ui;
        ui.label(txt("Welcome back", 30.0, W::Semi, c::INK));
        ui.label(txt("Choose your name and enter your PIN to sign in.", 14.0, W::Regular, c::MUTED));
        ui.add_space(26.0);
        ui.label(txt("Staff member", 13.0, W::Semi, c::INK));
        ui.add_space(4.0);

        if app.staff_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(txt("Contacting the website…", 13.0, W::Regular, c::MUTED));
            });
        } else if app.staff.is_empty() {
            let msg = app.staff_err.clone().unwrap_or_else(|| "No staff accounts found.".into());
            notice(ui, "⚠", &msg, c::RED, c::RED_BG);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if theme::button(ui, "Try again", Btn::Small, Some("⟳")).clicked() {
                    app.connect();
                }
                if theme::button(ui, "Connection settings", Btn::Small, Some("⚙")).clicked() {
                    app.dialog = Some(Dialog::connection(&app.cfg));
                }
            });
        } else {
            let cw = (col_w - 12.0) / 2.0;
            let rows = app.staff.len().div_ceil(2);
            for row in 0..rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    for i in row * 2..(row * 2 + 2).min(app.staff.len()) {
                        let s = app.staff[i].clone();
                        let (r, resp) = ui.allocate_exact_size(vec2(cw, 68.0), Sense::click());
                        let sel = app.sel_staff == i;
                        let p = ui.painter();
                        p.rect_filled(r, 12, if sel { c::BLUE_BG } else { c::WHITE });
                        p.rect_stroke(r, 12, if sel { Stroke::new(2.0, c::BLUE) } else { Stroke::new(1.0, if resp.hovered() { c::LINE_DARK } else { c::LINE }) }, StrokeKind::Inside);
                        theme::avatar(p, pos2(r.min.x + 34.0, r.center().y), 19.0, &s.initials, if sel { c::BLUE } else { c::INK }, c::WHITE);
                        theme::text_at(p, pos2(r.min.x + 64.0, r.min.y + 15.0), &s.name, font(14.0, W::Semi), c::INK, if sel { cw - 104.0 } else { cw - 76.0 });
                        theme::text_at(p, pos2(r.min.x + 64.0, r.min.y + 36.0), &s.role, font(12.0, W::Regular), c::MUTED, cw - 100.0);
                        if sel {
                            let cc = pos2(r.max.x - 24.0, r.center().y);
                            p.circle_filled(cc, 10.0, c::BLUE);
                            p.text(cc, Align2::CENTER_CENTER, "✔", font(11.0, W::Bold), c::WHITE);
                        }
                        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            app.sel_staff = i;
                            app.signin_err = None;
                        }
                    }
                });
                ui.add_space(4.0);
            }
        }
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.label(txt("PIN", 13.0, W::Semi, c::INK));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(txt("4–6 digits", 12.0, W::Regular, c::FAINT));
            });
        });
        ui.add_space(4.0);
        // PIN boxes
        let (pr, presp) = ui.allocate_exact_size(vec2(col_w, 58.0), Sense::click());
        let bw = (col_w - 5.0 * 10.0) / 6.0;
        let p = ui.painter();
        let blink = (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
        for i in 0..6 {
            let br = Rect::from_min_size(pos2(pr.min.x + i as f32 * (bw + 10.0), pr.min.y), vec2(bw, 58.0));
            let active = i == app.pin.len();
            p.rect_filled(br, 10, if active { c::WHITE } else { Color32::from_rgb(234, 241, 247) });
            if active {
                p.rect_stroke(br, 10, Stroke::new(2.0, c::BLUE), StrokeKind::Inside);
                if blink {
                    p.line_segment([pos2(br.center().x, br.center().y - 11.0), pos2(br.center().x, br.center().y + 11.0)], Stroke::new(1.5, c::INK));
                }
            }
            if i < app.pin.len() {
                p.circle_filled(br.center(), 6.0, c::INK);
            }
        }
        if presp.clicked() {
            app.signin_err = None;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
        ui.add_space(22.0);

        let can = app.pin.len() >= 4 && !app.staff.is_empty() && !app.signing_in;
        let (br, bresp) = ui.allocate_exact_size(vec2(col_w, 50.0), if can { Sense::click() } else { Sense::hover() });
        let p = ui.painter();
        p.rect_filled(br, 25, if !can { c::BLUE.gamma_multiply(0.55) } else if bresp.hovered() { c::BLUE_DARK } else { c::BLUE });
        let label = if app.signing_in { "Signing in…" } else { "Sign in" };
        let g = p.layout_no_wrap(label.into(), font(15.5, W::Semi), c::WHITE);
        let kg = p.layout_no_wrap("Enter".into(), font(10.5, W::Medium), c::WHITE);
        let total = g.size().x + 12.0 + kg.size().x + 14.0;
        let x0 = br.center().x - total / 2.0;
        let gw = g.size().x;
        p.galley(pos2(x0, br.center().y - g.size().y / 2.0), g, c::WHITE);
        let kr = Rect::from_min_size(pos2(x0 + gw + 12.0, br.center().y - 10.0), vec2(kg.size().x + 14.0, 20.0));
        p.rect_filled(kr, 5, Color32::from_white_alpha(40));
        p.galley(pos2(kr.min.x + 7.0, kr.center().y - kg.size().y / 2.0), kg, c::WHITE);
        if can && (bresp.on_hover_cursor(CursorIcon::PointingHand).clicked() || submit) {
            sign_in(app);
        }

        ui.add_space(12.0);
        if let Some(e) = app.signin_err.clone() {
            notice(ui, "⚠", &e, c::RED, c::RED_BG);
            ui.add_space(8.0);
        }
        notice(ui, "🛡", "The app locks after 5 minutes without activity. Every action is recorded under your name for the HIPAA audit trail.", c::MUTED, c::BG);
        ui.add_space(18.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(txt(format!("Forgot your PIN? Ask the office manager to reset it.  ·  v{}  · ", super::VERSION), 12.0, W::Regular, c::FAINT));
            let conn = if app.live() { format!("Website: {}", app.host) } else { "Demo data".into() };
            if theme::link(ui, &conn, 12.0, c::BLUE).on_hover_text("Connection settings").clicked() {
                app.dialog = Some(Dialog::connection(&app.cfg));
            }
        });
        if !app.live() {
            ui.add_space(4.0);
            ui.label(txt("Demo mode: any 4–6 digit PIN works. Sample data only.", 11.5, W::Regular, c::FAINT));
        }
    });
}

fn sign_in(app: &mut App) {
    if app.live() {
        if let Some(s) = app.staff.get(app.sel_staff) {
            app.signing_in = true;
            app.client.send(Cmd::Login { user_id: s.id, pin: app.pin.clone() });
        }
    } else if let Some(s) = app.staff.get(app.sel_staff).cloned() {
        app.signed_in(s);
    }
}

fn hero(app: &App, ui: &mut egui::Ui, r: Rect) {
    let p = ui.painter_at(r);
    // cover-fit the background photo
    let ts = app.tex.bg.size_vec2();
    let scale = (r.width() / ts.x).max(r.height() / ts.y);
    let (uw, uh) = (r.width() / (ts.x * scale), r.height() / (ts.y * scale));
    let uv = Rect::from_min_size(pos2((1.0 - uw) / 2.0, (1.0 - uh) / 2.0), vec2(uw, uh));
    p.image(app.tex.bg.id(), r, uv, Color32::WHITE);
    p.rect_filled(r, 0, Color32::from_rgba_unmultiplied(33, 38, 51, 205));
    let mut p = p;
    p.multiply_opacity(app.phase_t);
    let lift = (1.0 - app.phase_t) * crate::motion::RISE;

    let ls = app.tex.logo.size_vec2();
    let lh = 46.0;
    p.image(
        app.tex.logo.id(),
        Rect::from_min_size(pos2(r.min.x + 62.0, r.min.y + 44.0), vec2(ls.x / ls.y * lh, lh)),
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::from_rgb(235, 238, 243),
    );

    let x = r.min.x + 48.0;
    let w = r.width() - 96.0;
    let mut y = r.max.y - 290.0 + lift;
    let pill = "Front desk · Office App";
    let g = p.layout_no_wrap(pill.into(), font(12.0, W::Medium), c::WHITE);
    let pr = Rect::from_min_size(pos2(x, y), vec2(g.size().x + 34.0, 26.0));
    p.rect_filled(pr, 13, Color32::from_white_alpha(18));
    p.rect_stroke(pr, 13, Stroke::new(1.0, Color32::from_white_alpha(70)), StrokeKind::Inside);
    p.circle_filled(pos2(pr.min.x + 14.0, pr.center().y), 3.0, c::SKY);
    p.galley(pos2(pr.min.x + 24.0, pr.center().y - g.size().y / 2.0), g, c::WHITE);
    y += 40.0;
    let head = p.layout("Everything from the website, ready for Eaglesoft.".into(), font(30.0, W::Semi), c::WHITE, w);
    let hh = head.size().y;
    p.galley(pos2(x, y), head, c::WHITE);
    y += hh + 16.0;
    let body = p.layout(
        "Booking requests, new-patient forms, Smile Savings Plan members and Google review requests — in one place, so nothing gets lost.".into(),
        font(14.0, W::Regular),
        Color32::from_rgb(220, 224, 230),
        w,
    );
    let bh = body.size().y;
    p.galley(pos2(x, y), body, c::WHITE);
    y += bh + 18.0;
    p.line_segment([pos2(x, y), pos2(x + w, y)], Stroke::new(1.0, Color32::from_white_alpha(50)));
    p.text(pos2(x, y + 14.0), Align2::LEFT_TOP, "210 East 91st Street, Suite A · Indianapolis, IN 46240 · 317.818.7777", font(11.5, W::Regular), Color32::from_rgb(190, 196, 205));
    let _ = Id::new("hero");
}

pub fn notice(ui: &mut egui::Ui, icon: &str, msg: &str, fg: Color32, bg: Color32) {
    egui::Frame::new().fill(bg).corner_radius(10).inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            ui.label(txt(icon, 15.0, W::Regular, if fg == c::MUTED { c::BLUE } else { fg }));
            ui.add_space(4.0);
            ui.add(egui::Label::new(txt(msg, 12.5, W::Regular, fg)).wrap());
        });
    });
}
