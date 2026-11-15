//! Google review requests: recent visits, 180-day cooldown, message preview, sent log.

use super::{fmt_stamp, header, split, tiles_row, App, Dialog};
use crate::model::*;
use crate::theme::{self, c, font, txt, Btn, W};
use chrono::{Duration, Utc};
use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let today = app.today();
    let now = Utc::now();
    header(ui, "Google review requests", "Send a review link after a visit. Patients asked in the last 180 days are skipped automatically.", |ui| {
        if theme::button(ui, "Open Google profile", Btn::Outline, Some("↗")).clicked() {
            let url = app.data.settings.google_profile_url.clone();
            if url.is_empty() {
                app.toast("The Google profile link isn't set on the website yet", false);
            } else {
                let _ = open::that_detached(url);
            }
        }
        if theme::button(ui, "Log a visit", Btn::Outline, Some("+")).clicked() {
            app.dialog = Some(Dialog::LogVisit { name: String::new(), phone: String::new(), email: String::new(), date: today });
        }
    });

    let visits: Vec<Visit> = app.data.recent_visits(today).into_iter().cloned().collect();
    let ready = visits.iter().filter(|v| !v.asked_recently(now)).count();
    let skipped = visits.len() - ready;
    let week = app.data.review_log.iter().filter(|l| now - l.at < Duration::days(7)).count();
    let tw = tiles_row(ui, 4);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        theme::tile(ui, tw, "➡", c::BLUE, c::BLUE_BG, &ready.to_string(), "Ready to ask");
        theme::tile(ui, tw, "💬", c::BLUE, c::BLUE_BG, &week.to_string(), "Requests sent this week");
        theme::tile(ui, tw, "🕓", c::BLUE, c::BLUE_BG, &skipped.to_string(), "Skipped · asked < 180 days");
        theme::tile(ui, tw, "☆", c::AMBER_DOT, c::AMBER_BG, &format!("{} ★", app.data.settings.google_rating), "Google rating");
    });
    ui.add_space(20.0);

    let left_w = ui.available_width() - 16.0 - (ui.available_width() * 0.3).clamp(320.0, 400.0);
    let [(mut lu, lr), (mut ru, rr)] = split(ui, left_w, 16.0);

    // recent visits table
    theme::card_paint(lu.painter(), lr);
    let p = lu.painter().clone();
    let t = p.text(pos2(lr.min.x + 20.0, lr.min.y + 28.0), Align2::LEFT_CENTER, "Recent visits", font(15.5, W::Semi), c::INK);
    p.text(pos2(t.max.x + 12.0, t.center().y), Align2::LEFT_CENTER, "Last 7 days", font(12.0, W::Regular), c::FAINT);
    let th = Rect::from_min_size(pos2(lr.min.x + 1.0, lr.min.y + 56.0), vec2(lr.width() - 2.0, 34.0));
    p.rect_filled(th, 0, c::BG);
    theme::hline(&p, lr.min.x, lr.max.x, th.min.y);
    theme::hline(&p, lr.min.x, lr.max.x, th.max.y);
    let x0 = lr.min.x + 20.0;
    let w = lr.width() - 40.0;
    let col = |f: f32| x0 + w * f;
    for (f, h) in [(0.0, "PATIENT"), (0.5, "VISIT"), (0.66, "LAST ASKED")] {
        p.text(pos2(col(f), th.center().y), Align2::LEFT_CENTER, h, font(10.5, W::Semi), c::FAINT);
    }
    p.text(pos2(lr.max.x - 20.0, th.center().y), Align2::RIGHT_CENTER, "SEND REQUEST", font(10.5, W::Semi), c::FAINT);
    let body = Rect::from_min_max(pos2(lr.min.x + 1.0, th.max.y + 1.0), lr.max - vec2(1.0, 1.0));
    let mut bu = lu.new_child(egui::UiBuilder::new().max_rect(body));
    bu.set_clip_rect(body);
    let mut todo: Option<(Act, String)> = None;
    egui::ScrollArea::vertical().id_salt("visits").auto_shrink([false, false]).show(&mut bu, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if visits.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| ui.label(txt("No visits in the last 7 days. Scheduled requests with a date appear here after the visit, or use Log a visit.", 13.0, W::Regular, c::FAINT)));
        }
        for v in &visits {
            let (rr, _) = ui.allocate_exact_size(vec2(body.width(), 64.0), Sense::hover());
            let p = ui.painter();
            theme::hline(p, rr.min.x, rr.max.x, rr.max.y);
            let top = rr.min.y + 13.0;
            theme::text_at(p, pos2(col(0.0), top), &v.name, font(13.5, W::Semi), c::INK, w * 0.48);
            let contact = [v.phone.as_str(), v.email.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
            theme::text_at(p, pos2(col(0.0), top + 21.0), &contact, font(12.0, W::Regular), c::MUTED, w * 0.48);
            p.text(pos2(col(0.5), rr.center().y), Align2::LEFT_CENTER, v.visit_date.format("%b %-d, %Y").to_string(), font(13.0, W::Regular), c::INK);
            let last = match v.last_asked {
                None => ("Never".to_string(), c::FAINT),
                Some(t) if t.with_timezone(&chrono::Local).date_naive() == today => ("Today".into(), c::INK),
                Some(t) => (format!("{} days ago", (now - t).num_days().max(1)), c::INK),
            };
            p.text(pos2(col(0.66), rr.center().y), Align2::LEFT_CENTER, last.0, font(13.0, W::Regular), last.1);
            let ar = Rect::from_min_max(pos2(col(0.78), rr.min.y + 14.0), pos2(rr.max.x - 20.0, rr.max.y - 14.0));
            let mut au = ui.new_child(egui::UiBuilder::new().max_rect(ar).layout(egui::Layout::right_to_left(egui::Align::Center)));
            au.spacing_mut().item_spacing.x = 8.0;
            if v.asked_recently(now) {
                theme::chip(&mut au, "✔ Asked recently", c::MUTED, c::BG);
            } else {
                let sim = if app.data.settings.messaging_live { "" } else { " (simulated)" };
                if theme::button_sized(&mut au, "Email", Btn::Small, None, None, !v.email.is_empty() && !app.busy).clicked() {
                    todo = Some((Act::Review { visit_id: v.id, channel: "email".into() }, format!("Review request sent to {} by email{sim}", v.name)));
                }
                if theme::button_sized(&mut au, "Text", Btn::SmallPrimary, None, None, !v.phone.is_empty() && !app.busy).clicked() {
                    todo = Some((Act::Review { visit_id: v.id, channel: "text".into() }, format!("Review request sent to {} by text{sim}", v.name)));
                }
            }
        }
    });
    if let Some((a, m)) = todo {
        app.act(a, &m);
    }

    // right column: preview + sent log
    let prev_h = 290.0;
    let pr = Rect::from_min_size(rr.min, vec2(rr.width(), prev_h));
    theme::card_paint(ru.painter(), pr);
    let p = ru.painter().clone();
    p.text(pos2(pr.min.x + 20.0, pr.min.y + 30.0), Align2::LEFT_CENTER, "Message preview", font(15.5, W::Semi), c::INK);
    let mut tu = ru.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(pr.max.x - 160.0, pr.min.y + 10.0), pos2(pr.max.x - 16.0, pr.min.y + 50.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
    let items = vec![("Text".to_string(), None), ("Email".to_string(), None)];
    if let Some(i) = theme::segmented(&mut tu, &items, if app.preview_email { 1 } else { 0 }) {
        app.preview_email = i == 1;
    }
    let msg = review_message("{first name}", &app.data.settings.review_link, app.preview_email);
    let bubble = Rect::from_min_max(pos2(pr.min.x + 20.0, pr.min.y + 62.0), pos2(pr.max.x - 20.0, pr.max.y - 44.0));
    p.rect_filled(bubble, 12, c::BG);
    p.text(pos2(bubble.min.x + 14.0, bubble.min.y + 10.0), Align2::LEFT_TOP, format!("Northview Dental · {}", app.data.settings.practice_phone), font(10.5, W::Regular), c::FAINT);
    let inner = Rect::from_min_max(pos2(bubble.min.x + 14.0, bubble.min.y + 30.0), pos2(bubble.max.x - 24.0, bubble.max.y - 10.0));
    p.rect_filled(inner, 12, c::WHITE);
    p.rect_stroke(inner, 12, egui::Stroke::new(1.0, c::LINE), egui::StrokeKind::Inside);
    let mut mu = ru.new_child(egui::UiBuilder::new().max_rect(inner.shrink2(vec2(14.0, 10.0))));
    mu.set_clip_rect(inner.shrink(2.0));
    egui::ScrollArea::vertical().id_salt("msg-prev").auto_shrink([false, false]).show(&mut mu, |ui| {
        ui.add(egui::Label::new(txt(&msg, 12.5, W::Regular, c::INK)).wrap());
    });
    let chars = msg.chars().count();
    let foot = if app.preview_email { "Includes opt-out line · sent from the practice email".to_string() } else { format!("Includes opt-out line · {chars} characters · {} SMS", sms_segments(chars)) };
    p.text(pos2(pr.min.x + 20.0, pr.max.y - 22.0), Align2::LEFT_CENTER, format!("🛡 {foot}"), font(11.5, W::Regular), c::MUTED);

    // sent log
    let sr = Rect::from_min_max(pos2(rr.min.x, pr.max.y + 16.0), rr.max);
    theme::card_paint(ru.painter(), sr);
    p.text(pos2(sr.min.x + 20.0, sr.min.y + 30.0), Align2::LEFT_CENTER, "Sent", font(15.5, W::Semi), c::INK);
    let mut vu = ru.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(sr.max.x - 120.0, sr.min.y + 18.0), pos2(sr.max.x - 20.0, sr.min.y + 42.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
    if app.data.review_log.len() > 4 && theme::link(&mut vu, if app.show_all_sent { "Show less" } else { "View all" }, 12.5, c::BLUE).clicked() {
        app.show_all_sent = !app.show_all_sent;
    }
    let lb = Rect::from_min_max(pos2(sr.min.x + 20.0, sr.min.y + 56.0), sr.max - vec2(20.0, 8.0));
    let mut lu2 = ru.new_child(egui::UiBuilder::new().max_rect(lb));
    lu2.set_clip_rect(lb);
    let log: Vec<ReviewLog> = app.data.review_log.iter().take(if app.show_all_sent { 100 } else { 4 }).cloned().collect();
    egui::ScrollArea::vertical().id_salt("sent").auto_shrink([false, false]).show(&mut lu2, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if log.is_empty() {
            ui.label(txt("Nothing sent yet.", 12.5, W::Regular, c::FAINT));
        }
        for l in &log {
            let (r, _) = ui.allocate_exact_size(vec2(lb.width(), 58.0), Sense::hover());
            let p = ui.painter();
            theme::hline(p, r.min.x, r.max.x, r.min.y);
            let ic = pos2(r.min.x + 14.0, r.center().y);
            p.circle_filled(ic, 14.0, c::BLUE_BG);
            p.text(ic, Align2::CENTER_CENTER, if l.channel == "email" { "✉" } else { "💬" }, font(12.0, W::Regular), c::BLUE);
            theme::text_at(p, pos2(r.min.x + 40.0, r.min.y + 10.0), &format!("{} · {}", l.name, if l.channel == "email" { "Email" } else { "Text" }), font(13.0, W::Medium), c::INK, lb.width() - 44.0);
            p.text(pos2(r.min.x + 40.0, r.min.y + 31.0), Align2::LEFT_TOP, format!("{} · {}", l.by, fmt_stamp(l.at)), font(11.0, W::Regular), c::FAINT);
        }
    });
    let _ = Color32::WHITE;
}
