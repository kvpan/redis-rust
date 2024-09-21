use clap::Parser;

use crate::config;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long, default_value = "/tmp/redis-data")]
    dir: String,
    #[arg(long, default_value = "dump.rdb")]
    dbfilename: String,
}

pub fn init() {
    let cli = Cli::parse();
    config::set_dir(&cli.dir);
    config::set_dbfilename(&cli.dbfilename);
}
