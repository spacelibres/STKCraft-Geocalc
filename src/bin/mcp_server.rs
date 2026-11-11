fn main() {
    if let Err(error) = stkcraft_geocalc::mcp::run_stdio() {
        eprintln!("stkcraft-geocalc-mcp: {error}");
        std::process::exit(1);
    }
}
