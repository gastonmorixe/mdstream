use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = mdstream::cli::Cli::parse();
    match mdstream::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Broken pipe is normal when the downstream consumer (less, head,
            // a closed terminal) goes away. Exit cleanly.
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|err| err.kind() == std::io::ErrorKind::BrokenPipe)
            {
                return ExitCode::SUCCESS;
            }
            // Stdin attached to a terminal: the help banner has already been
            // written by `handle_tty_check`. Exit non-zero without printing
            // anything else (matches Python `sys.exit(1)` at mdstream.py:913).
            if error.is::<mdstream::StdinIsTerminal>() {
                return ExitCode::FAILURE;
            }
            eprintln!("mdstream: {error:#}");
            ExitCode::FAILURE
        }
    }
}
