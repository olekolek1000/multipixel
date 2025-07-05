use std::collections::VecDeque;

use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

use crate::server::ServerMutex;

#[cfg(feature = "dump")]
use {std::time::Duration, tokio::runtime::Handle, tokio::time::timeout};

#[cfg(feature = "dump")]
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

async fn process_command(line: String, server: &ServerMutex) -> anyhow::Result<()> {
	let mut parts: VecDeque<&str> = line.split(" ").collect();
	if let Some(raw_keyword) = parts.pop_front() {
		let keyword = raw_keyword.trim();
		match keyword {
			"help" | "?" => {
				print_help();
			}
			"dump" => {
				#[cfg(feature = "dump")]
				dump_tasks().await;
				#[cfg(not(feature = "dump"))]
				log::error!("Feature \"dump\" not enabled");
			}
			"exit" => {
				if let Err(e) = server.lock().await.save_and_exit().await {
					log::error!("Cannot exit gracefully: {e}.");
				}
			}
			_ => {
				if !keyword.is_empty() {
					log::error!("Unknown command \"{keyword}\".");
				}
			}
		}
	}
	Ok(())
}

async fn runner(server: ServerMutex) -> anyhow::Result<()> {
	let mut stdin = tokio_fd::AsyncFd::try_from(libc::STDIN_FILENO)?;
	let mut buf: Vec<u8> = vec![0; 32];

	loop {
		let mut cmd: Vec<u8> = Vec::new();
		while let Ok(byte_count) = stdin.read(&mut buf).await {
			for (idx, byte) in buf.iter().enumerate() {
				if *byte == b'\r' {
					//Inferior CRLF encoding, skip
					continue;
				}

				if *byte == b'\n' {
					// Parse line
					let line = String::from(String::from_utf8_lossy(&cmd));
					cmd.clear();
					process_command(line, &server).await?;
				} else {
					cmd.push(*byte);
				}

				if idx >= byte_count - 1 {
					break; // end of buf
				}
			}
		}
	}
}

pub fn start(server: ServerMutex, cancel_token: CancellationToken) {
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
						log::error!("Command runner error: {e}");
					}
				}
			}
		}
	});
}
