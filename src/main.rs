use std::{collections::VecDeque, sync::Arc};

use futures_util::{
	stream::{SplitSink, SplitStream},
	StreamExt,
};

use server::ServerMutex;
use session::{SessionHandle, SessionInstanceMutex};
use tokio::{
	net::{TcpListener, TcpStream},
	sync::Mutex,
};

use tokio_util::sync::CancellationToken;
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

use crate::{server::Server, session::SessionInstance};

extern crate pretty_env_logger;

mod canvas_cache;
mod chunk;
mod command;
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
mod serial_generator;
mod server;
mod session;
mod signal;
mod time;
mod tool;
mod util;

pub type Connection = WebSocketStream<TcpStream>;
pub type ConnectionWriter = SplitSink<WebSocketStream<TcpStream>, Message>;
pub type ConnectionReader = SplitStream<WebSocketStream<TcpStream>>;
pub type ConnectionMutex = Arc<Mutex<Connection>>;

async fn connection_read_packet(
	reader: &mut SplitStream<WebSocketStream<TcpStream>>,
	session_handle: &SessionHandle,
	server_mtx: &ServerMutex,
	session_mtx: &SessionInstanceMutex,
) -> anyhow::Result<()> {
	match reader.next().await {
		Some(res) => match res {
			Ok(msg) => {
				if msg.is_binary() {
					let session_weak = Arc::downgrade(session_mtx);
					let mut session = session_mtx.lock().await;
					session
						.process_payload(session_weak, session_handle, msg.as_payload(), server_mtx)
						.await?;
				} else {
					log::trace!("Got unknown message");
				}
			}
			Err(e) => {
				log::info!("Session {} error: {}", session_handle.id(), e);
				return Err(anyhow::anyhow!("Connection interrupted"));
			}
		},
		None => {
			// Just exit
			return Err(anyhow::anyhow!("Connection interrupted"));
		}
	}

	Ok(())
}

async fn task_connection(tcp_conn: TcpStream, server_mtx: ServerMutex) -> anyhow::Result<()> {
	let connection = ServerBuilder::new().accept(tcp_conn).await?;
	let (writer, mut reader) = connection.1.split();

	let mut server = server_mtx.lock().await;
	let cancel_token = CancellationToken::new();

	log::trace!("Creating new session");
	let (session_handle, session_mtx) = server.create_session(cancel_token.clone());
	drop(server);
	log::info!("Created session with ID {}", session_handle.id());

	{
		let mut session_instance = session_mtx.lock().await;

		// Spawn sender task
		SessionInstance::launch_task_sender(
			session_handle.id(),
			&mut session_instance,
			Arc::downgrade(&session_mtx),
			writer,
		);

		// Spawn tick task
		SessionInstance::launch_task_tick(
			session_handle,
			&mut session_instance,
			Arc::downgrade(&session_mtx),
		);
	}

	loop {
		tokio::select! {
			_ = cancel_token.cancelled() => {
				log::info!("Got cancel token, freeing session");
				break;
			}
			res = connection_read_packet(&mut reader, &session_handle,&server_mtx,&session_mtx) => {
				// exit loop on error
				if let Err(e) = res {
					log::error!("connection_read_packet error: {e}, closing connection");
					cancel_token.cancel();
					break;
				}
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

	//for debugging purposes
	debug_assert!(
		weak_session.strong_count() == 0,
		"session count ref is not 0"
	);

	log::info!("Task processor ended");

	Ok(())
}

async fn server_listener_loop(listener: TcpListener, server: ServerMutex) -> anyhow::Result<()> {
	while let Ok((tcp_conn, _)) = listener.accept().await {
		let s = server.clone();
		tokio::task::Builder::new()
			.name("Listener task")
			.spawn(async move {
				if let Err(e) = task_connection(tcp_conn, s).await {
					log::error!("task_processor: {e}");
				}
			})
			.unwrap();
	}

	Ok(())
}

async fn task_listener(listener: TcpListener, config: config::Config) -> anyhow::Result<()> {
	let cancel_token = CancellationToken::new();

	let enable_console = config.enable_console.unwrap_or(false);

	let server = Server::new(config, cancel_token.clone());

	let cancel_token_command = CancellationToken::new();
	if enable_console {
		command::start(server.clone(), cancel_token_command.clone());
	}

	tokio::select! {
		_ = cancel_token.cancelled() => {
			log::info!("Got cancel token, stopping server");
		}
		res = server_listener_loop(listener, server.clone()) => {
			// exit loop on error
			if let Err(e) = res {
				log::error!("Listener error: {e}");
			}
		}
	}

	cancel_token_command.cancel();

	Ok(())
}

async fn run() -> anyhow::Result<()> {
	let config = config::load().await?;

	let listen_addr = format!("{}:{}", config.listen_ip, config.listen_port);
	log::info!("Listening to {listen_addr}");

	let listener = TcpListener::bind(listen_addr).await?;

	if let Err(e) = task_listener(listener, config).await {
		log::error!("Listener error: {e}");
	}

	Ok(())
}

fn main() {
	{
		let runtime = tokio::runtime::Builder::new_multi_thread()
			.enable_time()
			.thread_name("mp")
			.worker_threads(std::thread::available_parallelism().unwrap().get().min(2)) // Max 4 threads
			.thread_stack_size(2 * 1024 * 1024)
			.enable_io()
			.build()
			.unwrap();

		let mut args: VecDeque<String> = std::env::args().collect();
		args.pop_front(); // Ignore program path

		while let Some(arg) = args.pop_front() {
			match arg.as_str() {
				"--console-subscriber" => {
					console_subscriber::init();
				}
				_ => {
					log::info!("Unknown argument: {arg}");
				}
			}
		}

		std::env::set_var("RUST_LOG", "trace");
		pretty_env_logger::init_timed();

		if let Err(e) = runtime.block_on(run()) {
			log::error!("{e}");
		}
	}

	log::info!("Server exited gracefully.");
}
