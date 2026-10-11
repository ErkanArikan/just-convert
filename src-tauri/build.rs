fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        link_clang_runtime();
    }
    tauri_build::build()
}

// whisper.cpp's Metal backend uses `@available` checks whose helper lives in clang's runtime
// library, which rustc does not always link (it passes -nodefaultlibs).
fn link_clang_runtime() {
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let Ok(output) = std::process::Command::new(compiler)
        .arg("--print-runtime-dir")
        .output()
    else {
        return;
    };
    let directory = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if std::path::Path::new(&directory)
        .join("libclang_rt.osx.a")
        .is_file()
    {
        println!("cargo:rustc-link-search=native={directory}");
        println!("cargo:rustc-link-lib=static=clang_rt.osx");
    }
}
