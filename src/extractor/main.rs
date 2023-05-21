use file_verification_code::extract;

use clap::Parser;
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(version)] // causes version to be read from Cargo.toml
#[command(disable_version_flag=true)] // since we use v for verbosity, we need to manually define the version flag
#[command(about="Extract archives")]
#[command(arg_required_else_help=true)]
struct CLI {
    #[arg(long, action=clap::ArgAction::Version)] // manually define --version flag since we are using v for verbosity
    // since neither -h nor --help are in use, help arg is auto-generated

    #[arg(short='v', long="verbose", help="Include more v's for higher verbosity", action=clap::ArgAction::Count)]
    verbose: u8,
    #[arg(help="source [target]")]
    files: Vec<std::path::PathBuf>,
}

fn main() {
    let cli = CLI::parse(); // parse command line

    // initialize logger
    stderrlog::new()
        .module(module_path!())
        .verbosity(match cli.verbose {
            0 => log::Level::Warn, // Start with Error and Warn
            1 => log::Level::Info,
            2 => log::Level::Debug,
            _ => log::Level::Trace // 3 or higher
        })
        .timestamp(match cli.verbose {
            0 | 1 => stderrlog::Timestamp::Off,
            2 => stderrlog::Timestamp::Second,
            _ => stderrlog::Timestamp::Millisecond // 3 or higher
        })
        .init()
        .expect("initializing logger");

    if cli.files.len() > 1 {
        // was given a source and target
        let source = &cli.files[0];
        let target = &cli.files[1];
        log::info!("extracting {} to {}", source.display(), target.display());
        match extract::extract_archive(source, target) {
            Ok(()) => {
                log::info!("extracted archive {} to {}", source.display().to_string().italic(), target.display().to_string().italic());
            },
            Err(err) => {
                log::error!("error extracting {} to {}: {}", source.display().to_string().italic(), target.display().to_string().italic(), err);
                std::process::exit(1);
            }
        }
    } else {
        let source = &cli.files[0];
        // extract source to a temporary directory
        let tmp_prefix = match source.file_name() {
            Some(file_name) => format!("extractor.{:?}", file_name),
            None => format!("extractor.{:?}", source)
        };
        let tmp = tempdir::TempDir::new(&tmp_prefix).expect("creating temporary directory");

        log::info!("extracting {} to {}", source.display().to_string().italic(), tmp.path().display().to_string().italic());
        match extract::extract_archive(&source, tmp.as_ref()) {
            Ok(()) => {
                log::info!("extracted archive {} to {}", source.display(), tmp.path().display());
            },
            Err(err) => {
                log::error!("error extracting {}: {}", source.display().to_string().italic(), err);
                tmp.close().expect("error cleaning up temporary directory");
                std::process::exit(1);
            }
        }
    }
}