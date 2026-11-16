//! Brand colours, bundled fonts and the custom widgets used across screens
//! (buttons, chips, tiles, cards, avatars, segmented filters, search boxes).

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId,
    Galley, Painter, Pos2, Rect, Response, RichText, Sense, Shadow, Stroke, StrokeKind, Ui,
};
use std::sync::Arc;

pub mod c {
    use eframe::egui::Color32;
    pub const INK: Color32 = Color32::from_rgb(33, 38, 51);
    pub const MUTED: Color32 = Color32::from_rgb(95, 102, 115);
    pub const FAINT: Color32 = Color32::from_rgb(138, 145, 158);
    pub const SUBTLE: Color32 = Color32::from_rgb(174, 180, 192);
    pub const LINE: Color32 = Color32::from_rgb(221, 228, 234);
    pub const LINE_DARK: Color32 = Color32::from_rgb(201, 206, 216);
    pub const BG: Color32 = Color32::from_rgb(243, 246, 250);
    pub const WHITE: Color32 = Color32::WHITE;
    pub const BLUE: Color32 = Color32::from_rgb(6, 111, 173);
    pub const BLUE_DARK: Color32 = Color32::from_rgb(5, 90, 142);
    pub const BLUE_BG: Color32 = Color32::from_rgb(231, 243, 249);
    pub const SKY: Color32 = Color32::from_rgb(22, 148, 220);
    pub const RED: Color32 = Color32::from_rgb(180, 35, 24);
    pub const RED_BRIGHT: Color32 = Color32::from_rgb(217, 45, 32);
    pub const RED_BG: Color32 = Color32::from_rgb(252, 236, 236);
    pub const RED_LINE: Color32 = Color32::from_rgb(244, 199, 195);
    pub const RED_ROW: Color32 = Color32::from_rgb(255, 247, 246);
    pub const AMBER: Color32 = Color32::from_rgb(148, 96, 10);
    pub const AMBER_DOT: Color32 = Color32::from_rgb(231, 155, 12);
    pub const AMBER_BG: Color32 = Color32::from_rgb(255, 244, 220);
    pub const AMBER_ROW: Color32 = Color32::from_rgb(255, 250, 241);
    pub const AMBER_LINE: Color32 = Color32::from_rgb(244, 217, 163);
    pub const GREEN: Color32 = Color32::from_rgb(20, 120, 74);
    pub const GREEN_DOT: Color32 = Color32::from_rgb(60, 203, 127);
    pub const GREEN_BG: Color32 = Color32::from_rgb(230, 246, 238);
    pub const SIDEBAR: Color32 = Color32::from_rgb(48, 54, 68);
    pub const SIDEBAR_HI: Color32 = Color32::from_rgb(64, 71, 87);
    pub const TITLEBAR: Color32 = Color32::from_rgb(33, 38, 51);
    pub const TOAST: Color32 = Color32::from_rgb(30, 35, 48);
}

#[derive(Clone, Copy, PartialEq)]
pub enum W {
    Regular,
    Medium,
    Semi,
    Bold,
}

pub fn font(size: f32, w: W) -> FontId {
    let fam = match w {
        W::Regular => FontFamily::Proportional,
        W::Medium => FontFamily::Name("medium".into()),
        W::Semi => FontFamily::Name("semibold".into()),
        W::Bold => FontFamily::Name("bold".into()),
    };
    FontId::new(size, fam)
}

pub fn txt(s: impl Into<String>, size: f32, w: W, color: Color32) -> RichText {
    RichText::new(s).font(font(size, w)).color(color)
}

pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let fonts: [(&str, &'static [u8]); 4] = [
        ("poppins-regular", include_bytes!("../assets/fonts/Poppins-Regular.ttf")),
        ("poppins-medium", include_bytes!("../assets/fonts/Poppins-Medium.ttf")),
        ("poppins-semibold", include_bytes!("../assets/fonts/Poppins-SemiBold.ttf")),
        ("poppins-bold", include_bytes!("../assets/fonts/Poppins-Bold.ttf")),
    ];
    for (name, bytes) in fonts {
        defs.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    let fallback: Vec<String> = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let fam = |main: &str| {
        let mut v = vec![main.to_string()];
        v.extend(fallback.iter().cloned());
        v
    };
    defs.families.insert(FontFamily::Proportional, fam("poppins-regular"));
    defs.families.insert(FontFamily::Name("medium".into()), fam("poppins-medium"));
    defs.families.insert(FontFamily::Name("semibold".into()), fam("poppins-semibold"));
    defs.families.insert(FontFamily::Name("bold".into()), fam("poppins-bold"));
    ctx.set_fonts(defs);

    ctx.style_mut(|s| {
        s.text_styles.insert(egui::TextStyle::Body, font(14.0, W::Regular));
        s.text_styles.insert(egui::TextStyle::Button, font(14.0, W::Medium));
        s.text_styles.insert(egui::TextStyle::Small, font(12.0, W::Regular));
        s.text_styles.insert(egui::TextStyle::Heading, font(22.0, W::Semi));
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(14.0, 7.0);
        s.spacing.interact_size.y = 34.0;
        let v = &mut s.visuals;
        *v = egui::Visuals::light();
        v.panel_fill = c::BG;
        v.window_fill = c::WHITE;
        v.extreme_bg_color = c::WHITE;
        v.override_text_color = Some(c::INK);
        v.hyperlink_color = c::BLUE;
        v.selection.bg_fill = c::BLUE_BG;
        v.selection.stroke = Stroke::new(1.5, c::BLUE);
        v.window_corner_radius = CornerRadius::same(14);
        v.window_stroke = Stroke::new(1.0, c::LINE);
        v.window_shadow = Shadow { offset: [0, 12], blur: 40, spread: 0, color: Color32::from_black_alpha(45) };
        v.popup_shadow = Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(30) };
        v.widgets.inactive.corner_radius = CornerRadius::same(8);
        v.widgets.hovered.corner_radius = CornerRadius::same(8);
        v.widgets.active.corner_radius = CornerRadius::same(8);
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, c::LINE);
        v.widgets.inactive.weak_bg_fill = c::WHITE;
        v.widgets.inactive.bg_fill = c::WHITE;
        v.widgets.hovered.weak_bg_fill = c::BG;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, c::LINE_DARK);
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c::LINE);
        v.text_cursor.stroke = Stroke::new(2.0, c::BLUE);
    });
}

// ------------------------------------------------------------------ painter helpers

pub fn galley(ui: &Ui, s: &str, f: FontId, color: Color32) -> Arc<Galley> {
    ui.painter().layout_no_wrap(s.to_string(), f, color)
}

/// One line of text, cut with an ellipsis if wider than `max_w`. Returns its rect.
pub fn text_at(p: &Painter, pos: Pos2, s: &str, f: FontId, color: Color32, max_w: f32) -> Rect {
    let mut job = egui::text::LayoutJob::simple_singleline(s.to_string(), f, color);
    job.wrap.max_width = max_w.max(10.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let g = p.layout_job(job);
    let r = Rect::from_min_size(pos, g.size());
    p.galley(pos, g, color);
    r
}

pub fn card_paint(p: &Painter, r: Rect) {
    p.rect_filled(r, 12, c::WHITE);
    p.rect_stroke(r, 12, Stroke::new(1.0, c::LINE), StrokeKind::Inside);
}

/// Chip drawn at `pos` (top-left). Returns its width.
pub fn chip_at(p: &Painter, pos: Pos2, s: &str, fg: Color32, bg: Color32) -> f32 {
    let g = p.layout_no_wrap(s.to_string(), font(11.5, W::Semi), fg);
    let r = Rect::from_min_size(pos, vec2(g.size().x + 16.0, 20.0));
    p.rect_filled(r, 10, bg);
    p.galley(pos2(r.min.x + 8.0, r.center().y - g.size().y / 2.0), g, fg);
    r.width()
}

pub fn chip(ui: &mut Ui, s: &str, fg: Color32, bg: Color32) -> Response {
    let g = galley(ui, s, font(11.5, W::Semi), fg);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 16.0, 20.0), Sense::hover());
    ui.painter().rect_filled(r, 10, bg);
    ui.painter().galley(pos2(r.min.x + 8.0, r.center().y - g.size().y / 2.0), g, fg);
    resp
}

pub fn status_colors(s: crate::model::Status) -> (Color32, Color32) {
    use crate::model::Status::*;
    match s {
        New => (c::AMBER, c::AMBER_BG),
        Contacted => (c::BLUE, c::BLUE_BG),
        Scheduled => (c::GREEN, c::GREEN_BG),
        Declined => (c::MUTED, c::BG),
    }
}

pub fn avatar(p: &Painter, center: Pos2, radius: f32, initials: &str, bg: Color32, fg: Color32) {
    p.circle_filled(center, radius, bg);
    p.text(center, Align2::CENTER_CENTER, initials, font(radius * 0.72, W::Semi), fg);
}

pub fn dot(p: &Painter, center: Pos2, r: f32, color: Color32) {
    p.circle_filled(center, r, color);
}

// ------------------------------------------------------------------ buttons

#[derive(Clone, Copy, PartialEq)]
pub enum Btn {
    Primary,
    Outline,
    DangerText,
    Danger,
    Small,
    SmallPrimary,
}

pub fn button(ui: &mut Ui, label: &str, kind: Btn, icon: Option<&str>) -> Response {
    button_sized(ui, label, kind, icon, None, true)
}

fn button_font(kind: Btn) -> FontId {
    let small = matches!(kind, Btn::Small | Btn::SmallPrimary);
    font(if small { 13.0 } else { 14.0 }, if matches!(kind, Btn::Primary | Btn::SmallPrimary | Btn::Danger) { W::Semi } else { W::Medium })
}

/// Width `button` will take for this label, so rows can pick a layout that fits before drawing.
pub fn button_width(ui: &Ui, label: &str, kind: Btn, icon: Option<&str>) -> f32 {
    let small = matches!(kind, Btn::Small | Btn::SmallPrimary);
    let text = match icon {
        Some(i) => format!("{i}  {label}"),
        None => label.to_string(),
    };
    ui.painter().layout_no_wrap(text, button_font(kind), c::INK).size().x + if small { 24.0 } else { 34.0 }
}

pub fn button_sized(ui: &mut Ui, label: &str, kind: Btn, icon: Option<&str>, width: Option<f32>, enabled: bool) -> Response {
    let small = matches!(kind, Btn::Small | Btn::SmallPrimary);
    let h = if small { 30.0 } else { 38.0 };
    let f = button_font(kind);
    let (fg, bg, stroke) = match kind {
        Btn::Primary | Btn::SmallPrimary => (c::WHITE, c::BLUE, None),
        Btn::Outline | Btn::Small => (c::INK, c::WHITE, Some(c::LINE)),
        Btn::DangerText => (c::RED, Color32::TRANSPARENT, None),
        Btn::Danger => (c::WHITE, c::RED, None),
    };
    let text = match icon {
        Some(i) => format!("{i}  {label}"),
        None => label.to_string(),
    };
    let g = ui.painter().layout_no_wrap(text, f, fg);
    let w = width.unwrap_or(g.size().x + if small { 24.0 } else { 34.0 });
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (full, resp) = ui.allocate_exact_size(vec2(w, h), sense);
    let hovered = enabled && resp.hovered();
    let ht = ui.ctx().animate_bool_with_time(resp.id.with("hover"), hovered, crate::motion::HOVER_SECS);
    let pt = ui.ctx().animate_bool_with_time(resp.id.with("press"), enabled && resp.is_pointer_button_down_on(), 0.06);
    let r = full.shrink(pt * 1.5);
    let hover_bg = match kind {
        Btn::Primary | Btn::SmallPrimary => c::BLUE_DARK,
        Btn::Outline | Btn::Small => c::BG,
        Btn::DangerText => c::RED_BG,
        Btn::Danger => Color32::from_rgb(150, 28, 20),
    };
    let bg = if !enabled {
        if bg == Color32::TRANSPARENT { bg } else { bg.gamma_multiply(0.45) }
    } else if bg == Color32::TRANSPARENT {
        hover_bg.gamma_multiply(ht)
    } else {
        bg.lerp_to_gamma(hover_bg, ht)
    };
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same((h / 2.0) as u8), bg);
    if let Some(s) = stroke {
        p.rect_stroke(r, CornerRadius::same((h / 2.0) as u8), Stroke::new(1.0, s.lerp_to_gamma(c::LINE_DARK, ht)), StrokeKind::Inside);
    }
    let gs = g.size();
    let color = if enabled { fg } else { fg.gamma_multiply(0.7) };
    p.galley_with_override_text_color(pos2(r.center().x - gs.x / 2.0, r.center().y - gs.y / 2.0), g, color);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// A text link (underlined on hover).
pub fn link(ui: &mut Ui, s: &str, size: f32, color: Color32) -> Response {
    let g = galley(ui, s, font(size, W::Medium), color);
    let (r, resp) = ui.allocate_exact_size(g.size(), Sense::click());
    let hovered = resp.hovered();
    ui.painter().galley(r.min, g, color);
    if hovered {
        ui.painter().line_segment([pos2(r.min.x, r.max.y), r.max], Stroke::new(1.0, color));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ------------------------------------------------------------------ composites

/// Summary tile: icon in a tinted circle, big value, label underneath.
pub fn tile(ui: &mut Ui, w: f32, icon: &str, icon_fg: Color32, icon_bg: Color32, value: &str, label: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 84.0), Sense::hover());
    let p = ui.painter();
    card_paint(p, r);
    let cc = pos2(r.min.x + 40.0, r.center().y);
    p.circle_filled(cc, 20.0, icon_bg);
    p.text(cc, Align2::CENTER_CENTER, icon, font(17.0, W::Regular), icon_fg);
    p.text(pos2(r.min.x + 74.0, r.min.y + 18.0), Align2::LEFT_TOP, value, font(22.0, W::Semi), c::INK);
    text_at(p, pos2(r.min.x + 74.0, r.min.y + 50.0), label, font(12.5, W::Regular), c::MUTED, w - 84.0);
}

/// Pill-shaped segmented control. Returns the clicked index.
pub fn segmented(ui: &mut Ui, items: &[(String, Option<usize>)], selected: usize) -> Option<usize> {
    let pad = 4.0;
    let mut widths = Vec::new();
    for (l, n) in items {
        let gw = galley(ui, l, font(13.5, W::Medium), c::INK).size().x;
        let nw = n.filter(|n| *n > 0).map(|n| galley(ui, &n.to_string(), font(11.0, W::Semi), c::INK).size().x + 14.0).unwrap_or(0.0);
        widths.push(gw + nw + 30.0);
    }
    let total: f32 = widths.iter().sum::<f32>() + pad * 2.0;
    let (r, _) = ui.allocate_exact_size(vec2(total, 40.0), Sense::hover());
    ui.painter().rect_filled(r, 20, c::WHITE);
    ui.painter().rect_stroke(r, 20, Stroke::new(1.0, c::LINE), StrokeKind::Inside);
    let mut rects = Vec::with_capacity(widths.len());
    let mut x = r.min.x + pad;
    for w in &widths {
        rects.push(Rect::from_min_size(pos2(x, r.min.y + pad), vec2(*w, 32.0)));
        x += w;
    }
    let target = rects.get(selected).copied().unwrap_or(rects[0]);
    let pid = ui.id().with(("seg-pill", items.first().map(|i| i.0.as_str()).unwrap_or("")));
    let px = ui.ctx().animate_value_with_time(pid.with("x"), target.min.x, crate::motion::SLIDE_SECS);
    let pw = ui.ctx().animate_value_with_time(pid.with("w"), target.width(), crate::motion::SLIDE_SECS);
    let pill = Rect::from_min_size(pos2(px, target.min.y), vec2(pw, 32.0));
    let mut clicked = None;
    let mut resps = Vec::with_capacity(rects.len());
    for (i, ((l, _), ir)) in items.iter().zip(rects.iter().copied()).enumerate() {
        let resp = ui.interact(ir, ui.id().with(("seg", l.as_str(), i)), Sense::click());
        let sel = i == selected;
        let ht = ui.ctx().animate_bool_with_time(resp.id.with("hover"), resp.hovered() && !sel, crate::motion::HOVER_SECS);
        if ht > 0.0 {
            ui.painter().rect_filled(ir, 16, c::BG.gamma_multiply(ht));
        }
        resps.push(resp);
    }
    ui.painter().rect_filled(pill, 16, c::INK);
    for (i, (((l, n), ir), resp)) in items.iter().zip(rects.iter().copied()).zip(resps).enumerate() {
        let p = ui.painter();
        // label turns white as the pill covers it
        let cover = (pill.intersect(ir).width().max(0.0) / ir.width()).clamp(0.0, 1.0);
        let sel = cover > 0.5;
        let fg = c::INK.lerp_to_gamma(c::WHITE, cover);
        let g = p.layout_no_wrap(l.clone(), font(13.5, W::Medium), fg);
        let gy = ir.center().y - g.size().y / 2.0;
        let gx = ir.min.x + 15.0;
        let gw = g.size().x;
        p.galley(pos2(gx, gy), g, fg);
        if let Some(n) = n.filter(|n| *n > 0) {
            let cx = gx + gw + 7.0;
            let ng = p.layout_no_wrap(n.to_string(), font(11.0, W::Semi), if sel { c::INK } else { c::MUTED });
            let nr = Rect::from_min_size(pos2(cx, ir.center().y - 9.0), vec2(ng.size().x + 10.0, 18.0));
            p.rect_filled(nr, 9, if sel { c::WHITE } else { c::BG });
            p.galley(pos2(nr.min.x + 5.0, nr.center().y - ng.size().y / 2.0), ng, c::INK);
        }
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// Rounded search field with a magnifier and a shortcut hint. Returns the text edit response.
pub fn search_box(ui: &mut Ui, id: &str, text: &mut String, hint: &str, shortcut: &str, w: f32, h: f32, dark: bool) -> Response {
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter().clone();
    let (bg, line, fg, hint_c) = if dark {
        (Color32::from_rgb(48, 54, 68), Color32::from_rgb(70, 77, 92), Color32::from_rgb(230, 233, 238), c::SUBTLE)
    } else {
        (c::WHITE, c::LINE, c::INK, c::FAINT)
    };
    let te_id = ui.id().with(id);
    let focused = ui.memory(|m| m.has_focus(te_id));
    p.rect_filled(r, CornerRadius::same((h / 2.0).min(20.0) as u8), bg);
    p.rect_stroke(
        r,
        CornerRadius::same((h / 2.0).min(20.0) as u8),
        if focused && !dark { Stroke::new(2.0, c::BLUE) } else { Stroke::new(1.0, line) },
        StrokeKind::Inside,
    );
    p.text(pos2(r.min.x + 16.0, r.center().y), Align2::LEFT_CENTER, "🔍", font(13.0, W::Regular), hint_c);
    let hint_shown = if focused && !text.is_empty() { "Esc to clear" } else { shortcut };
    let mut right = r.max.x - 8.0;
    if !hint_shown.is_empty() {
        let g = p.layout_no_wrap(hint_shown.to_string(), font(10.5, W::Medium), hint_c);
        let kr = Rect::from_min_size(pos2(r.max.x - 10.0 - g.size().x - 12.0, r.center().y - 10.0), vec2(g.size().x + 12.0, 20.0));
        p.rect_stroke(kr, 6, Stroke::new(1.0, line), StrokeKind::Inside);
        p.galley(pos2(kr.min.x + 6.0, kr.center().y - g.size().y / 2.0), g, hint_c);
        right = kr.min.x - 6.0;
    }
    let te_rect = Rect::from_min_max(pos2(r.min.x + 38.0, r.min.y + 2.0), pos2(right, r.max.y - 2.0));
    let te = egui::TextEdit::singleline(text)
        .id(te_id)
        .frame(false)
        .hint_text(txt(hint, 13.5, W::Regular, hint_c))
        .font(font(13.5, W::Regular))
        .text_color(fg)
        .vertical_align(egui::Align::Center)
        .desired_width(te_rect.width());
    // a child Ui keeps the field from moving the parent's layout cursor back inside the box
    let resp = ui.new_child(egui::UiBuilder::new().max_rect(te_rect).layout(egui::Layout::left_to_right(egui::Align::Center))).add_sized(te_rect.size(), te);
    if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        text.clear();
    }
    resp
}

/// Labelled form text field used in dialogs.
pub fn field(ui: &mut Ui, label: &str, text: &mut String, hint: &str, w: f32) -> Response {
    ui.label(txt(label, 12.5, W::Semi, c::INK));
    ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(txt(hint, 13.5, W::Regular, c::FAINT))
            .font(font(13.5, W::Regular))
            .desired_width(w)
            .margin(vec2(10.0, 8.0)),
    )
}

pub fn section_label(ui: &mut Ui, s: &str) {
    ui.label(txt(s.to_uppercase(), 11.0, W::Semi, c::FAINT).extra_letter_spacing(0.8));
}

pub fn hline(p: &Painter, x0: f32, x1: f32, y: f32) {
    p.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.0, c::LINE));
}

