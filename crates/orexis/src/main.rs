//! The `orexis` binary: one entry point, subcommands (ADR-0019, ticket
//! 019). The argument surface is hand-rolled — the workspace carries no
//! CLI parser, and the subcommands are few.

use std::path::PathBuf;

use orexis_protocol::ServerMsg;

mod minisign;
mod providers;
mod upgrade;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Command::Serve => runtime()?.block_on(orexis_daemon::serve_main()),
        Command::Acp => runtime()?.block_on(orexis_daemon::acp_main()),
        Command::Tui { socket, session } => orexis_adapters_tui::run(socket, session)
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>),
        Command::Attach { socket, session } => attach(socket, session),
        Command::Providers { detail } => providers(detail),
        Command::Init { profile, model } => init(profile, model),
        Command::Upgrade { from } => crate::upgrade::upgrade(from.as_deref())
            .map(|text| print!("{text}"))
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>),
        Command::Version => {
            println!("orexis {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::NotYet(name) => {
            eprintln!("not yet: {name} has no implementation; see .scratch/orexis/issues/");
            std::process::exit(2);
        }
        Command::Usage => {
            usage();
            Ok(())
        }
    }
}

fn providers(detail: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = orexis_composition::load_catalog()?;
    match detail {
        Some(id) => match crate::providers::show(&catalog, &id) {
            Some(text) => print!("{text}"),
            None => {
                eprintln!("unknown profile: {id}");
                std::process::exit(2);
            }
        },
        None => print!("{}", crate::providers::list(&catalog)),
    }
    Ok(())
}

fn init(profile: Option<String>, model: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let project_root = match std::env::var("OREXIS_PROJECT") {
        Ok(project) => PathBuf::from(project),
        Err(_) => std::env::current_dir()?,
    };
    let text = match (profile, model) {
        (Some(profile), Some(model)) => {
            crate::providers::init_selection(&project_root, &profile, &model)?
        }
        (None, None) => crate::providers::init_interactive(&project_root)?,
        _ => {
            eprintln!("init needs both --profile and --model, or neither (interactive)");
            std::process::exit(2);
        }
    };
    print!("{text}");
    Ok(())
}

fn runtime() -> Result<tokio::runtime::Runtime, Box<dyn std::error::Error>> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

/// The subcommands, parsed from argv. Unit-tested; `main` only executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Serve,
    Acp,
    Tui {
        socket: PathBuf,
        session: Option<String>,
    },
    Attach {
        socket: PathBuf,
        session: Option<String>,
    },
    Upgrade {
        from: Option<PathBuf>,
    },
    Version,
    /// A subcommand whose ticket has not landed; named in the error.
    NotYet(&'static str),
    Providers {
        detail: Option<String>,
    },
    Init {
        profile: Option<String>,
        model: Option<String>,
    },
    Usage,
}

fn parse(args: &[String]) -> Command {
    let Some(name) = args.first() else {
        return Command::Usage;
    };
    let flags = &args[1..];
    match name.as_str() {
        "serve" => Command::Serve,
        "acp" => Command::Acp,
        "tui" | "attach" => {
            let (socket, session) = socket_and_session(flags);
            if name == "tui" {
                Command::Tui { socket, session }
            } else {
                Command::Attach { socket, session }
            }
        }
        "providers" => Command::Providers {
            detail: flags.first().cloned(),
        },
        "init" => Command::Init {
            profile: flag_value(flags, "--profile"),
            model: flag_value(flags, "--model"),
        },
        "upgrade" => Command::Upgrade {
            from: flag_value(flags, "--from").map(PathBuf::from),
        },
        "--version" | "-V" | "version" => Command::Version,
        "bench" => Command::NotYet("bench"),
        "export" => Command::NotYet("export"),
        "import" => Command::NotYet("import"),
        "--help" | "-h" | "help" => Command::Usage,
        other => {
            eprintln!("unknown subcommand: {other}");
            usage();
            std::process::exit(2);
        }
    }
}

/// The value of a `--flag VALUE` pair, if present.
fn flag_value(flags: &[String], flag: &str) -> Option<String> {
    flags
        .iter()
        .position(|candidate| candidate == flag)
        .and_then(|index| flags.get(index + 1))
        .cloned()
}

/// `--socket PATH` (default `/tmp/orexis.sock`) and an optional session
/// id positional.
fn socket_and_session(flags: &[String]) -> (PathBuf, Option<String>) {
    let mut socket = PathBuf::from(
        std::env::var("OREXIS_SOCKET").unwrap_or_else(|_| "/tmp/orexis.sock".to_string()),
    );
    let mut session = None;
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match flag.as_str() {
            "--socket" => {
                if let Some(path) = flags.next() {
                    socket = PathBuf::from(path);
                }
            }
            "--session" => {
                if let Some(id) = flags.next() {
                    session = Some(id.clone());
                }
            }
            _ => session = Some(flag.clone()),
        }
    }
    (socket, session)
}

fn usage() {
    eprintln!(
        "usage: orexis <command>\n\n\
         commands:\n\
         \x20 serve            run the daemon (OREXIS_* env configures it)\n\
         \x20 acp              serve the ACP agent over stdio\n\
         \x20 tui [--socket P] [SESSION]\n\
         \x20                  attach the terminal UI\n\
         \x20 attach [--socket P] [SESSION]\n\
         \x20                  pipe client: stdin lines are prompts, events print as NDJSON\n\
         \x20 providers [ID]   list profiles (auth buildability visible) or show one\n\
         \x20 init [--profile P --model M]\n\
         \x20                  select a provider; interactive when flags are absent\n\
         \x20 upgrade [--from DIR]\n\
         \x20                  verify and install the newest release; state untouched\n\
         \x20 export | import | bench\n\
         \x20                  not yet (M9)"
    );
}

/// The pipe client: stdin lines become prompts; server messages print as
/// NDJSON — scriptable oversight without a terminal UI.
fn attach(socket: PathBuf, session: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = orexis_client::DaemonClient::connect(&socket)?;
    let session_id = client.attach(session)?;
    println!(
        "{}",
        serde_json::to_string(&ServerMsg::Attached {
            session_id: session_id.clone(),
        })?
    );
    let stdin = std::io::stdin();
    let mut prompt = String::new();
    loop {
        while let Some(message) = client.try_recv()? {
            println!("{}", serde_json::to_string(&message)?);
        }
        prompt.clear();
        if stdin.read_line(&mut prompt)? == 0 {
            return Ok(());
        }
        let line = prompt.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/stop" {
            client.interrupt()?;
            continue;
        }
        client.prompt(line, false)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn subcommands_route() {
        assert_eq!(parse(&args(&["serve"])), Command::Serve);
        assert_eq!(parse(&args(&["acp"])), Command::Acp);
        assert_eq!(
            parse(&args(&["tui", "--socket", "/tmp/x.sock"])),
            Command::Tui {
                socket: PathBuf::from("/tmp/x.sock"),
                session: None,
            }
        );
        assert_eq!(
            parse(&args(&["tui", "session-1"])),
            Command::Tui {
                socket: PathBuf::from("/tmp/orexis.sock"),
                session: Some("session-1".to_string()),
            }
        );
        assert_eq!(
            parse(&args(&["attach"])),
            Command::Attach {
                socket: PathBuf::from("/tmp/orexis.sock"),
                session: None,
            }
        );
    }

    #[test]
    fn unbuilt_subcommands_report_not_yet() {
        for name in ["bench", "export", "import"] {
            assert!(
                matches!(parse(&args(&[name])), Command::NotYet(_)),
                "{name} must be a NotYet stub"
            );
        }
    }

    #[test]
    fn providers_and_init_parse() {
        assert_eq!(
            parse(&args(&["providers"])),
            Command::Providers { detail: None }
        );
        assert_eq!(
            parse(&args(&["providers", "openai"])),
            Command::Providers {
                detail: Some("openai".to_string())
            }
        );
        assert_eq!(
            parse(&args(&["init", "--profile", "openai", "--model", "gpt-5"])),
            Command::Init {
                profile: Some("openai".to_string()),
                model: Some("gpt-5".to_string()),
            }
        );
        assert_eq!(
            parse(&args(&["init"])),
            Command::Init {
                profile: None,
                model: None
            }
        );
    }

    #[test]
    fn no_arguments_is_usage() {
        assert_eq!(parse(&[]), Command::Usage);
        assert_eq!(parse(&args(&["--help"])), Command::Usage);
    }
}
