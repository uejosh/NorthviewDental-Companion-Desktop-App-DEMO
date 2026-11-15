//! Northview Dental Office App — front-desk companion for the practice website.
//!
//! Options:
//!   --data-dir <folder>      keep config and demo data in this folder
//!   --demo                   start with demo data regardless of saved settings
//!   --no-motion              turn off screen transitions and animations
//!   --screenshot <file.png>  render the app (demo data) to an image and exit
//!   --screen <name>          with --screenshot: signin | requests | forms | savings | reviews

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod api;
mod model;
mod motion;
mod notify;
mod pdf;
mod sample;
mod storage;
mod theme;
mod ui;
#[cfg(test)]
mod tests;

use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let mut opts = ui::Options { data_dir: None, screenshot: None, screen: None, force_demo: false, live_shot: false, no_motion: false, shot_mid: false };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--data-dir" => opts.data_dir = args.next().map(PathBuf::from),
            "--screenshot" => opts.screenshot = args.next().map(PathBuf::from),
            "--screen" => opts.screen = args.next(),
            "--demo" => opts.force_demo = true,
            "--no-motion" => opts.no_motion = true,
            #[cfg(debug_assertions)]
            "--live-screenshot" => opts.live_shot = true,
            #[cfg(debug_assertions)]
            "--screenshot-mid-transition" => opts.shot_mid = true,
            "--version" => {
                println!("Northview Office {}", ui::VERSION);
                return Ok(());
            }
            other => eprintln!("Ignoring unknown option {other}"),
        }
    }

    let icon = image::load_from_memory(include_bytes!("../assets/icon.png")).ok().map(|i| {
        let i = i.to_rgba8();
        egui::IconData { width: i.width(), height: i.height(), rgba: i.into_raw() }
    });
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Northview Dental — Office")
        .with_app_id("SturlDigital.NorthviewOffice")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1180.0, 740.0])
        .with_decorations(false)
        .with_transparent(false);
    if let Some(i) = icon {
        viewport = viewport.with_icon(i);
    }
    let native = eframe::NativeOptions { viewport, centered: true, persist_window: opts.screenshot.is_none(), ..Default::default() };
    eframe::run_native("Northview Office", native, Box::new(move |cc| Ok(Box::new(ui::App::new(cc, opts)))))
}
