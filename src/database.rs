use glam::{IVec2, UVec2};
use num_enum::TryFromPrimitive;
use rusqlite::params;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chunk::layer::LayerRGBA;
use crate::compression::decompress_lz4;
use crate::pixel::ColorRGBA;

const SECONDS_BETWEEN_SNAPSHOTS: u32 = 14400;

fn get_unix_timestamp() -> u64 {
	match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
		Ok(n) => n.as_secs(),
		Err(_) => 0,
	}
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum CompressionType {
	Lz4 = 1,
}

pub struct Database {
	pub conn: rusqlite::Connection,
	cleaned_up: bool,
	pub migrated_from_version: u32,
}

#[allow(dead_code)]
pub struct ChunkDatabaseRecord {
	pub compression_type: CompressionType,
	pub created_at: u64,  //unix timestamp
	pub modified_at: u64, // unix timestamp
	pub data: Vec<u8>,
}

pub struct PreviewDatabaseRecord {
	pub data: Vec<u8>,
}

const DATABASE_VERSION: u32 = 1;

fn get_version(conn: &rusqlite::Connection) -> rusqlite::Result<u32> {
	let mut stmt = conn.prepare("PRAGMA user_version")?;
	let mut res = stmt.query([])?;

	let Some(row) = res.next()? else {
		return Ok(0);
	};

	let val: u32 = row.get(0)?;

	Ok(val)
}

fn set_version(conn: &rusqlite::Connection, version: u32) -> rusqlite::Result<()> {
	conn.execute(&format!("PRAGMA user_version = {version}"), [])?;
	Ok(())
}

fn migrate_to_version_1(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
	log::info!("Migrating to version 1");

	log::info!("removing previews");
	conn.execute("DELETE FROM previews", [])?;

	log::info!("loading all compressed rgb chunks into memory");
	struct ChunkDataRow {
		x: i32,
		y: i32,
		data: Vec<u8>,
	}

	let old_data: Vec<_> = {
		let mut stmt = conn.prepare("SELECT x, y, data FROM chunk_data")?;
		let res: Vec<_> = stmt
			.query_map([], |row| {
				Ok(ChunkDataRow {
					x: row.get(0)?,
					y: row.get(1)?,
					data: row.get(2)?,
				})
			})?
			.flatten()
			.collect();
		res
	};

	log::info!("loaded {} chunks", old_data.len());
	log::info!("converting chunks from rgb to rgba");

	let old_data_len = old_data.len();

	let mut i = 0;
	for old_cell in old_data {
		i += 1;
		log::info!(
			"updating chunk at {}x{} {}%",
			old_cell.x,
			old_cell.y,
			((i as f32 / old_data_len as f32) * 100.0).round()
		);

		let Some(rgb) = decompress_lz4(&old_cell.data, 256 * 256 * 3) else {
			log::error!("failed to decompress");
			continue;
		};

		let mut layer = LayerRGBA::new();
		layer.alloc_transparent_black();

		for y in 0..256 {
			for x in 0..256 {
				let offset = y * 256 * 3 + x * 3;

				let red = rgb[offset];
				let green = rgb[offset + 1];
				let blue = rgb[offset + 2];

				layer.set_pixel(
					UVec2::new(x as u32, y as u32),
					ColorRGBA {
						r: red,
						g: green,
						b: blue,
						a: 255,
					},
				);
			}
		}

		let compressed = layer.compress_lz4();

		conn.execute(
			"UPDATE chunk_data SET data=? WHERE x=? AND y=?",
			params![compressed, old_cell.x, old_cell.y],
		)?;
	}

	Ok(())
}

impl Database {
	pub async fn new(path: &str) -> rusqlite::Result<Self> {
		log::trace!("Opening database at path {path}");
		let conn = rusqlite::Connection::open(path)?;

		let mut db = Self {
			conn,
			cleaned_up: false,
			migrated_from_version: 0,
		};

		Self::run_empty_query(&db.conn, "PRAGMA SYNCHRONOUS=OFF")?;

		let db_version = get_version(&db.conn)?;
		db.migrated_from_version = db_version;

		if db_version != DATABASE_VERSION {
			log::info!("Updating database from version {db_version} to version {DATABASE_VERSION}");
		}

		if db_version == 0 {
			migrate_to_version_1(&db.conn)?;
		}

		set_version(&db.conn, DATABASE_VERSION)?;

		Self::init_table_chunk_data(&db.conn)?;
		Self::init_table_previews(&db.conn)?;

		log::info!("Database at path {path} loaded");

		Ok(db)
	}

	fn run_empty_query(conn: &rusqlite::Connection, query: &'static str) -> rusqlite::Result<()> {
		let mut stmt = conn.prepare(query)?;
		stmt.execute([])?;
		Ok(())
	}

	fn init_table_chunk_data(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
		Self::run_empty_query(conn, "CREATE TABLE IF NOT EXISTS chunk_data(x INT NOT NULL, y INT NOT NULL, data BLOB, modified INT64 NOT NULL, created INT64 NOT NULL, compression INT);")?;
		Self::run_empty_query(conn, "CREATE INDEX IF NOT EXISTS index_x on chunk_data(x)")?;
		Self::run_empty_query(conn, "CREATE INDEX IF NOT EXISTS index_y on chunk_data(y)")?;
		Ok(())
	}

	fn init_table_previews(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
		Self::run_empty_query(conn, "CREATE TABLE IF NOT EXISTS previews(x INT NOT NULL, y INT NOT NULL, zoom INT NOT NULL, data BLOB)")?;
		Self::run_empty_query(
			conn,
			"CREATE INDEX IF NOT EXISTS previews_index_x on previews(x)",
		)?;
		Self::run_empty_query(
			conn,
			"CREATE INDEX IF NOT EXISTS previews_index_y on previews(y)",
		)?;
		Ok(())
	}

	fn chunk_insert(
		conn: &rusqlite::Connection,
		pos: IVec2,
		data: &[u8],
		compression_type: CompressionType,
	) -> rusqlite::Result<()> {
		conn.execute(
			"INSERT INTO chunk_data (x,y,data,modified,created,compression) VALUES(?,?,?,?,?,?)",
			params![
				pos.x,
				pos.y,
				data,
				get_unix_timestamp(),
				get_unix_timestamp(),
				compression_type as i32,
			],
		)?;
		Ok(())
	}

	fn chunk_save_data(
		conn: &rusqlite::Connection,
		pos: IVec2,
		data: &[u8],
		compression_type: CompressionType,
	) -> rusqlite::Result<()> {
		struct Row {
			timestamp: i64,
			chunk_id: i64,
		}

		if let Ok(row) = conn.query_row(
			"SELECT created, rowid FROM chunk_data WHERE x = ? AND y = ? ORDER BY created DESC",
			params![pos.x, pos.y],
			|row| {
				Ok(Row {
					timestamp: row.get(0)?,
					chunk_id: row.get(1)?,
				})
			},
		) {
			//Chunk already exists, update chunk
			log::trace!("Updating chunk");
			if get_unix_timestamp() as i64 - row.timestamp > SECONDS_BETWEEN_SNAPSHOTS as i64 {
				//Insert a new chunk in its place
				Self::chunk_insert(conn, pos, data, compression_type)?;
			} else {
				//Replace chunk
				conn.execute(
					"UPDATE chunk_data SET modified = ?, data = ?, compression = ? WHERE rowid = ?",
					params![
						get_unix_timestamp(),
						data,
						compression_type as i32,
						row.chunk_id
					],
				)?;
			}
		} else {
			// Chunk doesn't exist, create chunk
			log::trace!("Creating chunk");
			Self::chunk_insert(conn, pos, data, compression_type)?;
		}

		Ok(())
	}

	fn chunk_list_all(conn: &rusqlite::Connection) -> rusqlite::Result<HashSet<IVec2>> {
		struct Row {
			x: i32,
			y: i32,
		}

		let mut stmt = conn.prepare("SELECT x, y FROM chunk_data")?;
		let iter = stmt.query_map([], |row| {
			Ok(Row {
				x: row.get(0)?,
				y: row.get(1)?,
			})
		})?;

		let mut res: HashSet<IVec2> = HashSet::new();

		for row in iter.flatten() {
			res.insert(IVec2::new(row.x, row.y));
		}

		Ok(res)
	}

	fn chunk_load_data(
		conn: &rusqlite::Connection,
		pos: IVec2,
	) -> rusqlite::Result<Option<ChunkDatabaseRecord>> {
		struct Row {
			data: Vec<u8>,
			compression: u8,
			modified: i64,
			created: i64,
		}

		if let Ok(row) = conn
			.query_row(
				"SELECT data, compression, modified, created FROM chunk_data WHERE x=? AND y=? ORDER BY modified DESC",
				params![pos.x, pos.y],
				|row| {
					Ok(Row {
						data: row.get(0)?,
						compression: row.get(1)?,
						modified: row.get(2)?,
						created: row.get(3)?,
					})
				},
			) {
				return Ok(Some(ChunkDatabaseRecord{
					compression_type: CompressionType::try_from(row.compression).unwrap_or(CompressionType::Lz4),
					created_at: row.created as u64,
					modified_at: row.modified as u64,
					data: row.data,
				}));
			}

		Ok(None)
	}

	fn preview_load_data(
		conn: &rusqlite::Connection,
		pos: &IVec2,
		zoom: u8,
	) -> rusqlite::Result<Option<PreviewDatabaseRecord>> {
		struct Row {
			data: Vec<u8>,
		}

		if let Ok(row) = conn.query_row(
			"SELECT data FROM previews WHERE x=? AND y=? AND zoom=?",
			params![pos.x, pos.y, zoom],
			|row| Ok(Row { data: row.get(0)? }),
		) {
			return Ok(Some(PreviewDatabaseRecord { data: row.data }));
		}

		Ok(None)
	}

	fn preview_save_data(
		conn: &rusqlite::Connection,
		pos: IVec2,
		zoom: u8,
		data: &[u8],
	) -> rusqlite::Result<()> {
		if let Ok(rowid) = conn.query_row(
			"SELECT rowid FROM previews WHERE x=? AND y=? AND zoom=?",
			params![pos.x, pos.y, zoom],
			|row| {
				let res: u32 = row.get(0)?;
				Ok(res)
			},
		) {
			// Update preview data
			conn.execute(
				"UPDATE previews SET x=?, y=?, zoom=?, data=? WHERE rowid=?",
				params![pos.x, pos.y, zoom, data, rowid],
			)?;
		} else {
			// Insert new preview data
			conn.execute(
				"INSERT INTO previews (x,y,zoom,data) VALUES (?,?,?,?)",
				params![pos.x, pos.y, zoom, data],
			)?;
		}

		Ok(())
	}

	pub async fn cleanup(&mut self) {
		log::trace!("Cleaning-up database");
		self.cleaned_up = true;
	}

	pub async fn get_conn<F, ResultType>(
		database: &Arc<Mutex<Database>>,
		callback: F,
	) -> anyhow::Result<ResultType>
	where
		for<'a> F: FnOnce(&'a rusqlite::Connection) -> rusqlite::Result<ResultType> + Send + 'static,
		ResultType: std::marker::Send + 'static,
	{
		let db = database.lock().await;
		let res = callback(&db.conn)?;
		Ok(res)
	}
}

impl Drop for Database {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
		log::trace!("Database freed");
	}
}

pub struct DatabaseFunc {}

impl DatabaseFunc {
	pub async fn chunk_list_all(database: &Arc<Mutex<Database>>) -> anyhow::Result<HashSet<IVec2>> {
		Database::get_conn(database, Database::chunk_list_all).await
	}

	pub async fn chunk_load_data(
		database: &Arc<Mutex<Database>>,
		chunk_pos: IVec2,
	) -> anyhow::Result<Option<ChunkDatabaseRecord>> {
		Database::get_conn(database, move |conn| {
			Database::chunk_load_data(conn, chunk_pos)
		})
		.await
	}

	pub async fn chunk_save_data(
		database: &Arc<Mutex<Database>>,
		pos: IVec2,
		data: Arc<Vec<u8>>,
		compression_type: CompressionType,
	) -> anyhow::Result<()> {
		Database::get_conn(database, move |conn| {
			Database::chunk_save_data(conn, pos, &data, compression_type)
		})
		.await
	}

	pub async fn preview_load_data(
		database: &Arc<Mutex<Database>>,
		pos: IVec2,
		zoom: u8,
	) -> anyhow::Result<Option<PreviewDatabaseRecord>> {
		Database::get_conn(database, move |conn| {
			Database::preview_load_data(conn, &pos, zoom)
		})
		.await
	}

	pub async fn preview_save_data(
		database: &Arc<Mutex<Database>>,
		pos: IVec2,
		zoom: u8,
		data: Vec<u8>,
	) -> anyhow::Result<()> {
		Database::get_conn(database, move |conn| {
			Database::preview_save_data(conn, pos, zoom, &data)
		})
		.await
	}
}
