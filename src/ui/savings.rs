//! Smile Savings Plan register: tiles, members table sorted by renewal, renew / remind / mark paid.

use super::{header, tiles_row, App, Dialog};
use crate::model::*;
use crate::theme::{self, c, font, txt, Btn, W};
use eframe::egui::{self, pos2, vec2, Align2, Rect, Sense};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let today = app.today();
    header(ui, "Smile Savings Plan", "In-house membership for patients without dental insurance. Each membership runs one year from enrolment.", |ui| {
        if theme::button(ui, "Add member", Btn::Primary, Some("+")).clicked() {
            app.dialog = Some(Dialog::add_member());
        }
    });

    let ms = app.data.members_sorted().into_iter().cloned().collect::<Vec<_>>();
    let people: usize = ms.iter().map(|m| m.people()).sum();
    let soon = ms.iter().filter(|m| m.renewing_soon(today)).count();
    let due = ms.iter().filter(|m| m.payment_due()).count();
    let value: u32 = ms.iter().map(|m| m.yearly()).sum();
    let tw = tiles_row(ui, 4);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        theme::tile(ui, tw, "👥", c::BLUE, c::BLUE_BG, &ms.len().to_string(), &format!("Memberships · {people} people"));
        theme::tile(ui, tw, "⟳", c::AMBER, c::AMBER_BG, &soon.to_string(), "Renewing within 30 days");
        theme::tile(ui, tw, "💳", c::RED, c::RED_BG, &due.to_string(), "Payment due");
        theme::tile(ui, tw, "$", c::BLUE, c::BLUE_BG, &money(value), "Annual plan value");
    });
    ui.add_space(20.0);

    let r = ui.available_rect_before_wrap();
    ui.allocate_rect(r, Sense::hover());
    theme::card_paint(ui.painter(), r);
    let mut cu = ui.new_child(egui::UiBuilder::new().max_rect(r));
    cu.set_clip_rect(r);
    let p = cu.painter().clone();

    // card header
    let hr = Rect::from_min_size(r.min, vec2(r.width(), 64.0));
    let t = p.text(pos2(hr.min.x + 20.0, hr.center().y), Align2::LEFT_CENTER, "Members", font(15.5, W::Semi), c::INK);
    p.text(pos2(t.max.x + 12.0, hr.center().y), Align2::LEFT_CENTER, "Soonest renewal first", font(12.0, W::Regular), c::FAINT);
    let mut hu = cu.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(hr.max.x - 640.0, hr.min.y + 12.0), pos2(hr.max.x - 16.0, hr.max.y - 12.0))).layout(egui::Layout::right_to_left(egui::Align::Center)));
    hu.spacing_mut().item_spacing.x = 12.0;
    theme::search_box(&mut hu, "member-search", &mut app.mem_query, "Search members", "", 230.0, 38.0, false);
    hu.add_space(6.0);
    let items = vec![(format!("All {}", ms.len()), None), ("Renewing soon".to_string(), Some(soon)), ("Payment due".to_string(), Some(due))];
    if let Some(i) = theme::segmented(&mut hu, &items, app.mem_filter) {
        app.mem_filter = i;
    }
    theme::hline(&p, r.min.x, r.max.x, hr.max.y);

    // columns
    let x0 = r.min.x + 20.0;
    let w = r.width() - 40.0;
    let cols = [0.0, 0.25, 0.37, 0.43, 0.55, 0.69, 0.78];
    let cx = |i: usize| x0 + w * cols[i];
    let th = Rect::from_min_size(pos2(r.min.x, hr.max.y), vec2(r.width(), 34.0));
    p.rect_filled(th, 0, c::BG);
    for (i, h) in ["MEMBERSHIP", "PLAN", "PEOPLE", "ENROLLED", "RENEWS ⬇", "PAYMENT"].iter().enumerate() {
        p.text(pos2(cx(i), th.center().y), Align2::LEFT_CENTER, *h, font(10.5, W::Semi), if i == 4 { c::BLUE } else { c::FAINT });
    }
    theme::hline(&p, r.min.x, r.max.x, th.max.y);

    let footer_h = 48.0;
    let body = Rect::from_min_max(pos2(r.min.x + 1.0, th.max.y + 1.0), pos2(r.max.x - 1.0, r.max.y - footer_h));
    let mut bu = cu.new_child(egui::UiBuilder::new().max_rect(body));
    bu.set_clip_rect(body);
    let shown: Vec<&Member> = ms
        .iter()
        .filter(|m| m.matches(&app.mem_query))
        .filter(|m| match app.mem_filter {
            1 => m.renewing_soon(today),
            2 => m.payment_due(),
            _ => true,
        })
        .collect();
    let mut todo: Option<(Act, String)> = None;
    egui::ScrollArea::vertical().id_salt("members").auto_shrink([false, false]).show(&mut bu, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if shown.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| ui.label(txt(if ms.is_empty() { "No members yet. Use Add member to enrol the first household." } else { "No members match." }, 13.0, W::Regular, c::FAINT)));
        }
        for m in shown {
            let (rr, _) = ui.allocate_exact_size(vec2(body.width(), 64.0), Sense::hover());
            let days = m.days_to_renewal(today);
            let soon = m.renewing_soon(today);
            let p = ui.painter();
            if soon {
                p.rect_filled(rr, 0, c::AMBER_ROW);
            }
            theme::hline(p, rr.min.x, rr.max.x, rr.max.y);
            let top = rr.min.y + 14.0;
            theme::text_at(p, pos2(cx(0), top), &m.name, font(13.5, W::Semi), c::INK, w * 0.24);
            theme::text_at(p, pos2(cx(0), top + 21.0), &m.household_text(), font(12.0, W::Regular), c::MUTED, w * 0.24);
            p.text(pos2(cx(1), top), Align2::LEFT_TOP, plan_label(&m.plan), font(13.0, W::Medium), c::INK);
            p.text(pos2(cx(1), top + 21.0), Align2::LEFT_TOP, format!("{} / yr", money(m.yearly())), font(12.0, W::Regular), c::MUTED);
            p.text(pos2(cx(2), rr.center().y), Align2::LEFT_CENTER, m.people().to_string(), font(13.0, W::Medium), c::INK);
            p.text(pos2(cx(3), rr.center().y), Align2::LEFT_CENTER, m.enrolled.format("%b %-d, %Y").to_string(), font(13.0, W::Regular), c::INK);
            p.text(pos2(cx(4), top), Align2::LEFT_TOP, m.renews.format("%b %-d, %Y").to_string(), font(13.0, W::Medium), c::INK);
            p.text(pos2(cx(4), top + 21.0), Align2::LEFT_TOP, renewal_countdown(days), font(12.0, if soon { W::Semi } else { W::Regular }), if soon { c::AMBER } else { c::FAINT });
            let (pl, fg, bg) = if m.payment_due() { ("Due", c::RED, c::RED_BG) } else { ("Paid", c::GREEN, c::GREEN_BG) };
            theme::chip_at(p, pos2(cx(5), rr.center().y - 10.0), pl, fg, bg);

            let ar = Rect::from_min_max(pos2(cx(6), rr.min.y + 14.0), pos2(rr.max.x - 20.0, rr.max.y - 14.0));
            let mut au = ui.new_child(egui::UiBuilder::new().max_rect(ar).layout(egui::Layout::right_to_left(egui::Align::Center)));
            au.spacing_mut().item_spacing.x = 8.0;
            if soon {
                let reminded = m.last_reminder.is_some_and(|t| (chrono::Utc::now() - t).num_days() < 14);
                if reminded {
                    au.label(txt("Reminder sent ✔", 12.0, W::Medium, c::AMBER));
                } else if theme::button(&mut au, "Send reminder", Btn::Small, None).clicked() {
                    let sim = if app.data.settings.messaging_live { "" } else { " (simulated)" };
                    todo = Some((Act::Member { id: m.id, op: MemberOp::Remind }, format!("Renewal reminder sent to {}{sim}", m.name)));
                }
                if theme::button(&mut au, "Renew", Btn::Small, None).on_hover_text("Extend one year and mark the new year's payment due").clicked() {
                    todo = Some((Act::Member { id: m.id, op: MemberOp::Renew }, format!("{} renewed to {}", m.name, add_year(m.renews).format("%b %-d, %Y"))));
                }
            }
            if m.payment_due() && theme::button(&mut au, "Mark paid", Btn::SmallPrimary, None).clicked() {
                todo = Some((Act::Member { id: m.id, op: MemberOp::Paid }, format!("{} marked paid", m.name)));
            }
        }
    });
    if let Some((a, msg)) = todo {
        app.act(a, &msg);
    }

    let fr = Rect::from_min_max(pos2(r.min.x, r.max.y - footer_h), r.max);
    p.rect_filled(fr.shrink(1.0), egui::CornerRadius { nw: 0, ne: 0, sw: 12, se: 12 }, c::BG);
    theme::hline(&p, r.min.x, r.max.x, fr.min.y);
    let t = p.text(pos2(fr.min.x + 20.0, fr.center().y), Align2::LEFT_CENTER, "ℹ  Prices match the website plan:", font(12.5, W::Semi), c::INK);
    let prices = PLANS.iter().map(|(_, l, _, pr)| format!("{l} {}", money(*pr))).collect::<Vec<_>>().join("  ·  ");
    theme::text_at(&p, pos2(t.max.x + 10.0, fr.center().y - 9.0), &format!("{prices}  ·  Each additional member {}", money(EXTRA_MEMBER_PRICE)), font(12.5, W::Regular), c::MUTED, fr.max.x - t.max.x - 30.0);
}
