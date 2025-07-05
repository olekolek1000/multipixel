use glam::IVec2;
use num_enum::TryFromPrimitive;
use rusqlite::params;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

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

impl Database {
	pub async fn new(path: &str) -> rusqlite::Result<Self> {
		log::trace!("Opening database at path {path}");
		let conn = rusqlite::Connection::open(path)?;

		let db = Self {
			conn,
			cleaned_up: false,
		};

		Self::run_empty_query(&db.conn, "PRAGMA SYNCHRONOUS=OFF")?;
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

		let mut stmt = conn.prepare("SELECT x, y FROM chunk_data").unwrap();
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
