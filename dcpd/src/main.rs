use anyhow::Result;
use clap::Parser;
use dcpd::DaemonArgs;

#[tokio::main]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();
    dcpd::run(args).await
}
