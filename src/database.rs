use async_sqlite::{ClientBuilder, JournalMode};
use glam::IVec2;
use num_enum::TryFromPrimitive;

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
enum CompressionType {
	Lz4 = 1,
}

pub struct Database {
	client: async_sqlite::Client,

	cleaned_up: bool,
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

		db.run_empty_query("PRAGMA SYNCHRONOUS=OFF").await?;
		db.init_table_chunk_data().await?;
		db.init_table_previews().await?;

		log::info!("Database {} loaded", path);

		Ok(db)
	}

	async fn run_empty_query(&self, query: &str) -> anyhow::Result<()> {
		let sql = String::from(query);
		self
			.client
			.conn(move |conn| {
				let mut stmt = conn.prepare(sql.as_str())?;
				stmt.execute([])?;
				Ok(())
			})
			.await?;

		Ok(())
	}

	async fn init_table_chunk_data(&self) -> anyhow::Result<()> {
		self
			.run_empty_query("CREATE TABLE IF NOT EXISTS chunk_data(x INT NOT NULL, y INT NOT NULL, data BLOB, modified INT64 NOT NULL, created INT64 NOT NULL, compression INT);")
			.await?;
		self
			.run_empty_query("CREATE INDEX IF NOT EXISTS index_x on chunk_data(x)")
			.await?;
		self
			.run_empty_query("CREATE INDEX IF NOT EXISTS index_y on chunk_data(y)")
			.await?;
		Ok(())
	}

	async fn init_table_previews(&self) -> anyhow::Result<()> {
		self
			.run_empty_query("CREATE TABLE IF NOT EXISTS previews(x INT NOT NULL, y INT NOT NULL, zoom INT NOT NULL, data BLOB)")
			.await?;
		self
			.run_empty_query("CREATE INDEX IF NOT EXISTS previews_index_x on previews(x)")
			.await?;
		self
			.run_empty_query("CREATE INDEX IF NOT EXISTS previews_index_y on previews(y)")
			.await?;
		Ok(())
	}

	pub async fn chunk_save_data(
		pos: IVec2,
		data: &[u8],
		compression_type: CompressionType,
	) -> anyhow::Result<()> {
		todo!()
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
