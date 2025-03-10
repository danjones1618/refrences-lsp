#![allow(clippy::print_stderr)]

use log::info;
use std::error::Error;
use stderrlog;

use lsp_types::{DocumentLinkOptions, OneOf, WorkDoneProgressOptions};
use lsp_types::{InitializeParams, ServerCapabilities};

use lsp_server::Connection;
use refrences_lsp::config::Config;
use refrences_lsp::Server;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    stderrlog::new()
        .module(module_path!())
        .verbosity(log::Level::Trace)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();
    // Note that  we must have our logging only write out to stderr.
    info!("starting generic LSP server");

    let config = Config::from_file().unwrap();
    info!("Using email {}", config.jira.email);

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    // let (connection, io_threads) =
    //     Connection::listen("localhost:9001").expect("Could not bind to address");
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
        inlay_hint_provider: Some(OneOf::Left(true)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
        }),
        ..Default::default()
    })
    .unwrap();
    let initialization_params: InitializeParams = match connection.initialize(server_capabilities) {
        Ok(it) => serde_json::from_value(it).unwrap(),
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    let mut server = Server::new(connection, initialization_params, &config);
    let _ = server.run_loop();
    io_threads.join()?;

    // Shut down gracefully.
    info!("shutting down server");
    Ok(())
}
