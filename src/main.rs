use clap::Parser;
use rssh::cli::{Cli, Command, ResolvedCommand};
use rssh::session::supervisor::Supervisor;

fn main() {
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .init();

    let exit_code = match cli.resolved_command() {
        ResolvedCommand::Subcommand(Command::Connect {
            ref host,
            auto_drop,
            ref profile,
        }) => {
            tracing::info!(host = %host, auto_drop, profile = %profile, "connecting");
            run_connect(host)
        }
        ResolvedCommand::Subcommand(Command::Doctor { ref host }) => run_doctor(host.as_deref()),
        ResolvedCommand::Subcommand(Command::Status { ref session }) => {
            eprintln!("Status for session {session} — not yet implemented");
            0
        }
        ResolvedCommand::Subcommand(Command::Gc {
            ref host,
            dry_run: _,
            apply: _,
        }) => {
            eprintln!("GC for {host} — not yet implemented");
            0
        }
        ResolvedCommand::NoArgs => {
            Cli::parse_from(["rssh", "--help"]);
            0
        }
    };

    std::process::exit(exit_code);
}

fn run_connect(host: &str) -> i32 {
    match Supervisor::new(host) {
        Ok(mut sup) => match sup.run() {
            Ok(code) => code,
            Err(e) => {
                eprintln!("rssh: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("rssh: {e}");
            1
        }
    }
}

fn run_doctor(host: Option<&str>) -> i32 {
    println!("rssh doctor");
    println!();

    print_check("SSH binary", check_ssh_binary());
    print_check("Terminal is TTY", check_is_tty());

    if let Some(host) = host {
        println!();
        println!("Remote checks for '{host}' are not yet implemented.");
    }

    0
}

fn print_check(name: &str, ok: bool) {
    let status = if ok { "ok" } else { "FAIL" };
    println!("  [{status}] {name}");
}

fn check_ssh_binary() -> bool {
    std::path::Path::new("/usr/bin/ssh").exists()
}

fn check_is_tty() -> bool {
    nix::unistd::isatty(std::io::stdin()).unwrap_or(false)
}
