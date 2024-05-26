use std::{error::Error, sync::Arc};

use futures_util::StreamExt;

use tokio::{
	net::{TcpListener, TcpStream},
	sync::Mutex,
};

use tokio_websockets::{ServerBuilder, WebSocketStream};

use crate::server::Server;

extern crate pretty_env_logger;

mod config;
mod id;
mod packet_client;
mod packet_server;
mod server;
mod session;

pub type Connection = WebSocketStream<TcpStream>;

async fn task_processor(
	tcp_conn: TcpStream,
	server: Arc<Mutex<Server>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
	let mut ws_conn = ServerBuilder::new().accept(tcp_conn).await?;

	let mut s = server.lock().await;
	let session_handle = s.create_session();
	log::info!("Created session with ID {}", session_handle.id());

	drop(s);

	while let Some(res) = ws_conn.next().await {
		match res {
			Ok(item) => {
				let mut s = server.lock().await;
				let session = s
					.sessions
					.get_mut(&session_handle)
					.ok_or("Session not found")?;

				if item.is_binary() {
					session
						.process_payload(session_handle.id(), item.as_payload(), &mut ws_conn)
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

	s = server.lock().await;
	s.sessions.remove(&session_handle);

	Ok(())
}

async fn task_listener(listener: TcpListener, server_obj: Server) -> Result<(), Box<dyn Error>> {
	let server = Arc::new(Mutex::new(server_obj));

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
async fn main() -> Result<(), Box<dyn Error>> {
	std::env::set_var("RUST_LOG", "trace");
	pretty_env_logger::init();

	let config = config::load().await?;

	let listen_addr = format!("{}:{}", config.listen_ip, config.listen_port);
	log::info!("Listening to {}", listen_addr);

	let listener = TcpListener::bind(listen_addr).await?;

	let server = Server::new();

	if let Err(e) = task_listener(listener, server).await {
		log::error!("Listener error: {}", e);
	}

	Ok(())
}
