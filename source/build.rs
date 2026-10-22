// Embeds the app icon and version info into wiggl3.exe on Windows. A failure here must never stop the
// build: the app works without the embedded icon (the window sets its own icon at run time).
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "wiggl3");
        res.set("FileDescription", "wiggl3");
        res.set("CompanyName", "wiggl3");
        res.set("LegalCopyright", "MIT licence");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
