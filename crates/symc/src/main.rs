use std::fs;
use std::path::PathBuf;

use clap::{Args, Parser as ClapParser, Subcommand, ValueEnum};
use symc::{
    format_error, format_error_json, lexer, load_and_check, run_module, run_module_vm, LoadOptions,
    SymError,
};

#[derive(ClapParser)]
#[command(
    name = "sym",
    version,
    about = "Sym language: check and run .sym programs"
)]
struct Cli {
    #[command(flatten)]
    opts: GlobalOpts,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum MessageFormat {
    /// Traditional `path:line:col: message` on stderr
    #[default]
    Human,
    /// One JSON object per line (industrial roadmap step 7)
    Json,
}

#[derive(Args)]
struct GlobalOpts {
    /// Do not prepend stdlib/prelude.sym (if present)
    #[arg(long, global = true)]
    no_prelude: bool,
    /// Directory for fallback imports (default: ./stdlib)
    #[arg(long, global = true, value_name = "DIR")]
    stdlib: Option<PathBuf>,
    /// How to print errors on stderr
    #[arg(long, global = true, value_enum, default_value_t = MessageFormat::Human)]
    message_format: MessageFormat,
}

#[derive(Subcommand)]
enum Cmd {
    /// Type-check a file (and its imports)
    Check {
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    /// Parse, check, and run `main`
    Run {
        #[arg(value_name = "FILE")]
        path: PathBuf,
        /// Run via stack bytecode VM (same semantics as the tree interpreter when the program is VM-eligible)
        #[arg(long)]
        vm: bool,
    },
    /// Print lexer tokens (debug)
    Tokens {
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Check { path } => dispatch_check(path, &cli.opts),
        Cmd::Run { path, vm } => dispatch_run(path, vm, &cli.opts),
        Cmd::Tokens { path } => run_tokens(path, &cli.opts),
    }
}

fn emit_diag(path_str: &str, source: &str, err: &SymError, fmt: MessageFormat) {
    match fmt {
        MessageFormat::Human => eprintln!("{}", format_error(path_str, source, err)),
        MessageFormat::Json => eprintln!("{}", format_error_json(path_str, source, err)),
    }
}

fn run_tokens(path: PathBuf, g: &GlobalOpts) -> std::process::ExitCode {
    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("sym: {}: {e}", path.display());
            return std::process::ExitCode::from(1);
        }
    };
    match lexer::lex(&src) {
        Ok(tokens) => {
            for (t, span) in tokens {
                if matches!(t, lexer::Token::Eof) {
                    break;
                }
                println!(
                    "{span_start}..{span_end}\t{t:?}",
                    span_start = span.start,
                    span_end = span.end
                );
            }
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            let path_str = path.to_string_lossy().to_string();
            emit_diag(&path_str, &src, &SymError::Lex(e), g.message_format);
            std::process::ExitCode::from(1)
        }
    }
}

fn dispatch_check(path: PathBuf, g: &GlobalOpts) -> std::process::ExitCode {
    let path_str = path.to_string_lossy().to_string();
    let load_opts = LoadOptions {
        no_prelude: g.no_prelude,
        stdlib_root: g.stdlib.clone(),
    };

    match load_and_check(&path, &load_opts) {
        Ok(_) => {
            println!("OK: {}", path.display());
            std::process::ExitCode::SUCCESS
        }
        Err((e, stitched_src)) => {
            let src = stitched_src.unwrap_or_else(|| fs::read_to_string(&path).unwrap_or_default());
            emit_diag(&path_str, &src, &e, g.message_format);
            std::process::ExitCode::from(1)
        }
    }
}

fn dispatch_run(path: PathBuf, vm: bool, g: &GlobalOpts) -> std::process::ExitCode {
    let path_str = path.to_string_lossy().to_string();
    let load_opts = LoadOptions {
        no_prelude: g.no_prelude,
        stdlib_root: g.stdlib.clone(),
    };

    let (module, stitched) = match load_and_check(&path, &load_opts) {
        Ok(x) => x,
        Err((e, stitched_src)) => {
            let src = stitched_src.unwrap_or_else(|| fs::read_to_string(&path).unwrap_or_default());
            emit_diag(&path_str, &src, &e, g.message_format);
            return std::process::ExitCode::from(1);
        }
    };

    let run_result = if vm {
        run_module_vm(&module)
    } else {
        run_module(&module)
    };
    match run_result {
        Ok(v) => {
            println!("{}", v);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            emit_diag(&path_str, &stitched, &e, g.message_format);
            std::process::ExitCode::from(1)
        }
    }
}
