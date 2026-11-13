//! Expose the same STKCraft-time dependency's MCP tools to the web server.
fn main() {
    if let Err(error) = stkcraft_time::mcp::run_stdio() {
        eprintln!("time MCP: {error}");
        std::process::exit(1);
    }
}
