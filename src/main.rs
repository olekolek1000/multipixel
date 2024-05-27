use std::sync::Arc;

use futures_util::{
	stream::{SplitSink, SplitStream},
	StreamExt,
};

use server::ServerMutex;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::Mutex,
};

use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

use crate::{server::Server, session::SessionInstance};

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
pub type ConnectionWriter = SplitSink<WebSocketStream<TcpStream>, Message>;
pub type ConnectionReader = SplitStream<WebSocketStream<TcpStream>>;
pub type ConnectionMutex = Arc<Mutex<Connection>>;

async fn task_processor(tcp_conn: TcpStream, server_mtx: ServerMutex) -> anyhow::Result<()> {
	let connection = ServerBuilder::new().accept(tcp_conn).await?;
	let (writer, mut reader) = connection.split();

	let mut server = server_mtx.lock().await;
	let (session_handle, session_mtx) = server.create_session();
	drop(server);
	log::info!("Created session with ID {}", session_handle.id());

	// Spawn sender task
	SessionInstance::launch_sender_task(Arc::downgrade(&session_mtx), writer);

	loop {
		match reader.next().await {
			Some(res) => match res {
				Ok(msg) => {
					if msg.is_binary() {
						let mut session = session_mtx.lock().await;
						session
							.process_payload(&session_handle, msg.as_payload(), &server_mtx)
							.await?;
					} else {
						log::trace!("Got unknown message");
					}
				}
				Err(e) => {
					log::info!("Session {} error: {}", session_handle.id(), e);
				}
			},
			None => {
				// Just exit
				log::info!("reader.next() returned nothing, exiting");
				break;
			}
		}
	}

	// Remove session
	session_mtx.lock().await.cleanup(&session_handle).await;

	server = server_mtx.lock().await;
	server.remove_session(&session_handle).await;
	drop(server);

	Ok(())
}

async fn task_listener(listener: TcpListener) -> anyhow::Result<()> {
	let server = Server::new();

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

async fn run() -> anyhow::Result<()> {
	let config = config::load().await?;

	let listen_addr = format!("{}:{}", config.listen_ip, config.listen_port);
	log::info!("Listening to {}", listen_addr);

	let listener = TcpListener::bind(listen_addr).await?;

	if let Err(e) = task_listener(listener).await {
		log::error!("Listener error: {}", e);
	}

	Ok(())
}

fn main() {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(2)
		.thread_name("mp")
		.thread_stack_size(2 * 1024 * 1024)
		.enable_io()
		.build()
		.unwrap();

	console_subscriber::init();

	std::env::set_var("RUST_LOG", "trace");
	pretty_env_logger::init();

	if let Err(e) = runtime.block_on(run()) {
		log::error!("{}", e);
	}

	log::info!("Exiting");
}
