//! Desktop notifications from one background thread so the UI never stalls.
//! Notifications never contain patient details (Windows can show them on the lock screen).

use std::sync::mpsc::{channel, Sender};
use std::thread;

#[derive(Clone)]
pub struct Notifier {
    tx: Sender<bool>,
}

impl Notifier {
    pub fn start() -> Self {
        let (tx, rx) = channel::<bool>();
        thread::Builder::new()
            .name("notify".into())
            .spawn(move || {
                for emergency in rx {
                    let summary = if emergency { "Emergency appointment request" } else { "New appointment request" };
                    let mut n = notify_rust::Notification::new();
                    n.summary(summary).body("A request arrived from the website.");
                    #[cfg(target_os = "windows")]
                    n.app_id("SturlDigital.NorthviewOffice");
                    #[cfg(not(target_os = "windows"))]
                    n.appname("Northview Office");
                    let _ = n.show();
                }
            })
            .expect("notification thread");
        Self { tx }
    }

    pub fn new_request(&self, emergency: bool) {
        let _ = self.tx.send(emergency);
    }
}
