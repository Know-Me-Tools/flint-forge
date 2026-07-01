//! Forge ops + dev CLI: register functions, manage hooks, run migrations.
fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    if cmd == "version" {
        println!("forge {}", env!("CARGO_PKG_VERSION"));
    } else {
        eprintln!("forge <command>");
        eprintln!("  version            print version");
        eprintln!("  fn register <p>    register an edge component (TODO)");
        eprintln!("  hook add <table>   register a webhook (TODO)");
        eprintln!("  migrate            apply image/extension migrations (TODO)");
    }
}
