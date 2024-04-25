use std::io;

mod env;
mod rdb;
mod types;

use clap::Parser;
use env::environment;
use flate2::read::GzDecoder;
use once_cell::sync::OnceCell;
use rdb::rdb_writer;
const DEFAULT_REDIS_VERSION: u8 = 7;
const INVALID_TOML_ERROR: &str = "Invalid TOML for Redis";

static REDIS_VERSION: OnceCell<u8> = OnceCell::new();

#[derive(Parser, Debug)]
#[command(about = "rdbdump - CLI to stream a TOML file into rdb format")]
struct Args {
    #[arg(
        short = 'g',
        long = "gzipped",
        help = "Whether the input file is gzipped (default: false)",
        default_value_t = false
    )]
    gzipped: bool,
}

fn main() -> Result<(), io::Error> {
    REDIS_VERSION.set(environment::get_redis_version()).unwrap();
    let mut stdout_buffer = io::BufWriter::new(io::stdout());

    let args = Args::parse();
    if args.gzipped {
        let mut stdin_buffer = io::BufReader::new(GzDecoder::new(io::stdin()));
        let _ = rdb_writer::rdb_from_buffer(&mut stdin_buffer, &mut stdout_buffer);
    } else {
        let mut stdin_buffer = io::BufReader::new(io::stdin());
        let _ = rdb_writer::rdb_from_buffer(&mut stdin_buffer, &mut stdout_buffer);
    }
    Ok(())
}
