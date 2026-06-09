mod cli;
mod types;
mod ingest;
mod normalize;
mod tokenize;
mod segment;
mod rank;
mod select;
mod schema;
mod render;
mod cache;
mod utils;

use cli::Cli;
use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
