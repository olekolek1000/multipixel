use std::sync::Arc;

use futures_util::StreamExt;

use server::ServerMutex;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::Mutex,
};

use tokio_websockets::{ServerBuilder, WebSocketStream};

use crate::server::Server;

extern crate pretty_env_logger;

mod config;
mod id;
mod limits;
mod packet_client;
mod packet_server;
mod room;
mod server;
mod session;

pub type Connection = WebSocketStream<TcpStream>;

async fn task_processor(tcp_conn: TcpStream, server_mtx: ServerMutex) -> anyhow::Result<()> {
	let mut ws_conn = ServerBuilder::new().accept(tcp_conn).await?;

	let mut server = server_mtx.lock().await;
	let session_handle = server.create_session();
	log::info!("Created session with ID {}", session_handle.id());
	drop(server);

	while let Some(res) = ws_conn.next().await {
		match res {
			Ok(item) => {
				let mut server = server_mtx.lock().await;
				let session_mtx = server
					.sessions
					.get_mut(&session_handle)
					.ok_or(anyhow::anyhow!("Session not found"))?
					.clone();
				drop(server);

				if item.is_binary() {
					let mut session = session_mtx.lock().await;
					session
						.process_payload(
							session_handle.id(),
							item.as_payload(),
							&mut ws_conn,
							&server_mtx,
						)
						.await?;
				}
			}
			Err(e) => {
				// Never triggered yet in my tests
				log::info!("Session {} error: {}", session_handle.id(), e);
			}
		}
	}

	// Remove session
	log::info!("Removing session {}", session_handle.id());

	server = server_mtx.lock().await;
	server.sessions.remove(&session_handle);

	Ok(())
}

async fn task_listener(listener: TcpListener) -> anyhow::Result<()> {
	let server = Arc::new(Mutex::new(Server::new()));

	while let Ok((tcp_conn, _)) = listener.accept().await {
		let s = server.clone();
		tokio::spawn(async move {
			if let Err(e) = task_processor(tcp_conn, s).await {
				log::error!("task_processor: {}", e);
			}
		});
	}

	Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	std::env::set_var("RUST_LOG", "trace");
	pretty_env_logger::init();

	let config = config::load().await?;

	let listen_addr = format!("{}:{}", config.listen_ip, config.listen_port);
	log::info!("Listening to {}", listen_addr);

	let listener = TcpListener::bind(listen_addr).await?;

	if let Err(e) = task_listener(listener).await {
		log::error!("Listener error: {}", e);
	}

	log::info!("Exiting");

	Ok(())
}
