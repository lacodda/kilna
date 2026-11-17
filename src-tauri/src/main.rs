// A console window would appear behind the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `kilna --mcp` is the same build without a window: the spaces served to
    // an agent on stdin/stdout. `--workspace <dir>` points it at another
    // application directory — a test's, or a copy kept for a rehearsal.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--mcp") {
        std::process::exit(match serve_mcp(&args) {
            Ok(()) => 0,
            Err(err) => {
                // Both: the file for later, and stderr for the agent's host,
                // which shows a server's stderr when it will not start.
                kilna_lib::log::error("mcp", &err.to_string());
                eprintln!("kilna --mcp: {err}");
                1
            }
        });
    }
    // `kilna --rehome-media` moves a space's folders on disk to where its
    // profile's templates now put them (FR-04): a plan and a report unless
    // `--apply` is said; `--also <dir>` names a folder outside the media
    // folder to look into, `--space <name>` a space other than the last open.
    if args.iter().any(|arg| arg == "--rehome-media") {
        std::process::exit(match rehome(&args) {
            Ok(report) => {
                println!("{}", report.display());
                0
            }
            Err(err) => {
                eprintln!("kilna --rehome-media: {err}");
                1
            }
        });
    }
    // A `space.kilna` double-clicked, or a `kilna://` link followed, arrives
    // as an argument (ADR 0077).
    let target = kilna_lib::commands::spaces::target_in(&args);
    kilna_lib::run_in(workspace_arg(&args), target)
}

/// The directory `--workspace <dir>` names, when it does.
fn workspace_arg(args: &[String]) -> Option<std::path::PathBuf> {
    args.iter()
        .position(|arg| arg == "--workspace")
        .and_then(|at| args.get(at + 1))
        .map(std::path::PathBuf::from)
}

/// Every value given after `flag`, one for each time it is said.
fn values_of<'a>(args: &'a [String], flag: &str) -> Vec<&'a String> {
    args.windows(2)
        .filter(|pair| pair[0] == flag)
        .map(|pair| &pair[1])
        .collect()
}

fn rehome(args: &[String]) -> kilna_lib::Result<std::path::PathBuf> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    let also: Vec<std::path::PathBuf> = values_of(args, "--also")
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect();
    let space = values_of(args, "--space").first().map(|name| name.as_str());
    kilna_lib::exchange::rehome::run(
        &app_dir,
        space,
        args.iter().any(|arg| arg == "--apply"),
        &also,
    )
}

fn serve_mcp(args: &[String]) -> kilna_lib::Result<()> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    kilna_lib::log::init(&app_dir, kilna_lib::log::MCP_FILE);
    kilna_lib::mcp::serve(&app_dir)
}
