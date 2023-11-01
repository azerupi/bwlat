use std::{net::IpAddr, path::PathBuf};

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use humantime::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, infer_subcommands = true)]
pub(crate) struct CliOptions {
    #[command(subcommand)]
    pub mode: Modes,

    #[command(flatten)]
    pub verbose: Verbosity<InfoLevel>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Modes {
    Server(ServerOptions),
    Client(ClientOptions),
}

#[derive(Parser, Debug)]
pub(crate) struct ServerOptions {
    #[arg(short, long)]
    pub port: u16,
}

#[derive(Parser, Debug)]
pub(crate) struct ClientOptions {
    pub address: IpAddr,
    pub port: u16,

    #[arg(long, default_value = "0")]
    pub client_port: u16,

    #[arg(short, long, default_value = "20ms")]
    pub interval: Duration,

    #[arg(short = 'z', long, default_value = "64")]
    pub packet_size: usize,

    #[arg(short, long, default_value = "100")]
    pub count: u32,

    #[arg(long)]
    pub csv: Option<PathBuf>,
}
