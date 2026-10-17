fn main() {
    println!("cargo:rerun-if-changed=assets/app_icon.ico");

    #[cfg(windows)]
    embed_windows_resources();
}

#[cfg(windows)]
fn embed_windows_resources() {
    use std::env;

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();

    res.set_icon("assets/app_icon.ico");

    res.set("ProductName", "Lightweight Autoclicker");
    res.set("FileDescription", "Lightweight Native Desktop Autoclicker");
    res.set("CompanyName", "OpenSource Desktop Tools");
    res.set("LegalCopyright", "Copyright (c) 2026");
    res.set("OriginalFilename", "autoclicker.exe");

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    let file_version = format!("{version}.0");
    res.set("FileVersion", &file_version);
    res.set("ProductVersion", &file_version);

    if let Err(e) = res.compile() {
        println!(
            "cargo:warning=winresource failed ({e}): .exe icon and VersionInfo skipped (rc.exe from the Windows SDK required)"
        );
    }
}
