use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "nusku",
    about = "Real-time native process profiler for Linux",
    version
)]
pub struct Args {
    /// Binary to run directly
    #[arg(value_name = "BINARY", conflicts_with_all = ["pid", "command"])]
    pub binary: Option<String>,
    /// Attach to a running process by PID
    #[arg(short = 'p', long, conflicts_with = "command")]
    pub pid: Option<u32>,

    /// Command to launch and profile shell command (e.g. node ./server)
    #[arg(short = 'c', long, value_name = "COMMAND", conflicts_with = "pid", num_args = 1..)]
    pub command: Option<Vec<String>>,

    /// Sampling rate in Hz (default: 99)
    #[arg(long, default_value = "99")]
    pub rate: u64,

    /// Stop after N seconds (default: run until Ctrl+C)
    #[arg(short = 'd', long)]
    pub duration: Option<u64>,
}
