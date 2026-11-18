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
    // `kilna --import-ideas <file>` brings a bank of phrases kept as markdown
    // into a space (FR-09 9.3), and with `--documents-tagged <a,b>` marks the
    // notes carrying those tags as documents: a plan and a report unless
    // `--apply` is said; `--space <name>` a space other than the last open.
    if args.iter().any(|arg| arg == "--import-ideas") {
        std::process::exit(match import_ideas(&args) {
            Ok(report) => {
                println!("{}", report.display());
                0
            }
            Err(err) => {
                eprintln!("kilna --import-ideas: {err}");
                1
            }
        });
    }
    // `kilna --import-songs <dir>` brings the folders of songs a previous
    // system kept (FR-10 10.1): the sections of each `info.md` and its pages
    // as notes on the work, the section `--images-heading <name>` names as
    // phrases - a plan and a report unless `--apply` is said.
    if args.iter().any(|arg| arg == "--import-songs") {
        std::process::exit(match import_songs(&args) {
            Ok(report) => {
                println!("{}", report.display());
                0
            }
            Err(err) => {
                eprintln!("kilna --import-songs: {err}");
                1
            }
        });
    }
    // `kilna --demo-all` makes a demo stand in the application folder (or the
    // one `--workspace` names): a space of every shipped craft with its demo,
    // wiped and made again on the next call. `kilna --seed-demo` fills one
    // space - the last opened, or `--space <name>` - while it holds no works.
    if args.iter().any(|arg| arg == "--demo-all") {
        std::process::exit(match demo_all(&args) {
            Ok(made) => {
                for (name, report) in made {
                    println!(
                        "{name}: {} works, {} notes, {} releases",
                        report.works, report.notes, report.releases
                    );
                }
                0
            }
            Err(err) => {
                eprintln!("kilna --demo-all: {err}");
                1
            }
        });
    }
    if args.iter().any(|arg| arg == "--seed-demo") {
        std::process::exit(match seed_demo(&args) {
            Ok((name, report)) => {
                println!(
                    "{name}: {} works, {} notes, {} releases",
                    report.works, report.notes, report.releases
                );
                0
            }
            Err(err) => {
                eprintln!("kilna --seed-demo: {err}");
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

fn import_ideas(args: &[String]) -> kilna_lib::Result<std::path::PathBuf> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    let file = values_of(args, "--import-ideas")
        .first()
        .map(std::path::PathBuf::from)
        .ok_or_else(|| kilna_lib::Error::refused("import.noFile"))?;
    let tagged: Vec<String> = values_of(args, "--documents-tagged")
        .into_iter()
        .flat_map(|list| list.split(',').map(|tag| tag.trim().to_owned()))
        .filter(|tag| !tag.is_empty())
        .collect();
    let space = values_of(args, "--space").first().map(|name| name.as_str());
    kilna_lib::exchange::ideas::run(
        &app_dir,
        space,
        &file,
        &tagged,
        args.iter().any(|arg| arg == "--apply"),
    )
}

fn import_songs(args: &[String]) -> kilna_lib::Result<std::path::PathBuf> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    let root = values_of(args, "--import-songs")
        .first()
        .map(std::path::PathBuf::from)
        .ok_or_else(|| kilna_lib::Error::refused("import.noFile"))?;
    let images = values_of(args, "--images-heading")
        .first()
        .map(|name| name.as_str());
    let space = values_of(args, "--space").first().map(|name| name.as_str());
    kilna_lib::exchange::songs::run(
        &app_dir,
        space,
        &root,
        images,
        args.iter().any(|arg| arg == "--apply"),
    )
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

fn demo_all(args: &[String]) -> kilna_lib::Result<Vec<(String, kilna_lib::demo::DemoReport)>> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    kilna_lib::demo::stand(&app_dir)
}

fn seed_demo(args: &[String]) -> kilna_lib::Result<(String, kilna_lib::demo::DemoReport)> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    let space = values_of(args, "--space").first().map(|name| name.as_str());
    kilna_lib::demo::seed_space(&app_dir, space)
}

fn serve_mcp(args: &[String]) -> kilna_lib::Result<()> {
    let app_dir = match workspace_arg(args) {
        Some(dir) => dir,
        None => kilna_lib::space::app_dir()?,
    };
    kilna_lib::log::init(&app_dir, kilna_lib::log::MCP_FILE);
    kilna_lib::mcp::serve(&app_dir)
}
