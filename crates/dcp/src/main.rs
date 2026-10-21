//! Droid Cache Proxy (`dcp`) command line entrypoint.
//!
//! Subcommands mirror `docs/04-contracts/api.md`: `serve` starts the gateway,
//! while `stats`, `analyze` and `report` are offline reporting tools.
//! Handlers are stubs; each returns an error until its phase lands.

use anyhow::anyhow;
use clap::{Parser, Subcommand, ValueEnum};
use tracing::info;

/// Droid Cache Proxy: local OpenAI-compatible reverse proxy.
#[derive(Debug, Parser)]
#[command(
    name = "dcp",
    version,
    about = "Droid Cache Proxy gateway and analysis CLI"
)]
struct Cli {
    /// Subcommand to run.
    #[command(subcommand)]
    command: Commands,
}

/// Available `dcp` subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Start the proxy gateway on 127.0.0.1:6789.
    Serve(ServeArgs),
    /// Show cache-hit baseline statistics from SQLite.
    Stats(StatsArgs),
    /// Analyze prefix divergence causes from logs (Phase 2).
    Analyze(AnalyzeArgs),
    /// Compare baseline vs evaluated periods (Phase 2).
    Report(ReportArgs),
}

/// Arguments for `dcp serve`.
#[derive(Debug, Parser)]
struct ServeArgs {
    /// Path to the TOML config file.
    #[arg(short, long)]
    config: Option<String>,
    /// Override the listen address, e.g. 127.0.0.1:6789.
    #[arg(short, long)]
    listen: Option<String>,
    /// Override the transform mode.
    #[arg(short, long, value_enum)]
    mode: Option<TransformMode>,
    /// Enable verbose debug logging.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

/// Arguments for `dcp stats`.
#[derive(Debug, Parser)]
struct StatsArgs {
    /// Path to the SQLite database file.
    #[arg(long)]
    db: Option<String>,
    /// Number of recent days to include.
    #[arg(long, default_value_t = 7)]
    days: u32,
    /// Filter statistics to a single model name.
    #[arg(long)]
    model: Option<String>,
    /// Emit machine-readable JSON output.
    #[arg(long, default_value_t = false)]
    json: bool,
}

/// Arguments for `dcp analyze`.
#[derive(Debug, Parser)]
struct AnalyzeArgs {
    /// Path to the SQLite database file.
    #[arg(long)]
    db: Option<String>,
    /// Directory containing daily JSONL log files.
    #[arg(long)]
    jsonl_dir: Option<String>,
    /// Restrict analysis to a single session id.
    #[arg(long)]
    session: Option<String>,
    /// Number of top divergence causes to display.
    #[arg(long, default_value_t = 10)]
    top: u32,
    /// Report output format.
    #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
    format: ReportFormat,
}

/// Arguments for `dcp report`.
#[derive(Debug, Parser)]
struct ReportArgs {
    /// Baseline date or session identifier.
    #[arg(long)]
    baseline: String,
    /// Evaluated date or session identifier.
    #[arg(long)]
    evaluated: String,
}

/// Transform pipeline operating mode.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum TransformMode {
    /// Phase 1 passthrough, no transformation.
    Off,
    /// Run the pipeline in memory, keep sending the original body.
    Shadow,
    /// Send the transformed body upstream.
    On,
}

/// Report output format for offline commands.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum ReportFormat {
    /// Human-readable terminal tables.
    #[default]
    Text,
    /// Machine-readable JSON.
    Json,
    /// Markdown report.
    Markdown,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli);

    match &cli.command {
        Commands::Serve(args) => run_serve(args),
        Commands::Stats(args) => run_stats(args),
        Commands::Analyze(args) => run_analyze(args),
        Commands::Report(args) => run_report(args),
    }
}

fn init_tracing(cli: &Cli) {
    let verbose = matches!(&cli.command, Commands::Serve(args) if args.verbose);
    let level = if verbose { "debug" } else { "info" };
    let result = tracing_subscriber::fmt().with_env_filter(level).try_init();
    if result.is_err() {
        eprintln!("tracing subscriber already initialized");
    }
}

fn run_serve(args: &ServeArgs) -> anyhow::Result<()> {
    info!(?args, "serve requested");
    Err(anyhow!("not implemented yet: dcp serve"))
}

fn run_stats(args: &StatsArgs) -> anyhow::Result<()> {
    info!(?args, "stats requested");
    Err(anyhow!("not implemented yet: dcp stats"))
}

fn run_analyze(args: &AnalyzeArgs) -> anyhow::Result<()> {
    info!(?args, "analyze requested");
    Err(anyhow!("not implemented yet: dcp analyze"))
}

fn run_report(args: &ReportArgs) -> anyhow::Result<()> {
    info!(?args, "report requested");
    Err(anyhow!("not implemented yet: dcp report"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
