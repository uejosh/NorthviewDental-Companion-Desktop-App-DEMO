//! Embeds the app icon and version details into the Windows executable, so Explorer,
//! shortcuts and the taskbar show the Northview icon. Does nothing on other platforms.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico")
        .set("ProductName", "Northview Office")
        .set("FileDescription", "Northview Dental front-desk app")
        .set("CompanyName", "Sturl Digital Services")
        .set("LegalCopyright", "Sturl Digital Services");
    if let Err(e) = res.compile() {
        panic!("could not embed the Windows icon: {e}");
    }
}
