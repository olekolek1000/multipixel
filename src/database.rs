use async_sqlite::{
	rusqlite::{self, OptionalExtension},
	ClientBuilder, JournalMode,
};
use glam::IVec2;
use num_enum::TryFromPrimitive;

const SECONDS_BETWEEN_SNAPSHOTS: u32 = 14400;

fn get_unix_timestamp() -> u64 {
	match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
		Ok(n) => n.as_secs(),
		Err(_) => return 0,
	}
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum CompressionType {
	Lz4 = 1,
}

pub struct Database {
	client: async_sqlite::Client,

	cleaned_up: bool,
}

pub struct ChunkDatabaseRecord {
	pub compression_type: CompressionType,
	pub created_at: u64,  //unix timestamp
	pub modified_at: u64, // unix timestamp
	pub data: Vec<u8>,
}

impl Database {
	pub async fn new(path: &str) -> anyhow::Result<Self> {
		let client = ClientBuilder::new()
			.path(path)
			.journal_mode(JournalMode::Wal)
			.open()
			.await?;

		let db = Self {
			client,
			cleaned_up: false,
		};

		db.client
			.conn(move |conn| {
				Self::run_empty_query(conn, "PRAGMA SYNCHRONOUS=OFF")?;
				Self::init_table_chunk_data(conn)?;
				Self::init_table_previews(conn)?;
				Ok(())
			})
			.await?;

		log::info!("Database {} loaded", path);

		Ok(db)
	}

	fn run_empty_query(
		conn: &rusqlite::Connection,
		query: &'static str,
	) -> Result<(), rusqlite::Error> {
		let mut stmt = conn.prepare(query)?;
		stmt.execute([])?;
		Ok(())
	}

	fn init_table_chunk_data(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
		Self::run_empty_query(conn, "CREATE TABLE IF NOT EXISTS chunk_data(x INT NOT NULL, y INT NOT NULL, data BLOB, modified INT64 NOT NULL, created INT64 NOT NULL, compression INT);")?;
		Self::run_empty_query(conn, "CREATE INDEX IF NOT EXISTS index_x on chunk_data(x)")?;
		Self::run_empty_query(conn, "CREATE INDEX IF NOT EXISTS index_y on chunk_data(y)")?;
		Ok(())
	}

	fn init_table_previews(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
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

	pub fn chunk_insert(
		conn: rusqlite::Connection,
		pos: IVec2,
		data: &[u8],
		compression_type: CompressionType,
	) -> Result<(), rusqlite::Error> {
		conn.execute(
			"INSERT INTO chunk_data (x,y,data,modified,created,compression) VALUES(?,?,?,?,?,?)",
			rusqlite::params![
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

	pub fn chunk_save_data(
		conn: rusqlite::Connection,
		pos: IVec2,
		data: &[u8],
		compression_type: CompressionType,
	) -> Result<(), rusqlite::Error> {
		struct Row {
			timestamp: i64,
			chunk_id: i64,
		}

		if let Some(row) = conn
			.query_row(
				"SELECT created, rowid FROM chunk_data WHERE x = ? AND y = ? ORDER BY created DESC",
				rusqlite::params![pos.x, pos.y],
				|row| {
					Ok(Row {
						timestamp: row.get(0)?,
						chunk_id: row.get(1)?,
					})
				},
			)
			.optional()?
		{
			//Chunk already exists, update chunk

			if get_unix_timestamp() as i64 - row.timestamp > SECONDS_BETWEEN_SNAPSHOTS as i64 {
				//Insert a new chunk in its place
				Self::chunk_insert(conn, pos, data, compression_type)?;
			} else {
				//Replace chunk
				conn.execute(
					"UPDATE chunk_data SET modified = ?, data = ?, compression = ? WHERE rowid = ?",
					rusqlite::params![0i64, data, compression_type as i32, row.chunk_id],
				)?;
			}
		} else {
			// Chunk doesn't exist, create chunk
			Self::chunk_insert(conn, pos, data, compression_type)?;
		}

		Ok(())
	}

	pub fn chunk_load_data(
		conn: rusqlite::Connection,
		pos: IVec2,
	) -> anyhow::Result<Option<ChunkDatabaseRecord>> {
		struct Row {
			data: Vec<u8>,
			compression: u8,
			modified: i64,
			created: i64,
		}

		if let Some(row) = conn
			.query_row(
				"SELECT data, compression, modified, created FROM chunk_data WHERE x=? AND y=? ORDER BY modified DESC",
				rusqlite::params![pos.x, pos.y],
				|row| {
					Ok(Row {
						data: row.get(0)?,
						compression: row.get(1)?,
						modified: row.get(2)?,
						created: row.get(3)?,
					})
				},
			)
			.optional()? {
				return Ok(Some(ChunkDatabaseRecord{
					compression_type: CompressionType::try_from(row.compression)?,
					created_at: row.created as u64,
					modified_at: row.modified as u64,
					data: row.data,
				}));
			}

		Ok(None)
	}

	pub async fn cleanup(&mut self) {
		self.cleaned_up = true;
	}
}

impl Drop for Database {
	fn drop(&mut self) {
		assert!(self.cleaned_up, "cleanup() not called");
		log::trace!("Database freed");
	}
}
