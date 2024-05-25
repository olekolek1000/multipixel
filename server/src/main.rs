use futures_util::{SinkExt, StreamExt};
use std::error::Error;

use tokio::net::{TcpListener, TcpStream};

use tokio_websockets::ServerBuilder;

extern crate pretty_env_logger;

mod config;
mod packet;
mod session;

async fn task_acceptor(tcp_conn: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut ws_conn = ServerBuilder::new().accept(tcp_conn).await?;

    while let Some(Ok(item)) = ws_conn.next().await {
        println!("Received: {item:?}");
    }

    Ok(())
}

async fn task_listener(listener: TcpListener) -> Result<(), Box<dyn Error>> {
    while let Ok((tcp_conn, _)) = listener.accept().await {
        if let Err(e) = task_acceptor(tcp_conn).await {
            log::error!("Connection error: {}", e);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "trace");
    pretty_env_logger::init();

    let config = config::load().await?;

    let listen_addr = format!("{}:{}", config.listen_ip, config.listen_port);
    log::info!("Listening to {}", listen_addr);

    let listener = TcpListener::bind(listen_addr).await?;

    if let Err(e) = task_listener(listener).await {
        log::error!("Listener error: {}", e);
    }

    Ok(())
}
