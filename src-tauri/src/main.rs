// A console window would appear behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `kilna --mcp` is the same build without a window: the workspace served
    // to an agent on stdin/stdout. `--workspace <dir>` points it at another
    // workspace directory — a test's, or a second one kept elsewhere.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--mcp") {
        std::process::exit(match serve_mcp(&args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("kilna --mcp: {err}");
                1
            }
        });
    }
    kilna_lib::run_in(workspace_arg(&args))
}

/// The directory `--workspace <dir>` names, when it does.
fn workspace_arg(args: &[String]) -> Option<std::path::PathBuf> {
    args.iter()
        .position(|arg| arg == "--workspace")
        .and_then(|at| args.get(at + 1))
        .map(std::path::PathBuf::from)
}

fn serve_mcp(args: &[String]) -> kilna_lib::Result<()> {
    let workspace = workspace_arg(args);
    let data_dir = match workspace {
        Some(dir) => dir,
        None => kilna_lib::db::default_data_dir()?,
    };
    let state = kilna_lib::state::AppState::open(&kilna_lib::db::default_path(&data_dir))?;
    let conn = state.conn();
    kilna_lib::mcp::serve(&conn)
}
