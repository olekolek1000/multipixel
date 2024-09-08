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

mod canvas_cache;
mod chunk;
mod chunk_cache;
mod chunk_system;
mod compression;
mod config;
mod database;
mod event_queue;
mod id;
mod limits;
mod packet_client;
mod packet_server;
mod pixel;
mod preview_system;
mod room;
mod server;
mod session;
mod time;
mod tool;
mod util;

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

	{
		let mut session_instance = session_mtx.lock().await;

		// Spawn sender task
		SessionInstance::launch_sender_task(
			session_handle.id(),
			&mut session_instance,
			Arc::downgrade(&session_mtx),
			writer,
		);

		// Spawn tick task
		SessionInstance::launch_tick_task(
			session_handle,
			&mut session_instance,
			Arc::downgrade(&session_mtx),
		);
	}

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

	let weak_session = Arc::downgrade(&session_mtx);

	server = server_mtx.lock().await;
	server.remove_session(&session_handle).await;
	drop(server);
	drop(session_mtx);

	// for debugging purposes
	debug_assert!(
		weak_session.strong_count() == 0,
		"session count ref is not 0"
	);

	Ok(())
}

async fn task_listener(listener: TcpListener, config: config::Config) -> anyhow::Result<()> {
	let server = Server::new(config);

	while let Ok((tcp_conn, _)) = listener.accept().await {
		let s = server.clone();
		tokio::task::Builder::new()
			.name("Listener task")
			.spawn(async move {
				if let Err(e) = task_processor(tcp_conn, s).await {
					log::error!("task_processor: {}", e);
				}
			})
			.unwrap();
	}

	Ok(())
}

async fn run() -> anyhow::Result<()> {
	let config = config::load().await?;

	let listen_addr = format!("{}:{}", config.listen_ip, config.listen_port);
	log::info!("Listening to {}", listen_addr);

	let listener = TcpListener::bind(listen_addr).await?;

	if let Err(e) = task_listener(listener, config).await {
		log::error!("Listener error: {}", e);
	}

	Ok(())
}

fn main() {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_time()
		.worker_threads(8)
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
