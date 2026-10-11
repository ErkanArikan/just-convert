// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--mcp") {
        if let Err(message) = just_convert_lib::mcp::serve() {
            eprintln!("just-convert --mcp: {message}");
            std::process::exit(1);
        }
        return;
    }
    just_convert_lib::run()
}
