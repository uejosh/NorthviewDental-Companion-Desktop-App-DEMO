//! App shell: custom title bar, sidebar, sign-in/lock, sync with the website,
//! toasts and dialogs. Each screen lives in its own module.

mod dialogs;
mod forms;
mod inbox;
mod reviews;
mod savings;
mod signin;

use crate::api::{Client, Cmd, Reply, Staff};
use crate::model::*;
use crate::motion::{self, Track};
use crate::notify::Notifier;
use crate::sample;
use crate::storage::{Config, Loaded, Storage};
use crate::theme::{self, c, font, txt, W};
use chrono::{DateTime, Local, NaiveDate, Utc};
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, Id, Key, Modifiers, Rect, ResizeDirection, Sense, Stroke, StrokeKind, TextureHandle,
    ViewportCommand,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use dialogs::Dialog;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const SYNC_EVERY: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Requests,
    Forms,
    Savings,
    Reviews,
}

impl Screen {
    pub fn from_arg(s: &str) -> Option<Screen> {
        Some(match s {
            "requests" | "booking" | "inbox" => Screen::Requests,
            "forms" => Screen::Forms,
            "savings" | "members" => Screen::Savings,
            "reviews" => Screen::Reviews,
            _ => return None,
        })
    }
}

#[derive(PartialEq)]
pub enum Phase {
    Damaged(String),
    SignIn,
    Main,
}

pub struct Options {
    pub data_dir: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub screen: Option<String>,
    pub force_demo: bool,
    /// Debug builds only: take the screenshot against the configured website,
    /// signing in with the PIN in NVO_TEST_PIN.
    pub live_shot: bool,
    /// Turn off transitions (also off automatically for screenshots).
    pub no_motion: bool,
    /// Debug builds: capture the screenshot partway through a transition into `screen`.
    pub shot_mid: bool,
}

pub struct Toast {
    pub msg: String,
    pub ok: bool,
    pub at: Instant,
}

pub struct Textures {
    pub icon: TextureHandle,
    pub logo: TextureHandle,
    pub bg: TextureHandle,
}

pub struct App {
    pub storage: Storage,
    pub cfg: Config,
    pub client: Client,
    pub notifier: Notifier,
    pub tex: Textures,
    pub data: Data,
    pub phase: Phase,

    // sign-in
    pub staff: Vec<Staff>,
    pub staff_err: Option<String>,
    pub staff_loading: bool,
    pub sel_staff: usize,
    pub pin: String,
    pub signin_err: Option<String>,
    pub signing_in: bool,
    pub me: Option<Staff>,
    pub last_input: Instant,

    pub screen: Screen,
    pub dialog: Option<Dialog>,
    pub toast: Option<Toast>,

    // booking inbox
    pub filter: Filter,
    pub query: String,
    pub sel_req: Option<u64>,
    pub viewed: HashSet<u64>,
    pub focus_inbox_search: bool,

    // forms
    pub sel_form: Option<u64>,
    pub jump_medical: bool,

    // savings
    pub mem_filter: usize,
    pub mem_query: String,

    // reviews
    pub preview_email: bool,
    pub show_all_sent: bool,

    // global search
    pub gsearch: String,
    pub focus_gsearch: bool,

    // website sync
    pub last_sync: Option<DateTime<Utc>>,
    pub sync_err: Option<String>,
    pub next_sync: Instant,
    pub syncing: bool,
    pub busy: bool,
    pub pending_msg: Option<String>,
    pub known_ids: Option<HashSet<u64>>,
    pub host: String,

    pub pending_sim: Option<Instant>,

    // motion
    pub motion: bool,
    pub tr_phase: Track<u8>,
    pub tr_screen: Track<Screen>,
    pub tr_detail: Track<u64>,
    pub tr_form: Track<u64>,
    /// Eased progress of the current sign-in/lock transition (1.0 = settled).
    pub phase_t: f32,

    shot: Option<(PathBuf, u32, bool)>,
    shot_mid: Option<Screen>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Options) -> Self {
        theme::install(&cc.egui_ctx);
        let storage = Storage::new(opts.data_dir.clone());
        let mut cfg = storage.load_config();
        if opts.force_demo || (opts.screenshot.is_some() && !opts.live_shot) {
            cfg.mode = "demo".into();
        }
        let client = Client::start(Some(cc.egui_ctx.clone()));
        let tex = Textures {
            icon: load_tex(&cc.egui_ctx, "app-icon", include_bytes!("../../assets/icon.png")),
            logo: load_tex(&cc.egui_ctx, "logo", include_bytes!("../../assets/NorthViewDental-Logo-R.png")),
            bg: load_tex(&cc.egui_ctx, "signin-bg", include_bytes!("../../assets/Sign-in_screen_bg.jpg")),
        };
        let mut app = App {
            storage,
            cfg,
            client,
            notifier: Notifier::start(),
            tex,
            data: Data::default(),
            phase: Phase::SignIn,
            staff: vec![],
            staff_err: None,
            staff_loading: false,
            sel_staff: 0,
            pin: String::new(),
            signin_err: None,
            signing_in: false,
            me: None,
            last_input: Instant::now(),
            screen: Screen::Requests,
            dialog: None,
            toast: None,
            filter: Filter::Open,
            query: String::new(),
            sel_req: None,
            viewed: HashSet::new(),
            focus_inbox_search: false,
            sel_form: None,
            jump_medical: false,
            mem_filter: 0,
            mem_query: String::new(),
            preview_email: false,
            show_all_sent: false,
            gsearch: String::new(),
            focus_gsearch: false,
            last_sync: None,
            sync_err: None,
            next_sync: Instant::now(),
            syncing: false,
            busy: false,
            pending_msg: None,
            known_ids: None,
            host: String::new(),
            pending_sim: None,
            shot: None,
            motion: opts.screenshot.is_none() && !opts.no_motion,
            tr_phase: Track::default(),
            tr_screen: Track::default(),
            tr_detail: Track::default(),
            tr_form: Track::default(),
            phase_t: 1.0,
            shot_mid: None,
        };
        app.connect();
        if let Some(path) = opts.screenshot {
            let screen = opts.screen.as_deref().unwrap_or("requests");
            if app.live() {
                app.screen = Screen::from_arg(screen).unwrap_or(Screen::Requests);
                app.shot = Some((path, 0, false));
                return app;
            }
            if screen != "signin" {
                app.demo_sign_in(0);
                app.screen = Screen::from_arg(screen).unwrap_or(Screen::Requests);
                if app.screen == Screen::Requests {
                    app.sel_req = app.data.requests.iter().find(|r| r.id == 1044).map(|r| r.id);
                }
                // screenshot-only hooks for checking layout states (demo data only)
                if let Ok(q) = std::env::var("NVO_SHOT_QUERY") {
                    app.query = q.clone();
                    app.mem_query = q;
                }
                if let Ok(plan) = std::env::var("NVO_SHOT_PLAN") {
                    if let Dialog::AddMember { plan: p, plan_touched, .. } = &mut *app.dialog.insert(Dialog::add_member()) {
                        *p = plan;
                        *plan_touched = true;
                    }
                }
            } else {
                app.pin = "1234".into();
            }
            app.shot = Some((path, 0, false));
            if opts.shot_mid && app.phase == Phase::Main {
                app.motion = true;
                app.shot_mid = Some(app.screen);
                app.screen = if app.screen == Screen::Requests { Screen::Reviews } else { Screen::Requests };
            }
        }
        app
    }

    pub fn live(&self) -> bool {
        self.cfg.live()
    }

    pub fn today(&self) -> NaiveDate {
        Local::now().date_naive()
    }

    pub fn user_name(&self) -> String {
        self.me.as_ref().map(|m| m.name.clone()).unwrap_or_default()
    }

    /// (Re)connect according to the saved configuration.
    pub fn connect(&mut self) {
        self.staff.clear();
        self.staff_err = None;
        self.sel_staff = 0;
        self.data = Data::default();
        self.known_ids = None;
        self.last_sync = None;
        self.sync_err = None;
        self.phase = Phase::SignIn;
        if self.live() {
            self.client.send(Cmd::Configure { url: self.cfg.site_url.clone(), key: self.cfg.device_key.clone() });
            self.client.send(Cmd::Staff);
            self.staff_loading = true;
            self.host = self.cfg.site_url.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string();
        } else {
            self.staff_loading = false;
            self.staff = demo_staff();
            self.host = "demo".into();
            match self.storage.load_demo() {
                Loaded::Ok(d) => self.data = d,
                Loaded::Missing => {
                    self.data = sample::seed(Utc::now());
                    let _ = self.storage.save_demo(&self.data);
                }
                Loaded::Damaged(msg) => self.phase = Phase::Damaged(msg),
            }
            self.known_ids = Some(self.data.requests.iter().map(|r| r.id).collect());
        }
        if let Some(i) = self.staff.iter().position(|s| s.id == self.cfg.last_staff_id) {
            self.sel_staff = i;
        }
    }

    pub fn demo_sign_in(&mut self, idx: usize) {
        if let Some(s) = self.staff.get(idx).cloned() {
            self.me = Some(s);
            self.phase = Phase::Main;
            self.last_input = Instant::now();
        }
    }

    pub fn signed_in(&mut self, s: Staff) {
        self.cfg.last_staff_id = s.id;
        let _ = self.storage.save_config(&self.cfg);
        self.me = Some(s);
        self.pin.clear();
        self.signin_err = None;
        self.signing_in = false;
        self.phase = Phase::Main;
        self.last_input = Instant::now();
        self.viewed.clear();
        if self.live() {
            self.request_sync();
        }
    }

    /// Lock: back to sign-in. In live mode the session is ended and patient data is cleared from memory.
    pub fn lock(&mut self, reason: Option<String>) {
        if self.live() {
            self.client.send(Cmd::Logout);
            self.data = Data::default();
            self.known_ids = None;
            self.last_sync = None;
        }
        self.me = None;
        self.pin.clear();
        self.dialog = None;
        self.gsearch.clear();
        self.signin_err = reason;
        self.signing_in = false;
        self.busy = false;
        self.phase = Phase::SignIn;
        if self.live() {
            self.client.send(Cmd::Staff);
        }
    }

    pub fn request_sync(&mut self) {
        if self.live() && self.phase == Phase::Main && !self.syncing {
            self.client.send(Cmd::Sync);
            self.syncing = true;
            self.next_sync = Instant::now() + SYNC_EVERY;
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>, ok: bool) {
        self.toast = Some(Toast { msg: msg.into(), ok, at: Instant::now() });
    }

    /// Perform a staff action: locally in demo mode, through the website in live mode.
    pub fn act(&mut self, act: Act, msg: &str) {
        let quiet = matches!(act, Act::Viewed(_) | Act::Copied(_) | Act::FormPdf(_));
        if self.live() {
            if !quiet {
                self.busy = true;
                self.pending_msg = Some(msg.to_string());
            }
            self.client.send(Cmd::Act(act));
        } else {
            let user = self.user_name();
            match self.data.apply(&act, &user, Utc::now()) {
                Ok(()) => {
                    if let Err(e) = self.storage.save_demo(&self.data) {
                        self.toast(e, false);
                        return;
                    }
                    if !quiet && !msg.is_empty() {
                        self.toast(msg, true);
                    }
                }
                Err(e) => self.toast(e, false),
            }
        }
    }

    pub fn reset_demo(&mut self) {
        self.data = sample::seed(Utc::now());
        let _ = self.storage.save_demo(&self.data);
        self.known_ids = Some(self.data.requests.iter().map(|r| r.id).collect());
        self.sel_req = None;
        self.sel_form = None;
        self.pending_sim = None;
        self.toast("Demo data restored", true);
    }

    fn on_new_requests(&mut self, ctx: &egui::Context, new: Vec<(u64, bool)>) {
        if new.is_empty() {
            return;
        }
        let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        let emergency = new.iter().any(|n| n.1);
        if focused {
            self.toast(
                if emergency { "Emergency appointment request arrived from the website" } else { "New appointment request arrived from the website" },
                true,
            );
        } else {
            for (_, e) in &new {
                self.notifier.new_request(*e);
            }
        }
    }

    fn handle_replies(&mut self, ctx: &egui::Context) {
        while let Some(r) = self.client.poll() {
            match r {
                Reply::Staff(Ok(list)) => {
                    self.staff_loading = false;
                    self.staff_err = if list.staff.is_empty() { Some("No staff have a PIN yet. Ask the office manager to set one up on the website (Tools → Northview Office App).".into()) } else { None };
                    self.staff = list.staff;
                    if !list.host.is_empty() {
                        self.host = list.host;
                    }
                    self.sel_staff = self.staff.iter().position(|s| s.id == self.cfg.last_staff_id).unwrap_or(0);
                }
                Reply::Staff(Err(e)) => {
                    self.staff_loading = false;
                    self.staff_err = Some(e);
                }
                Reply::Login(Ok(s)) => self.signed_in(s),
                Reply::Login(Err(e)) => {
                    self.signing_in = false;
                    self.pin.clear();
                    self.signin_err = Some(e);
                }
                Reply::Snapshot(Ok(s)) => {
                    self.syncing = false;
                    self.busy = false;
                    self.sync_err = None;
                    self.last_sync = Some(Utc::now());
                    if !s.host.is_empty() {
                        self.host = s.host.clone();
                    }
                    let ids: HashSet<u64> = s.requests.iter().map(|r| r.id).collect();
                    let new: Vec<(u64, bool)> = match &self.known_ids {
                        Some(k) => s.requests.iter().filter(|r| r.status == Status::New && !k.contains(&r.id)).map(|r| (r.id, r.emergency)).collect(),
                        None => vec![],
                    };
                    self.known_ids = Some(ids);
                    self.data = s.into();
                    if let Some(m) = self.pending_msg.take() {
                        if !m.is_empty() {
                            self.toast(m, true);
                        }
                    }
                    self.on_new_requests(ctx, new);
                }
                Reply::Snapshot(Err(e)) => {
                    self.syncing = false;
                    if self.busy {
                        self.toast(e.clone(), false);
                    }
                    self.busy = false;
                    self.pending_msg = None;
                    self.sync_err = Some(e);
                }
                Reply::Done(Ok(())) => {}
                Reply::Done(Err(e)) => self.toast(e, false),
                Reply::SignedOut(m) => {
                    if self.phase == Phase::Main {
                        self.lock(Some(m));
                    }
                }
            }
        }
    }

    fn tick(&mut self, ctx: &egui::Context) {
        // Activity tracking for auto-lock.
        let active = ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(e, egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::PointerButton { .. } | egui::Event::PointerMoved(_) | egui::Event::MouseWheel { .. })
            })
        });
        if active {
            self.last_input = Instant::now();
        }
        if self.phase == Phase::Main && self.shot.is_none() && self.last_input.elapsed() >= Duration::from_secs(AUTO_LOCK_SECS) {
            self.lock(Some("Locked after 5 minutes without activity.".into()));
        }
        if self.phase == Phase::Main && self.live() && Instant::now() >= self.next_sync {
            self.request_sync();
        }
        if let Some(t) = self.pending_sim {
            if Instant::now() >= t {
                self.pending_sim = None;
                let r = sample::simulated_request(&mut self.data, Utc::now());
                let pair = (r.id, r.emergency);
                self.data.requests.push(r);
                let _ = self.storage.save_demo(&self.data);
                if let Some(k) = self.known_ids.as_mut() {
                    k.insert(pair.0);
                }
                self.on_new_requests(ctx, vec![pair]);
            }
        }
        if let Some(t) = &self.toast {
            if t.at.elapsed().as_secs_f32() > toast_life(t.ok) {
                self.toast = None;
            }
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    // ------------------------------------------------------------------ window chrome

    fn resize_edges(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        let screen = ctx.screen_rect();
        let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
        let m = 5.0;
        let (l, r, t, b) = (p.x <= screen.min.x + m, p.x >= screen.max.x - m, p.y <= screen.min.y + m, p.y >= screen.max.y - m);
        let dir = match (l, r, t, b) {
            (true, _, true, _) => Some((ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest)),
            (_, true, true, _) => Some((ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast)),
            (true, _, _, true) => Some((ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest)),
            (_, true, _, true) => Some((ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast)),
            (true, ..) => Some((ResizeDirection::West, CursorIcon::ResizeWest)),
            (_, true, ..) => Some((ResizeDirection::East, CursorIcon::ResizeEast)),
            (_, _, true, _) => Some((ResizeDirection::North, CursorIcon::ResizeNorth)),
            (.., true) => Some((ResizeDirection::South, CursorIcon::ResizeSouth)),
            _ => None,
        };
        if let Some((d, icon)) = dir {
            ctx.set_cursor_icon(icon);
            if ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(d));
            }
        }
    }

    fn title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(40.0)
            .frame(egui::Frame::new().fill(c::TITLEBAR))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let r = ui.max_rect();
                let drag = ui.interact(r, Id::new("titlebar-drag"), Sense::click_and_drag());
                if drag.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
                }
                let p = ui.painter();
                let lr = Rect::from_center_size(pos2(r.min.x + 24.0, r.center().y), vec2(22.0, 22.0));
                p.image(self.tex.icon.id(), lr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                p.text(pos2(r.min.x + 42.0, r.center().y), Align2::LEFT_CENTER, "Northview Dental — Office", font(12.5, W::Medium), Color32::from_rgb(220, 224, 230));

                // window buttons
                let bw = 46.0;
                let labels = [("🗙", 0), ("🗖", 1), ("🗕", 2)];
                for (i, (g, kind)) in labels.iter().enumerate() {
                    let br = Rect::from_min_size(pos2(r.max.x - bw * (i as f32 + 1.0), r.min.y), vec2(bw, r.height()));
                    let resp = ui.interact(br, Id::new(("winbtn", i)), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(br, 0, if *kind == 0 { c::RED_BRIGHT } else { Color32::from_rgb(55, 62, 78) });
                    }
                    ui.painter().text(br.center(), Align2::CENTER_CENTER, *g, font(12.0, W::Regular), Color32::from_rgb(220, 224, 230));
                    if resp.clicked() {
                        match kind {
                            0 => ctx.send_viewport_cmd(ViewportCommand::Close),
                            1 => {
                                let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                                ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
                            }
                            _ => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                        }
                    }
                }

                if self.phase == Phase::Main {
                    let w = 440.0_f32.min(r.width() - 520.0).max(220.0);
                    let sr = Rect::from_center_size(pos2(r.center().x, r.center().y), vec2(w, 28.0));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(sr));
                    let resp = theme::search_box(&mut child, "gsearch", &mut self.gsearch, "Search patients, requests, members…", "Ctrl K", w, 28.0, true);
                    if self.focus_gsearch {
                        resp.request_focus();
                        self.focus_gsearch = false;
                    }
                    if !self.gsearch.trim().is_empty() {
                        self.global_results(ctx, sr, resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
                    }
                }
            });
    }

    fn global_results(&mut self, ctx: &egui::Context, anchor: Rect, enter: bool) {
        let q = self.gsearch.trim().to_lowercase();
        let mut rows: Vec<(String, String, String, Screen, u64)> = vec![];
        for r in &self.data.requests {
            if r.matches(&q) {
                rows.push(("Request".into(), r.name(), format!("{} · {}", r.status.label(), r.reason), Screen::Requests, r.id));
            }
        }
        for f in &self.data.forms {
            if f.name.to_lowercase().contains(&q) {
                rows.push(("Form".into(), f.name.clone(), if f.attached() { "In Eaglesoft".into() } else { "To attach".into() }, Screen::Forms, f.id));
            }
        }
        for m in &self.data.members {
            if m.matches(&q) {
                rows.push(("Member".into(), m.name.clone(), format!("{} · renews {}", plan_label(&m.plan), m.renews.format("%b %-d, %Y")), Screen::Savings, m.id));
            }
        }
        for v in &self.data.visits {
            if v.name.to_lowercase().contains(&q) {
                rows.push(("Visit".into(), v.name.clone(), format!("Visit {}", v.visit_date.format("%b %-d")), Screen::Reviews, v.id));
            }
        }
        rows.truncate(8);
        let mut go: Option<(Screen, u64)> = if enter { rows.first().map(|r| (r.3, r.4)) } else { None };
        egui::Area::new(Id::new("gsearch-results"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(anchor.min.x, anchor.max.y + 6.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).fill(c::WHITE).corner_radius(10).inner_margin(6).show(ui, |ui| {
                    ui.set_width(anchor.width());
                    if rows.is_empty() {
                        ui.add_space(6.0);
                        ui.label(txt(format!("No matches for “{}”", self.gsearch.trim()), 13.0, W::Regular, c::MUTED));
                        ui.add_space(6.0);
                    }
                    for (kind, name, sub, screen, id) in &rows {
                        let (r, resp) = ui.allocate_exact_size(vec2(anchor.width(), 44.0), Sense::click());
                        let p = ui.painter();
                        if resp.hovered() {
                            p.rect_filled(r, 8, c::BG);
                        }
                        theme::chip_at(p, pos2(r.min.x + 8.0, r.center().y - 10.0), kind, c::BLUE, c::BLUE_BG);
                        p.text(pos2(r.min.x + 86.0, r.min.y + 5.0), Align2::LEFT_TOP, name, font(13.5, W::Semi), c::INK);
                        theme::text_at(p, pos2(r.min.x + 86.0, r.min.y + 24.0), sub, font(11.5, W::Regular), c::MUTED, r.width() - 96.0);
                        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            go = Some((*screen, *id));
                        }
                    }
                });
            });
        if let Some((screen, id)) = go {
            self.screen = screen;
            match screen {
                Screen::Requests => {
                    self.filter = Filter::All;
                    self.query.clear();
                    self.sel_req = Some(id);
                }
                Screen::Forms => self.sel_form = Some(id),
                Screen::Savings => self.mem_query = self.data.members.iter().find(|m| m.id == id).map(|m| m.name.clone()).unwrap_or_default(),
                Screen::Reviews => {}
            }
            self.gsearch.clear();
        }
    }

    // ------------------------------------------------------------------ sidebar

    fn sidebar(&mut self, ctx: &egui::Context) {
        let today = self.today();
        let new_count = self.data.requests.iter().filter(|r| r.status == Status::New).count();
        let attach = self.data.forms_to_attach();
        let renewing = self.data.members.iter().filter(|m| m.renewing_soon(today)).count();
        egui::SidePanel::left("sidebar")
            .exact_width(244.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(c::SIDEBAR))
            .show(ctx, |ui| {
                ui.set_opacity(self.phase_t);
                let r = ui.max_rect();
                let p = ui.painter().clone();
                let logo_h = 38.0;
                let ls = self.tex.logo.size_vec2();
                let lw = ls.x / ls.y * logo_h;
                p.image(
                    self.tex.logo.id(),
                    Rect::from_min_size(pos2(r.min.x + 32.0, r.min.y + 24.0), vec2(lw, logo_h)),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::from_rgb(235, 238, 243),
                );
                let role = if self.me.as_ref().is_some_and(|m| m.manager) { "OFFICE MANAGER" } else { "FRONT DESK" };
                p.text(pos2(r.min.x + 24.0, r.min.y + 88.0), Align2::LEFT_TOP, role, font(10.5, W::Semi), c::SUBTLE);

                let items = [
                    (Screen::Requests, "📥", "Booking requests", new_count, true),
                    (Screen::Forms, "📄", "Patient forms", attach, false),
                    (Screen::Savings, "💳", "Smile Savings Plan", renewing, false),
                    (Screen::Reviews, "☆", "Review requests", 0, false),
                ];
                let top = r.min.y + 112.0;
                let sel_idx = items.iter().position(|i| i.0 == self.screen).unwrap_or(0) as f32;
                let target = top + sel_idx * 46.0;
                let hy = if self.motion { ctx.animate_value_with_time(Id::new("nav-highlight"), target, motion::SLIDE_SECS) } else { target };
                let hr = Rect::from_min_size(pos2(r.min.x + 12.0, hy), vec2(r.width() - 24.0, 40.0));
                p.rect_filled(hr, 8, c::SIDEBAR_HI);
                p.rect_filled(Rect::from_min_size(pos2(hr.min.x, hr.min.y + 10.0), vec2(3.0, 20.0)), 2, c::SKY);
                let mut y = top;
                for (screen, icon, label, count, red) in items {
                    let ir = Rect::from_min_size(pos2(r.min.x + 12.0, y), vec2(r.width() - 24.0, 40.0));
                    let resp = ui.interact(ir, Id::new(("nav", label)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    let sel = self.screen == screen;
                    let h = ctx.animate_bool_with_time(resp.id.with("hover"), resp.hovered() && !sel, motion::HOVER_SECS);
                    if h > 0.0 {
                        p.rect_filled(ir, 8, Color32::from_rgb(56, 63, 79).gamma_multiply(h));
                    }
                    // text brightens as the sliding highlight arrives
                    let near = (1.0 - (hy - y).abs() / 46.0).clamp(0.0, 1.0);
                    let fg = Color32::from_rgb(208, 213, 221).lerp_to_gamma(c::WHITE, near);
                    p.text(pos2(ir.min.x + 14.0, ir.center().y), Align2::LEFT_CENTER, icon, font(15.0, W::Regular), fg.lerp_to_gamma(Color32::from_rgb(120, 190, 240), near));
                    p.text(pos2(ir.min.x + 42.0, ir.center().y), Align2::LEFT_CENTER, label, font(13.5, if sel { W::Semi } else { W::Medium }), fg);
                    if count > 0 {
                        let cc = pos2(ir.max.x - 20.0, ir.center().y);
                        p.circle_filled(cc, 10.0, if red { c::RED_BRIGHT } else { Color32::from_rgb(78, 86, 103) });
                        p.text(cc, Align2::CENTER_CENTER, count.to_string(), font(10.5, W::Semi), c::WHITE);
                    }
                    if resp.clicked() {
                        self.screen = screen;
                    }
                    y += 46.0;
                }

                // connection card
                let cr = Rect::from_min_size(pos2(r.min.x + 12.0, r.max.y - 150.0), vec2(r.width() - 24.0, 60.0));
                p.rect_filled(cr, 10, Color32::from_rgb(56, 63, 79));
                p.rect_stroke(cr, 10, Stroke::new(1.0, Color32::from_rgb(70, 77, 92)), StrokeKind::Inside);
                let (dotc, title, sub) = if !self.live() {
                    (c::SKY, "Demo data".to_string(), "Sample patients only".to_string())
                } else if let Some(e) = &self.sync_err {
                    let _ = e;
                    (c::AMBER_DOT, "Website unreachable".to_string(), format!("{} · retrying", self.host))
                } else if let Some(t) = self.last_sync {
                    (c::GREEN_DOT, "Website connected".to_string(), format!("Synced {} · {}", ago(t, Utc::now()), self.host))
                } else {
                    (c::SUBTLE, "Connecting…".to_string(), self.host.clone())
                };
                theme::dot(&p, pos2(cr.min.x + 16.0, cr.min.y + 20.0), 4.0, dotc);
                p.text(pos2(cr.min.x + 28.0, cr.min.y + 11.0), Align2::LEFT_TOP, title, font(12.5, W::Semi), c::WHITE);
                theme::text_at(&p, pos2(cr.min.x + 14.0, cr.min.y + 33.0), &sub, font(11.0, W::Regular), c::SUBTLE, cr.width() - 28.0);
                if !self.live() {
                    let lr = Rect::from_min_size(pos2(cr.max.x - 84.0, cr.min.y + 9.0), vec2(74.0, 20.0));
                    let resp = ui.interact(lr, Id::new("reset-demo"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    p.text(lr.right_center(), Align2::RIGHT_CENTER, "Reset demo", font(11.0, W::Medium), if resp.hovered() { c::WHITE } else { Color32::from_rgb(140, 200, 240) });
                    if resp.clicked() {
                        self.dialog = Some(Dialog::ResetDemo);
                    }
                } else if let Some(e) = self.sync_err.clone() {
                    ui.interact(cr, Id::new("sync-err"), Sense::hover()).on_hover_text(e);
                }

                // user row
                p.line_segment([pos2(r.min.x + 12.0, r.max.y - 76.0), pos2(r.max.x - 12.0, r.max.y - 76.0)], Stroke::new(1.0, Color32::from_rgb(64, 71, 87)));
                if let Some(me) = &self.me {
                    let ac = pos2(r.min.x + 32.0, r.max.y - 38.0);
                    theme::avatar(&p, ac, 17.0, &me.initials, c::BLUE, c::WHITE);
                    theme::text_at(&p, pos2(ac.x + 28.0, ac.y - 18.0), &me.name, font(13.0, W::Semi), c::WHITE, 120.0);
                    let left = Duration::from_secs(AUTO_LOCK_SECS).saturating_sub(self.last_input.elapsed()).as_secs();
                    p.text(pos2(ac.x + 28.0, ac.y + 2.0), Align2::LEFT_TOP, format!("Auto-lock in {}:{:02}", left / 60, left % 60), font(11.0, W::Regular), c::SUBTLE);
                    let lr = Rect::from_center_size(pos2(r.max.x - 34.0, ac.y), vec2(34.0, 34.0));
                    let resp = ui.interact(lr, Id::new("lock"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand).on_hover_text("Lock now");
                    p.rect_stroke(lr, 8, Stroke::new(1.0, if resp.hovered() { c::WHITE } else { Color32::from_rgb(110, 118, 135) }), StrokeKind::Inside);
                    p.text(lr.center(), Align2::CENTER_CENTER, "🔒", font(14.0, W::Regular), c::WHITE);
                    if resp.clicked() {
                        self.lock(None);
                    }
                }
            });
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let Some(t) = &self.toast else { return };
        let (msg, ok) = (t.msg.clone(), t.ok);
        let age = t.at.elapsed().as_secs_f32();
        let life = toast_life(ok);
        let (rise, alpha) = if self.motion {
            let t_in = egui::emath::easing::cubic_out((age / 0.24).clamp(0.0, 1.0));
            let t_out = ((life - age) / 0.3).clamp(0.0, 1.0);
            ((1.0 - t_in) * 22.0, t_in.min(t_out))
        } else {
            (0.0, 1.0)
        };
        if alpha < 1.0 || rise > 0.0 {
            ctx.request_repaint();
        }
        let mut dismiss = false;
        let screen = ctx.screen_rect();
        egui::Area::new(Id::new("toast"))
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_BOTTOM, vec2(122.0, -24.0 + rise))
            .show(ctx, |ui| {
                ui.set_opacity(alpha);
                egui::Frame::new()
                    .fill(c::TOAST)
                    .corner_radius(24)
                    .inner_margin(egui::Margin { left: 18, right: 8, top: 8, bottom: 8 })
                    .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(60) })
                    .show(ui, |ui| {
                        ui.set_max_width(screen.width() * 0.6);
                        ui.horizontal(|ui| {
                            ui.label(txt(if ok { "✔" } else { "⚠" }, 14.0, W::Semi, if ok { c::GREEN_DOT } else { Color32::from_rgb(255, 170, 160) }));
                            ui.add_space(4.0);
                            ui.label(txt(&msg, 13.5, W::Medium, c::WHITE));
                            ui.add_space(8.0);
                            let b = ui.add(egui::Button::new(txt("Dismiss", 12.5, W::Semi, c::WHITE)).fill(Color32::from_rgb(55, 62, 78)).corner_radius(14).stroke(Stroke::NONE));
                            if b.clicked() {
                                dismiss = true;
                            }
                        });
                    });
            });
        if dismiss {
            self.toast = None;
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.phase != Phase::Main {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
            self.focus_gsearch = true;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::F)) {
            match self.screen {
                Screen::Requests => self.focus_inbox_search = true,
                _ => self.focus_gsearch = true,
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::L)) {
            self.lock(None);
        }
    }

    fn screenshot(&mut self, ctx: &egui::Context) {
        if self.shot.is_none() {
            return;
        }
        #[cfg(debug_assertions)]
        if self.live() {
            if self.phase == Phase::SignIn && !self.staff.is_empty() && !self.signing_in && self.me.is_none() {
                if let Ok(pin) = std::env::var("NVO_TEST_PIN") {
                    self.signing_in = true;
                    self.client.send(Cmd::Login { user_id: self.staff[0].id, pin });
                }
            }
            if self.last_sync.is_none() && self.signin_err.is_none() {
                ctx.request_repaint_after(Duration::from_millis(100));
                return;
            }
        }
        let Some((path, frames, requested)) = self.shot.as_mut() else { return };
        *frames += 1;
        if let Some(target) = self.shot_mid {
            if *frames == 12 {
                self.screen = target;
            }
        }
        let capture_at = if self.shot_mid.is_some() { 15 } else { 12 };
        if !*requested && *frames > capture_at {
            *requested = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let path = path.clone();
        let img = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = img {
            let [w, h] = img.size;
            let bytes: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
            if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                if let Err(e) = buf.save(&path) {
                    eprintln!("screenshot: {e}");
                }
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.95, 0.96, 0.98, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_replies(ctx);
        self.tick(ctx);
        self.shortcuts(ctx);
        self.resize_edges(ctx);
        self.title_bar(ctx);
        let phase_key = match self.phase {
            Phase::Damaged(_) => 0u8,
            Phase::SignIn => 1,
            Phase::Main => 2,
        };
        self.phase_t = self.tr_phase.update(&phase_key, motion::SCREEN_SECS, self.motion);
        let screen_t = self.tr_screen.update(&self.screen, motion::SCREEN_SECS, self.motion);
        if self.phase_t < 1.0 || screen_t < 1.0 {
            ctx.request_repaint();
        }
        match &self.phase {
            Phase::Damaged(msg) => {
                let msg = msg.clone();
                dialogs::damaged(self, ctx, &msg);
            }
            Phase::SignIn => signin::show(self, ctx),
            Phase::Main => {
                self.sidebar(ctx);
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(c::BG).inner_margin(egui::Margin { left: 32, right: 32, top: 26, bottom: 28 }))
                    .show(ctx, |ui| {
                        // incoming screen fades in while rising slightly into place
                        let t = self.phase_t.min(screen_t);
                        let r = ui.max_rect().translate(vec2(0.0, (1.0 - t) * motion::RISE));
                        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
                        ui.set_opacity(t);
                        let ui = &mut ui;
                        match self.screen {
                            Screen::Requests => inbox::show(self, ui),
                            Screen::Forms => forms::show(self, ui),
                            Screen::Savings => savings::show(self, ui),
                            Screen::Reviews => reviews::show(self, ui),
                        }
                    });
            }
        }
        dialogs::show(self, ctx);
        self.toast_ui(ctx);
        self.screenshot(ctx);
    }
}

// ------------------------------------------------------------------ shared bits

pub fn toast_life(ok: bool) -> f32 {
    if ok { 4.0 } else { 7.0 }
}

pub fn demo_staff() -> Vec<Staff> {
    vec![
        Staff { id: 9001, name: "Front Desk".into(), role: "Reception".into(), initials: "FD".into(), manager: false },
        Staff { id: 9002, name: "Office Manager".into(), role: "Oversight".into(), initials: "OM".into(), manager: true },
    ]
}

fn load_tex(ctx: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let img = image::load_from_memory(bytes).map(|i| i.to_rgba8()).unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture(name, ci, egui::TextureOptions::LINEAR)
}

/// Page header: title + subtitle on the left; `right` draws right-aligned controls.
pub fn header(ui: &mut egui::Ui, title: &str, sub: &str, right: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(txt(title, 26.0, W::Semi, c::INK));
            ui.add_space(-2.0);
            ui.label(txt(sub, 13.5, W::Regular, c::MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
    ui.add_space(18.0);
}

/// Splits the remaining area into a fixed-width left column and a flexible right column.
pub fn split(ui: &mut egui::Ui, left_w: f32, gap: f32) -> [(egui::Ui, Rect); 2] {
    let r = ui.available_rect_before_wrap();
    let lr = Rect::from_min_size(r.min, vec2(left_w, r.height()));
    let rr = Rect::from_min_max(pos2(r.min.x + left_w + gap, r.min.y), r.max);
    let lu = ui.new_child(egui::UiBuilder::new().max_rect(lr).layout(egui::Layout::top_down(egui::Align::Min)));
    let ru = ui.new_child(egui::UiBuilder::new().max_rect(rr).layout(egui::Layout::top_down(egui::Align::Min)));
    ui.allocate_rect(r, Sense::hover());
    [(lu, lr), (ru, rr)]
}

pub fn tiles_row(ui: &mut egui::Ui, n: usize) -> f32 {
    let gap = 16.0;
    (ui.available_width() - gap * (n as f32 - 1.0)) / n as f32
}

pub fn fmt_time(t: DateTime<Utc>) -> String {
    t.with_timezone(&Local).format("%-I:%M %p").to_string()
}

pub fn fmt_stamp(t: DateTime<Utc>) -> String {
    t.with_timezone(&Local).format("%b %-d, %-I:%M %p").to_string()
}

