fn main() {
    if let Err(message) = just_convert_lib::mcp::serve() {
        eprintln!("just-convert-mcp: {message}");
        std::process::exit(1);
    }
}
