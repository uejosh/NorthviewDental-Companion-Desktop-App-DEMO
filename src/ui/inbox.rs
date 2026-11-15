//! Booking requests: summary tiles, filters + search, pinned emergencies, request detail,
//! actions and activity log.

use super::{fmt_stamp, fmt_time, header, split, tiles_row, App, Dialog};
use crate::model::*;
use crate::theme::{self, c, font, txt, Btn, W};
use chrono::Utc;
use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, Id, Key, Rect, Sense, Stroke, StrokeKind};
use std::time::{Duration, Instant};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let now = Utc::now();
    let today = app.today();
    let reqs = app.data.requests.clone();

    header(ui, "Booking requests", "Appointment requests from the website. Book them in Eaglesoft, then mark them here.", |ui| {
        if app.live() {
            if theme::button(ui, if app.syncing { "Syncing…" } else { "Sync now" }, Btn::Outline, Some("⟳")).clicked() {
                app.request_sync();
            }
            let upd = match app.last_sync {
                Some(t) if (now - t).num_seconds() < 60 => "Updated just now".to_string(),
                Some(t) => format!("Updated {}", fmt_time(t)),
                None => "Not synced yet".into(),
            };
            ui.label(txt(upd, 12.5, W::Regular, c::FAINT));
        } else {
            let waiting = app.pending_sim.is_some();
            if theme::button_sized(ui, if waiting { "Request on its way…" } else { "Simulate a website request" }, Btn::Outline, Some("✉"), None, !waiting)
                .on_hover_text("Demo only: a new request arrives in about 5 seconds")
                .clicked()
            {
                app.pending_sim = Some(Instant::now() + Duration::from_secs(5));
                app.toast("A website request will arrive in about 5 seconds", true);
            }
        }
    });

    // tiles
    let new_n = count_status(&reqs, Filter::New);
    let em = reqs.iter().filter(|r| r.status == Status::New && r.emergency).count();
    let contacted = count_status(&reqs, Filter::Contacted);
    let sched = scheduled_this_week(&reqs, now);
    let avg = avg_first_response(&reqs, now).map(fmt_duration).unwrap_or_else(|| "—".into());
    let tw = tiles_row(ui, 4);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        theme::tile(ui, tw, "📥", c::BLUE, c::BLUE_BG, &new_n.to_string(), &if em > 0 { format!("New · {em} emergency") } else { "New".into() });
        theme::tile(ui, tw, "🕓", c::BLUE, c::BLUE_BG, &contacted.to_string(), "Awaiting reply");
        theme::tile(ui, tw, "📅", c::BLUE, c::BLUE_BG, &sched.to_string(), "Scheduled this week");
        theme::tile(ui, tw, "⚡", c::BLUE, c::BLUE_BG, &avg, "Avg. first response");
    });
    ui.add_space(20.0);

    // filters + search
    ui.horizontal(|ui| {
        let items: Vec<(String, Option<usize>)> = Filter::ALL
            .iter()
            .map(|f| (f.label().to_string(), if matches!(f, Filter::Open | Filter::New | Filter::Contacted) { Some(count_status(&reqs, *f)) } else { None }))
            .collect();
        let sel = Filter::ALL.iter().position(|f| *f == app.filter).unwrap_or(0);
        if let Some(i) = theme::segmented(ui, &items, sel) {
            app.filter = Filter::ALL[i];
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = theme::search_box(ui, "inbox-search", &mut app.query, "Search name, phone or reason", "Ctrl F", 330.0, 40.0, false);
            if app.focus_inbox_search {
                resp.request_focus();
                app.focus_inbox_search = false;
            }
            if !app.query.trim().is_empty() {
                let (p, r) = inbox(&reqs, app.filter, &app.query);
                let n = p.len() + r.len();
                ui.label(txt(format!("{n} {} for “{}”", if n == 1 { "match" } else { "matches" }, app.query.trim()), 12.5, W::Regular, c::FAINT));
            }
        });
    });
    ui.add_space(16.0);

    let (pinned, rest) = inbox(&reqs, app.filter, &app.query);
    let order: Vec<u64> = pinned.iter().chain(rest.iter()).map(|r| r.id).collect();
    if app.sel_req.is_none_or(|id| !reqs.iter().any(|r| r.id == id)) || (!order.is_empty() && app.sel_req.is_some_and(|id| !order.contains(&id)) && !app.query.is_empty()) {
        app.sel_req = order.first().copied();
    }
    // keyboard: J/K or arrows move through the list when no text field has focus
    let typing = ui.ctx().wants_keyboard_input();
    if !typing && app.dialog.is_none() {
        let pos = app.sel_req.and_then(|id| order.iter().position(|x| *x == id));
        if ui.input(|i| i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::J)) {
            if let Some(p) = pos.filter(|p| p + 1 < order.len()) {
                app.sel_req = Some(order[p + 1]);
            }
        }
        if ui.input(|i| i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::K)) {
            if let Some(p) = pos.filter(|p| *p > 0) {
                app.sel_req = Some(order[p - 1]);
            }
        }
    }

    let list_w = (ui.available_width() * 0.34).clamp(330.0, 430.0);
    let qry = app.query.clone();
    let [(mut lu, lr), (mut ru, rr)] = split(ui, list_w, 16.0);
    list(app, &mut lu, lr, &pinned, &rest, now, &qry);
    match app.sel_req.and_then(|id| reqs.iter().find(|x| x.id == id)) {
        Some(req) => {
            let pos = order.iter().position(|x| *x == req.id);
            detail(app, &mut ru, rr, req, pos, &order, today);
        }
        None => empty(&mut ru, rr, if reqs.is_empty() { "No requests yet. New website requests appear here automatically." } else { "Nothing matches this filter." }),
    }
}

fn highlight_job(name: &str, q: &str, f: egui::FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let q = q.trim().to_lowercase();
    let lower = name.to_lowercase();
    let fmt = |u: bool| egui::TextFormat {
        font_id: f.clone(),
        color: if u { c::BLUE } else { c::INK },
        underline: if u { Stroke::new(1.0, c::BLUE) } else { Stroke::NONE },
        ..Default::default()
    };
    if let (false, Some(i)) = (q.is_empty(), lower.find(&q)) {
        if name.is_char_boundary(i) && name.is_char_boundary(i + q.len()) {
            job.append(&name[..i], 0.0, fmt(false));
            job.append(&name[i..i + q.len()], 0.0, fmt(true));
            job.append(&name[i + q.len()..], 0.0, fmt(false));
            return job;
        }
    }
    job.append(name, 0.0, fmt(false));
    job
}

fn list(app: &mut App, ui: &mut egui::Ui, r: Rect, pinned: &[&Request], rest: &[&Request], now: chrono::DateTime<Utc>, q: &str) {
    theme::card_paint(ui.painter(), r);
    let inner = r.shrink(1.0);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    ui.set_clip_rect(inner);
    egui::ScrollArea::vertical().id_salt("inbox-list").auto_shrink([false, false]).show(&mut ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let w = inner.width();
        if !pinned.is_empty() {
            band(ui, w, "⚠  PINNED · EMERGENCY", "", c::RED, c::RED_ROW, true);
            for req in pinned {
                row(app, ui, w, req, now, q);
            }
        }
        let label = if q.trim().is_empty() { format!("{} requests", rest.len()) } else { format!("{} {}", rest.len(), if rest.len() == 1 { "match" } else { "matches" }) };
        band(ui, w, "NEWEST FIRST", &label, c::FAINT, c::BG, !pinned.is_empty());
        for req in rest {
            row(app, ui, w, req, now, q);
        }
        if pinned.is_empty() && rest.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| ui.label(txt("No requests here", 13.0, W::Regular, c::FAINT)));
        }
    });
}

fn band(ui: &mut egui::Ui, w: f32, left: &str, right: &str, fg: Color32, bg: Color32, top_line: bool) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 0, bg);
    if top_line {
        theme::hline(p, r.min.x, r.max.x, r.min.y);
    }
    theme::hline(p, r.min.x, r.max.x, r.max.y);
    p.text(pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, left, font(10.5, W::Semi), fg);
    p.text(pos2(r.max.x - 16.0, r.center().y), Align2::RIGHT_CENTER, right, font(11.0, W::Regular), c::FAINT);
}

fn row(app: &mut App, ui: &mut egui::Ui, w: f32, req: &Request, now: chrono::DateTime<Utc>, q: &str) {
    let (r, resp) = ui.allocate_exact_size(vec2(w, 78.0), Sense::click());
    let sel = app.sel_req == Some(req.id);
    let p = ui.painter();
    let bg = if req.pinned() {
        if sel { Color32::from_rgb(253, 238, 236) } else { c::RED_ROW }
    } else if sel {
        c::BLUE_BG
    } else if resp.hovered() {
        Color32::from_rgb(249, 251, 253)
    } else {
        c::WHITE
    };
    p.rect_filled(r, 0, bg);
    if sel || req.pinned() {
        p.rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0, if req.pinned() { c::RED_BRIGHT } else { c::BLUE });
    }
    theme::hline(p, r.min.x, r.max.x, r.max.y);
    let mut x = r.min.x + 16.0;
    let unseen = req.status == Status::New && !app.viewed.contains(&req.id) && (now - req.received_at).num_minutes() < 10;
    if unseen {
        theme::dot(p, pos2(x + 3.0, r.min.y + 23.0), 4.0, c::SKY);
        x += 14.0;
    }
    let job = highlight_job(&req.name(), q, font(14.5, W::Semi));
    let g = p.layout_job(job);
    p.galley(pos2(x, r.min.y + 13.0), g, c::INK);
    let a = ago(req.received_at, now);
    text_ago(p, pos2(r.max.x - 16.0, r.min.y + 15.0), &a);
    let mut cx = r.min.x + 16.0;
    let cy = r.min.y + 42.0;
    if req.emergency {
        cx += theme::chip_at(p, pos2(cx, cy), "Emergency", c::RED, c::RED_BG) + 6.0;
    }
    let (fg, bgc) = theme::status_colors(req.status);
    cx += theme::chip_at(p, pos2(cx, cy), req.status.label(), fg, bgc) + 8.0;
    theme::text_at(p, pos2(cx, cy + 1.0), &format!("{} · {}", req.patient_kind(), req.reason), font(12.5, W::Regular), c::MUTED, r.max.x - cx - 14.0);
    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
        app.sel_req = Some(req.id);
    }
}

fn text_ago(p: &egui::Painter, right_top: egui::Pos2, a: &str) {
    let color = if a == "just now" { c::BLUE } else { c::FAINT };
    p.text(right_top, Align2::RIGHT_TOP, a, font(12.0, if a == "just now" { W::Medium } else { W::Regular }), color);
}

pub fn empty(ui: &mut egui::Ui, r: Rect, msg: &str) {
    theme::card_paint(ui.painter(), r);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, msg, font(13.5, W::Regular), c::FAINT);
}

fn detail(app: &mut App, ui: &mut egui::Ui, r: Rect, req: &Request, pos: Option<usize>, order: &[u64], today: chrono::NaiveDate) {
    // audit: record each request view once per sign-in
    if app.viewed.insert(req.id) {
        app.act(Act::Viewed(req.id), "");
    }
    theme::card_paint(ui.painter(), r);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(r));
    ui.set_clip_rect(r);
    let ui = &mut ui;
    let w = r.width();
    let p = ui.painter().clone();

    // header
    let hr = Rect::from_min_size(r.min, vec2(w, 92.0));
    theme::avatar(&p, pos2(hr.min.x + 48.0, hr.center().y), 24.0, &req.initials(), if req.emergency { c::RED_BG } else { c::BLUE_BG }, if req.emergency { c::RED } else { c::BLUE });
    let name_r = p.text(pos2(hr.min.x + 86.0, hr.min.y + 22.0), Align2::LEFT_TOP, req.name(), font(20.0, W::Semi), c::INK);
    let mut cx = name_r.max.x + 10.0;
    if req.emergency {
        cx += theme::chip_at(&p, pos2(cx, name_r.min.y + 5.0), "Emergency", c::RED, c::RED_BG) + 6.0;
    }
    let (fg, bg) = theme::status_colors(req.status);
    theme::chip_at(&p, pos2(cx, name_r.min.y + 5.0), req.status.label(), fg, bg);
    theme::text_at(
        &p,
        pos2(hr.min.x + 86.0, hr.min.y + 53.0),
        &format!("{} · Received {} from the website form", req.reference, fmt_stamp(req.received_at)),
        font(12.5, W::Regular),
        c::MUTED,
        w - 300.0,
    );
    // prev / next
    if let Some(i) = pos {
        for (k, (glyph, target)) in [("⏶", i.checked_sub(1)), ("⏷", Some(i + 1).filter(|n| *n < order.len()))].into_iter().enumerate() {
            let br = Rect::from_min_size(pos2(hr.max.x - 98.0 + k as f32 * 46.0, hr.center().y - 17.0), vec2(34.0, 34.0));
            let resp = ui.interact(br, Id::new(("nav-req", k)), if target.is_some() { Sense::click() } else { Sense::hover() });
            p.rect_stroke(br, 8, Stroke::new(1.0, if resp.hovered() && target.is_some() { c::LINE_DARK } else { c::LINE }), StrokeKind::Inside);
            p.text(br.center(), Align2::CENTER_CENTER, glyph, font(13.0, W::Regular), if target.is_some() { c::INK } else { c::SUBTLE });
            if let Some(t) = target {
                if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    app.sel_req = Some(order[t]);
                }
            }
        }
        p.text(pos2(hr.max.x - 112.0, hr.center().y), Align2::RIGHT_CENTER, format!("{} of {}", i + 1, order.len()), font(12.5, W::Regular), c::MUTED);
    }
    theme::hline(&p, r.min.x, r.max.x, hr.max.y);
    let mut y = hr.max.y;

    // emergency banner
    if req.emergency && req.status.is_open() {
        let br = Rect::from_min_size(pos2(r.min.x, y), vec2(w, 56.0));
        p.rect_filled(br, 0, c::RED_BG);
        p.text(pos2(br.min.x + 24.0, br.center().y), Align2::LEFT_CENTER, "⚠  Emergency request. Call this patient first.", font(14.0, W::Semi), c::RED);
        if !req.phone.is_empty() {
            let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(br.max.x - 240.0, br.min.y + 9.0), pos2(br.max.x - 20.0, br.max.y - 9.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
            if theme::button(&mut bu, &format!("Call {}", req.phone), Btn::Danger, Some("☎")).clicked() {
                call(app, &req.phone);
            }
        }
        theme::hline(&p, r.min.x, r.max.x, br.max.y);
        y = br.max.y;
    }

    // actions
    let ar = Rect::from_min_size(pos2(r.min.x, y), vec2(w, 66.0));
    p.rect_filled(ar, 0, Color32::from_rgb(248, 250, 252));
    theme::hline(&p, r.min.x, r.max.x, ar.max.y);
    let mut au = ui.new_child(egui::UiBuilder::new().max_rect(ar.shrink2(vec2(24.0, 14.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    au.spacing_mut().item_spacing.x = 10.0;
    if theme::button(&mut au, "Copy for Eaglesoft", Btn::Outline, Some("📋")).on_hover_text("Copies the patient's details, ready to paste into Eaglesoft").clicked() {
        au.ctx().copy_text(req.eaglesoft_text(today));
        app.act(Act::Copied(req.id), "");
        app.toast("Copied. Paste into Eaglesoft.", true);
    }
    let open = req.status.is_open();
    if req.status == Status::New && theme::button(&mut au, "Mark contacted", Btn::Outline, Some("💬")).clicked() {
        app.dialog = Some(Dialog::Contacted { id: req.id, note: String::new() });
    } else if req.status == Status::Contacted && theme::button(&mut au, "Contacted again", Btn::Outline, Some("💬")).clicked() {
        app.dialog = Some(Dialog::Contacted { id: req.id, note: String::new() });
    }
    if open && theme::button_sized(&mut au, "Mark scheduled", Btn::Primary, Some("✔"), None, !app.busy).clicked() {
        app.dialog = Some(Dialog::Scheduled { id: req.id, date: req.appt_date.unwrap_or(today + chrono::Duration::days(1)), with_date: true });
    }
    if !open && theme::button_sized(&mut au, "Reopen", Btn::Outline, Some("↺"), None, !app.busy).clicked() {
        app.act(Act::SetStatus { id: req.id, status: Status::New, note: String::new(), appt: None }, &format!("{} reopened", req.name()));
    }
    au.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if open && theme::button(ui, "Decline…", Btn::DangerText, None).clicked() {
            app.dialog = Some(Dialog::Decline { id: req.id, reason: String::new() });
        }
        if app.busy {
            ui.spinner();
        }
    });
    y = ar.max.y;

    // body: details left, next step + activity right
    let body = Rect::from_min_max(pos2(r.min.x, y), r.max);
    let mut bu = ui.new_child(egui::UiBuilder::new().max_rect(body));
    bu.set_clip_rect(body.intersect(r));
    egui::ScrollArea::vertical().id_salt(("req-body", req.id)).auto_shrink([false, false]).show(&mut bu, |ui| {
        let bw = body.width() - 48.0;
        let right_w = (bw * 0.36).clamp(220.0, 300.0);
        let left_w = bw - right_w - 28.0;
        ui.add_space(22.0);
        ui.horizontal_top(|ui| {
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.set_width(left_w);
                theme::section_label(ui, "Request details");
                ui.add_space(6.0);
                let dob = match parse_date(&req.dob) {
                    Some(d) => format!("{}  ·  {} yrs", d.format("%b %-d, %Y"), age(d, today)),
                    None => req.dob.clone(),
                };
                let mut rows: Vec<(&str, String)> = vec![("Reason", req.reason.clone()), ("Patient", req.patient_kind().to_string())];
                kv(ui, left_w, "Reason", &rows[0].1);
                kv(ui, left_w, "Patient", &rows[1].1);
                // phone with click-to-call
                {
                    let (rr, _) = ui.allocate_exact_size(vec2(left_w, 36.0), Sense::hover());
                    let p = ui.painter();
                    p.text(pos2(rr.min.x, rr.center().y), Align2::LEFT_CENTER, "Phone", font(13.0, W::Regular), c::MUTED);
                    if req.phone.is_empty() {
                        p.text(pos2(rr.min.x + 130.0, rr.center().y), Align2::LEFT_CENTER, "—", font(13.5, W::Medium), c::FAINT);
                    } else {
                        let g = p.layout_no_wrap(req.phone.clone(), font(13.5, W::Medium), c::BLUE);
                        let lr = Rect::from_min_size(pos2(rr.min.x + 130.0, rr.center().y - g.size().y / 2.0), g.size());
                        p.galley(lr.min, g, c::BLUE);
                        p.line_segment([pos2(lr.min.x, lr.max.y), lr.max], Stroke::new(1.0, c::BLUE));
                        p.text(pos2(rr.max.x, rr.center().y), Align2::RIGHT_CENTER, "Click to call", font(11.5, W::Regular), c::FAINT);
                        let resp = ui.interact(lr.expand(4.0), Id::new(("call", req.id)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                        if resp.clicked() {
                            call(app, &req.phone);
                        }
                    }
                    theme::hline(ui.painter(), rr.min.x, rr.max.x, rr.max.y);
                }
                rows.clear();
                if !req.email.is_empty() {
                    let (rr, _) = ui.allocate_exact_size(vec2(left_w, 36.0), Sense::hover());
                    let p = ui.painter();
                    p.text(pos2(rr.min.x, rr.center().y), Align2::LEFT_CENTER, "Email", font(13.0, W::Regular), c::MUTED);
                    let lr = theme::text_at(p, pos2(rr.min.x + 130.0, rr.center().y - 10.0), &req.email, font(13.5, W::Medium), c::INK, left_w - 140.0);
                    if ui.interact(lr, Id::new(("mail", req.id)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand).on_hover_text("Open in your email app").clicked() {
                        let _ = open::that_detached(format!("mailto:{}", req.email));
                    }
                    theme::hline(ui.painter(), rr.min.x, rr.max.x, rr.max.y);
                }
                if !req.address.is_empty() {
                    kv(ui, left_w, "Address", &req.address);
                }
                if !req.dob.is_empty() {
                    kv(ui, left_w, "Date of birth", &dob);
                }
                kv(ui, left_w, "Preferred", &if req.preferred.is_empty() { "No preference".into() } else { req.preferred.join(", ") });
                if !req.insurance.is_empty() {
                    kv(ui, left_w, "Insurance", &req.insurance);
                }
                if let Some(d) = req.appt_date {
                    kv(ui, left_w, "Appointment", &d.format("%A, %b %-d, %Y").to_string());
                }
                ui.add_space(20.0);
                theme::section_label(ui, "Patient message");
                ui.add_space(4.0);
                let em = req.emergency && !req.message.is_empty();
                egui::Frame::new().fill(if em { c::RED_ROW } else { c::BG }).corner_radius(10).inner_margin(egui::Margin::symmetric(14, 12)).show(ui, |ui| {
                    ui.set_width(left_w - 28.0);
                    if req.message.trim().is_empty() {
                        ui.label(txt("No message provided.", 13.5, W::Regular, c::FAINT).italics());
                    } else {
                        ui.add(egui::Label::new(txt(&req.message, 13.5, W::Regular, c::INK)).wrap());
                    }
                });
                ui.add_space(24.0);
            });
            ui.add_space(28.0);
            ui.vertical(|ui| {
                ui.set_width(right_w);
                egui::Frame::new().fill(c::BLUE_BG).corner_radius(12).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
                    ui.set_width(right_w - 32.0);
                    ui.label(txt("Next step", 13.5, W::Semi, c::BLUE_DARK));
                    ui.add(egui::Label::new(txt(next_step(req), 12.5, W::Regular, c::MUTED)).wrap());
                });
                ui.add_space(22.0);
                theme::section_label(ui, "Activity");
                ui.add_space(6.0);
                let tl = req.timeline();
                let n = tl.len();
                for (i, a) in tl.iter().enumerate() {
                    let last = i + 1 == n;
                    let (rr, _) = ui.allocate_exact_size(vec2(right_w, if a.detail.is_empty() || a.action == "received" { 54.0 } else { 70.0 }), Sense::hover());
                    let p = ui.painter();
                    let dc = pos2(rr.min.x + 6.0, rr.min.y + 10.0);
                    if !last {
                        p.line_segment([pos2(dc.x, dc.y + 8.0), pos2(dc.x, rr.max.y + 2.0)], Stroke::new(1.0, c::LINE));
                    }
                    if last {
                        p.circle_stroke(dc, 4.5, Stroke::new(1.5, c::FAINT));
                    } else {
                        p.circle_filled(dc, 5.0, c::SKY);
                    }
                    theme::text_at(p, pos2(rr.min.x + 22.0, rr.min.y), &a.title(), font(13.0, W::Medium), c::INK, right_w - 24.0);
                    let mut yy = rr.min.y + 20.0;
                    if !a.detail.is_empty() && a.action != "received" {
                        theme::text_at(p, pos2(rr.min.x + 22.0, yy), &a.detail, font(12.0, W::Regular), c::MUTED, right_w - 24.0);
                        yy += 17.0;
                    }
                    let when = if a.action == "received" { format!("{} · {}", app.host_label(), fmt_stamp(a.at)) } else { fmt_stamp(a.at) };
                    p.text(pos2(rr.min.x + 22.0, yy), Align2::LEFT_TOP, when, font(11.0, W::Regular), c::FAINT);
                }
            });
        });
    });
}

fn kv(ui: &mut egui::Ui, w: f32, k: &str, v: &str) {
    let (rr, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
    let p = ui.painter();
    p.text(pos2(rr.min.x, rr.center().y), Align2::LEFT_CENTER, k, font(13.0, W::Regular), c::MUTED);
    theme::text_at(p, pos2(rr.min.x + 130.0, rr.center().y - 10.0), if v.is_empty() { "—" } else { v }, font(13.5, W::Medium), c::INK, w - 132.0);
    theme::hline(p, rr.min.x, rr.max.x, rr.max.y);
}

fn next_step(req: &Request) -> String {
    let first = req.first_name.trim();
    match (req.status, req.emergency) {
        (Status::New, true) => format!("Call {first} first, then book the first available emergency slot in Eaglesoft and press Mark scheduled."),
        (Status::New, false) => "Copy the details, create the patient and appointment in Eaglesoft, then press Mark scheduled.".into(),
        (Status::Contacted, _) => format!("Waiting to hear back from {first}. Once the visit is booked in Eaglesoft, press Mark scheduled."),
        (Status::Scheduled, _) => {
            if req.new_patient {
                "Booked. The new-patient form is tracked under Patient forms.".into()
            } else {
                "Booked in Eaglesoft. Nothing else to do.".into()
            }
        }
        (Status::Declined, _) => "Closed without booking. Reopen it if the patient gets back in touch.".into(),
    }
}

fn call(app: &mut App, phone: &str) {
    let digits = digits_of(phone);
    if open::that_detached(format!("tel:{digits}")).is_err() {
        app.toast("No calling app is set up on this computer", false);
    }
}

impl App {
    pub fn host_label(&self) -> String {
        if self.live() { self.host.clone() } else { "nvdindy.com".into() }
    }
}
