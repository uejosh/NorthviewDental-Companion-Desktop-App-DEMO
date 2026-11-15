//! Website connection. All HTTP runs on one worker thread; the UI sends commands and
//! polls replies each frame, so a slow network never freezes the screen.
//!
//! Every request carries the device key (X-NVO-Device). After sign-in, the session token
//! is held only in this thread's memory and dropped on lock or sign-out.

use crate::model::{Act, Snapshot};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Staff {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub initials: String,
    #[serde(default)]
    pub manager: bool,
}

#[derive(Debug, Deserialize)]
pub struct StaffList {
    pub staff: Vec<Staff>,
    #[serde(default)]
    pub host: String,
}

pub enum Cmd {
    Configure { url: String, key: String },
    Staff,
    Login { user_id: u64, pin: String },
    Logout,
    Sync,
    Act(Act),
}

pub enum Reply {
    Staff(Result<StaffList, String>),
    Login(Result<Staff, String>),
    Snapshot(Result<Snapshot, String>),
    /// An action that does not return data finished (or failed).
    Done(Result<(), String>),
    /// The website no longer accepts the session (expired or revoked).
    SignedOut(String),
}

pub struct Client {
    tx: Sender<Cmd>,
    rx: Receiver<Reply>,
}

impl Client {
    pub fn start(ctx: Option<eframe::egui::Context>) -> Self {
        let (tx, crx) = channel::<Cmd>();
        let (rtx, rx) = channel::<Reply>();
        thread::Builder::new()
            .name("website".into())
            .spawn(move || Worker::default().run(crx, rtx, ctx))
            .expect("website thread");
        Self { tx, rx }
    }
    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }
    pub fn poll(&self) -> Option<Reply> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

#[derive(Default)]
struct Worker {
    base: String,
    key: String,
    token: Option<String>,
}

enum Fail {
    Session(String),
    Other(String),
}

impl Worker {
    fn run(mut self, rx: Receiver<Cmd>, tx: Sender<Reply>, ctx: Option<eframe::egui::Context>) {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("NorthviewOffice/", env!("CARGO_PKG_VERSION")))
            .build();
        for cmd in rx {
            let reply = self.handle(&agent, cmd);
            if let Some(r) = reply {
                if tx.send(r).is_err() {
                    break;
                }
                if let Some(c) = &ctx {
                    c.request_repaint();
                }
            }
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/wp-json/nvoffice/v1{}", self.base.trim_end_matches('/'), path)
    }

    fn call(&self, agent: &ureq::Agent, method: &str, path: &str, body: Option<Value>) -> Result<Value, Fail> {
        if self.base.is_empty() || self.key.is_empty() {
            return Err(Fail::Other("The website connection is not set up on this computer.".into()));
        }
        let mut req = agent.request(method, &self.url(path)).set("X-NVO-Device", &self.key).set("Accept", "application/json");
        if let Some(t) = &self.token {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        let res = match body {
            Some(b) => req.send_json(b),
            None => req.call(),
        };
        match res {
            Ok(r) => r.into_json::<Value>().map_err(|e| Fail::Other(format!("Unexpected reply from the website ({e})."))),
            Err(ureq::Error::Status(code, r)) => {
                let v: Value = r.into_json().unwrap_or(Value::Null);
                let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("The website refused the request.").to_string();
                let api_code = v.get("code").and_then(|m| m.as_str()).unwrap_or("");
                if code == 401 && api_code == "nvo_session" || code == 403 && api_code == "nvo_session" {
                    Err(Fail::Session(msg))
                } else if code == 404 && api_code == "rest_no_route" {
                    Err(Fail::Other("The Northview Office plugin is not active on the website.".into()))
                } else {
                    Err(Fail::Other(msg))
                }
            }
            Err(ureq::Error::Transport(t)) => Err(Fail::Other(format!("Can't reach the website ({}).", t.kind()))),
        }
    }

    fn snapshot(&mut self, v: Result<Value, Fail>) -> Reply {
        match v {
            Ok(v) => Reply::Snapshot(serde_json::from_value::<Snapshot>(v).map_err(|e| format!("Unexpected data from the website ({e})."))),
            Err(Fail::Session(m)) => {
                self.token = None;
                Reply::SignedOut(m)
            }
            Err(Fail::Other(m)) => Reply::Snapshot(Err(m)),
        }
    }

    fn done(&mut self, v: Result<Value, Fail>) -> Reply {
        match v {
            Ok(_) => Reply::Done(Ok(())),
            Err(Fail::Session(m)) => {
                self.token = None;
                Reply::SignedOut(m)
            }
            Err(Fail::Other(m)) => Reply::Done(Err(m)),
        }
    }

    fn handle(&mut self, a: &ureq::Agent, cmd: Cmd) -> Option<Reply> {
        Some(match cmd {
            Cmd::Configure { url, key } => {
                self.base = url.trim().trim_end_matches('/').to_string();
                self.key = key.trim().to_string();
                self.token = None;
                return None;
            }
            Cmd::Staff => Reply::Staff(match self.call(a, "GET", "/staff", None) {
                Ok(v) => serde_json::from_value(v).map_err(|e| format!("Unexpected data from the website ({e}).")),
                Err(Fail::Session(m) | Fail::Other(m)) => Err(m),
            }),
            Cmd::Login { user_id, pin } => {
                self.token = None;
                Reply::Login(match self.call(a, "POST", "/session", Some(json!({ "user_id": user_id, "pin": pin }))) {
                    Ok(v) => {
                        self.token = v.get("token").and_then(|t| t.as_str()).map(String::from);
                        serde_json::from_value::<Staff>(v.get("staff").cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())
                    }
                    Err(Fail::Session(m) | Fail::Other(m)) => Err(m),
                })
            }
            Cmd::Logout => {
                if self.token.is_some() {
                    let _ = self.call(a, "DELETE", "/session", None);
                }
                self.token = None;
                return None;
            }
            Cmd::Sync => {
                if self.token.is_none() {
                    return None;
                }
                let r = self.call(a, "GET", "/sync", None);
                self.snapshot(r)
            }
            Cmd::Act(act) => {
                if self.token.is_none() {
                    return Some(Reply::SignedOut("Please sign in again.".into()));
                }
                match act {
                    Act::SetStatus { id, status, note, appt } => {
                        let body = json!({ "status": status.key(), "note": note, "appt_date": appt.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default() });
                        let r = self.call(a, "POST", &format!("/requests/{id}/status"), Some(body));
                        self.snapshot(r)
                    }
                    Act::Viewed(id) => {
                        let r = self.call(a, "POST", &format!("/requests/{id}/viewed"), Some(json!({})));
                        self.done(r)
                    }
                    Act::Copied(id) => {
                        let r = self.call(a, "POST", &format!("/requests/{id}/copied"), Some(json!({})));
                        self.done(r)
                    }
                    Act::FormAttached(id) => {
                        let r = self.call(a, "POST", &format!("/forms/{id}/attached"), Some(json!({})));
                        self.snapshot(r)
                    }
                    Act::FormPdf(id) => {
                        let r = self.call(a, "POST", &format!("/forms/{id}/pdf"), Some(json!({})));
                        self.done(r)
                    }
                    Act::FormLink { request_id, .. } => {
                        let r = self.call(a, "POST", "/forms/link", Some(json!({ "request_id": request_id })));
                        self.snapshot(r)
                    }
                    Act::MemberAdd { name, plan, enrolled, household, paid } => {
                        let body = json!({ "name": name, "plan": plan, "enrolled": enrolled.format("%Y-%m-%d").to_string(), "household": household, "payment": if paid { "paid" } else { "due" } });
                        let r = self.call(a, "POST", "/members", Some(body));
                        self.snapshot(r)
                    }
                    Act::Member { id, op } => {
                        let r = self.call(a, "POST", &format!("/members/{id}/{}", op.key()), Some(json!({})));
                        self.snapshot(r)
                    }
                    Act::VisitAdd { name, phone, email, date } => {
                        let body = json!({ "name": name, "phone": phone, "email": email, "visit_date": date.format("%Y-%m-%d").to_string() });
                        let r = self.call(a, "POST", "/visits", Some(body));
                        self.snapshot(r)
                    }
                    Act::Review { visit_id, channel } => {
                        let r = self.call(a, "POST", "/reviews", Some(json!({ "visit_id": visit_id, "channel": channel })));
                        self.snapshot(r)
                    }
                }
            }
        })
    }
}
