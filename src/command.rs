use std::{collections::VecDeque, os::fd::AsRawFd, time::Duration};

use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
use tokio::{io::AsyncBufReadExt, runtime::Handle, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::server::ServerMutex;

async fn dump_tasks() {
	let handle = Handle::current();
	if let Ok(dump) = timeout(Duration::from_millis(2000), handle.dump()).await {
		for (i, task) in dump.tasks().iter().enumerate() {
			let trace = task.trace();
			log::info!("TASK {i}: \n{trace}\n");
		}
	}
	log::info!("Task dump finished. Server restart is heavily advised due to a Tokio bug.");
}

fn print_help() {
	log::info!("help - Print this help");
	log::info!("dump - Show stacktrace of all async tasks (dangerous!)");
	log::info!("exit - Save everything and exit");
}

fn make_stdin_async() {
	// prevent hanging infinitely until pressing enter
	unsafe {
		let fd = std::io::stdin().as_raw_fd();
		let flags = fcntl(fd, F_GETFL, 0);
		fcntl(fd, F_SETFL, flags | O_NONBLOCK);
	}
}

async fn runner(server: ServerMutex) -> anyhow::Result<()> {
	loop {
		let stdin = tokio::io::stdin();
		let mut reader = tokio::io::BufReader::new(stdin);
		let mut line = String::new();

		// FIXME: this is a dirty hack, make_stdin_async causes stdin to be non-blocking and we are catching those errors to exit the server gracefully
		// tokio stdin is NOT async and cannot be cancelled (!!)
		if (reader.read_line(&mut line).await).is_err() {
			tokio::time::sleep(Duration::from_millis(250)).await;
			return Ok(()); // stdin is async, it will throw "Resource temporarily unavailable" errors
		}
		let mut parts: VecDeque<&str> = line.split(" ").collect();
		if let Some(raw_keyword) = parts.pop_front() {
			let keyword = raw_keyword.trim();
			match keyword {
				"help" | "?" => {
					print_help();
				}
				"dump" => {
					dump_tasks().await;
				}
				"exit" => {
					if let Err(e) = server.lock().await.save_and_exit().await {
						log::error!("Cannot exit gracefully: {}.", e);
					}
				}
				_ => {
					if !keyword.is_empty() {
						log::error!("Unknown command \"{}\".", keyword);
					}
				}
			}
		}
	}
}

pub fn start(server: ServerMutex, cancel_token: CancellationToken) {
	make_stdin_async();

	tokio::spawn(async move {
		loop {
			tokio::select! {
				_ = cancel_token.cancelled() => {
					log::info!("Exiting command runner");
					break;
				}
				res = runner(server.clone()) => {
					// exit loop on error
					if let Err(e) = res {
						log::error!("Command runner error: {}", e);
					}
				}
			}
		}
	});
}
