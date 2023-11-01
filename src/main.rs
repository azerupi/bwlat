mod action;
mod cli;
mod client;
mod components;
mod network;
mod server;
mod tui;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use cli::{CliOptions, ClientOptions, ServerOptions};
use client::Client;
use color_eyre::eyre::Result;
use server::Server;
use tracing::error;
use tracing_log::AsTrace;

use crate::tui::Tui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_options = CliOptions::parse();

    initialize_logging(&cli_options.verbose)?;
    initialize_panic_handler()?;

    match cli_options.mode {
        cli::Modes::Server(options) => run_server(options).await?,
        cli::Modes::Client(options) => run_client(options).await?,
    };

    Ok(())
}

fn initialize_logging(verbosity: &Verbosity<InfoLevel>) -> Result<()> {
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(verbosity.log_level_filter().as_trace())
        .init();

    Ok(())
}

pub fn initialize_panic_handler() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();

    eyre_hook.install()?;

    std::panic::set_hook(Box::new(move |panic_info| {
        if let Ok(mut t) = Tui::new() {
            if let Err(r) = t.exit() {
                error!("Unable to exit Terminal: {:?}", r);
            }
        }

        let msg = format!("{}", panic_hook.panic_report(panic_info));
        error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

async fn run_client(options: ClientOptions) -> Result<()> {
    let mut client = Client::new(
        options.address,
        options.port,
        options.client_port,
        options.packet_size,
        options.count,
    );

    client.set_interval(options.interval.into());

    if let Some(csv_path) = options.csv {
        client.enable_output_csv(csv_path);
    }

    client.run().await
}

async fn run_server(options: ServerOptions) -> Result<()> {
    let server = Server::new(options.port);

    server.run().await
}
