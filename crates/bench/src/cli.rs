//! `orexis bench` — run and compare entry points (M8-10).

use std::path::PathBuf;
use std::time::Duration;

use crate::compare::run_compare;
use crate::runner::{run_suite, Mode, SuiteArgs};

/// `orexis bench run|compare …`. Flags follow the binary's hand-rolled
/// convention: `--name value`, booleans bare.
pub async fn cli(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args.first().map(String::as_str) {
        Some("run") => {
            let rest = &args[1..];
            let suite = SuiteArgs {
                tasks: PathBuf::from(value(rest, "--tasks").ok_or("bench run needs --tasks PATH")?),
                profile: value(rest, "--profile").ok_or("bench run needs --profile ID")?,
                model: value(rest, "--model").ok_or("bench run needs --model ID")?,
                modes: value(rest, "--modes")
                    .map(|list| {
                        list.split(',')
                            .filter_map(|mode| match mode.trim() {
                                "serial" => Some(Mode::Serial),
                                "team" => Some(Mode::Team),
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| vec![Mode::Serial, Mode::Team]),
                timeout: Duration::from_secs(
                    value(rest, "--timeout-secs")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(900),
                ),
                out: PathBuf::from(
                    value(rest, "--out").unwrap_or_else(|| "bench-results.jsonl".to_string()),
                ),
                append: rest.iter().any(|arg| arg == "--append"),
                providers_dir: value(rest, "--providers-dir")
                    .map(PathBuf::from)
                    .or_else(|| {
                        let default_dir = PathBuf::from("bench/providers");
                        if default_dir.exists() {
                            Some(default_dir)
                        } else {
                            None
                        }
                    }),
            };
            run_suite(&suite).await
        }
        Some("compare") => {
            let rest = &args[1..];
            let serial =
                PathBuf::from(value(rest, "--serial").ok_or("bench compare needs --serial FILE")?);
            let team =
                PathBuf::from(value(rest, "--team").ok_or("bench compare needs --team FILE")?);
            let report = run_compare(&serial, &team)?;
            println!("{report}");
            Ok(())
        }
        _ => {
            eprintln!(
                "usage: orexis bench run --tasks FILE --profile ID --model ID \
                 [--modes serial,team] [--out FILE] [--timeout-secs N] [--providers-dir PATH] [--append]\n\
                 \x20      orexis bench compare --serial FILE --team FILE"
            );
            std::process::exit(2);
        }
    }
}

fn value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}
