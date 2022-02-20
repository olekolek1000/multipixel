#include "database.hpp"
#include "Statement.h"
#include "lib/SQLiteCpp/SQLiteCpp.h"
#include "src/server.hpp"
#include "util/smartptr.hpp"
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include <stdexcept>
#include <stdio.h>
#include <string>

DatabaseConnector::DatabaseConnector() {
}

void DatabaseConnector::init(const char *dbpath) {
	db.create(dbpath, SQLite::OPEN_CREATE | SQLite::OPEN_READWRITE);

	initTableChunkData();
	initTablePreviews();
}

void DatabaseConnector::initTableChunkData() {
	{
		SQLite::Statement query(*db, "CREATE TABLE IF NOT EXISTS chunk_data(x INT NOT NULL, y INT NOT NULL, data BLOB, modified INT64 NOT NULL, created INT64 NOT NULL, compression INT);");
		query.exec();
	}

	// X index
	{
		SQLite::Statement query(*db, "CREATE INDEX IF NOT EXISTS index_x on chunk_data(x)");
		query.exec();
	}

	// Y index
	{
		SQLite::Statement query(*db, "CREATE INDEX IF NOT EXISTS index_y on chunk_data(y)");
		query.exec();
	}
}

void DatabaseConnector::initTablePreviews() {
	{
		SQLite::Statement query(*db, "CREATE TABLE IF NOT EXISTS previews(x INT NOT NULL, y INT NOT NULL, zoom INT NOT NULL, data BLOB)");
		query.exec();
	}

	// X index
	{
		SQLite::Statement query(*db, "CREATE INDEX IF NOT EXISTS previews_index_x on previews(x)");
		query.exec();
	}

	// Y index
	{
		SQLite::Statement query(*db, "CREATE INDEX IF NOT EXISTS previews_index_y on previews(y)");
		query.exec();
	}
}

DatabaseConnector::~DatabaseConnector() {
}

auto DatabaseConnector::chunkSaveData(Int2 pos, const void *data, size_t size, CompressionType type) -> void {
	SQLite::Statement query_select(*db, "SELECT created, rowid FROM chunk_data WHERE x = ? AND y = ? ORDER BY created DESC");
	query_select.bind(1, pos.x);
	query_select.bind(2, pos.y);

	if(query_select.executeStep()) {
		// Chunk already exists, update chunk
		s64 timestamp = query_select.getColumn(0);

		if(time(nullptr) - timestamp > seconds_between_snapshot) {
			insert(pos, data, size, type);
		} else {
			s64 chunk_id = query_select.getColumn(1);

			SQLite::Statement query_update(*db, "UPDATE chunk_data SET modified = ?, data = ?, compression = ? WHERE rowid = ?");
			query_update.bind(1, time(nullptr));
			query_update.bind(2, data, size);
			query_update.bind(3, (int)type);
			query_update.bind(4, chunk_id);
			query_update.exec();
		}
	} else {
		// Chunk does not exist, create chunk
		insert(pos, data, size, type);
	}
}

auto DatabaseConnector::chunkLoadData(Int2 pos) -> ChunkDatabaseRecord {
	ChunkDatabaseRecord rec;

	SQLite::Statement query(*db, "SELECT data, compression, modified, created FROM chunk_data WHERE x=? AND y=? ORDER BY modified DESC");
	query.bind(1, pos.x);
	query.bind(2, pos.y);

	if(query.executeStep()) {
		const auto &col = query.getColumn(0);
		auto *blob = col.getBlob();
		auto blob_size = col.size();

		rec.compression_type = (CompressionType)query.getColumn(1).getInt();
		rec.modified = query.getColumn(2).getInt64();
		rec.created = query.getColumn(3).getInt64();

		rec.data = createSharedVector<u8>(blob_size);
		memcpy(rec.data->data(), blob, blob_size);
	}

	return rec;
}

void DatabaseConnector::foreachChunk(std::function<void(Int2)> callback) {
	SQLite::Statement query(*db, "SELECT x, y FROM chunk_data");
	while(query.executeStep()) {
		callback({(s32)query.getColumn(0), (s32)query.getColumn(1)});
	}
}

void DatabaseConnector::previewSaveData(Int2 pos, u8 zoom, const void *data, size_t size) {
	SQLite::Statement query_select(*db, "SELECT rowid FROM previews WHERE x=? AND y=? AND zoom=?");
	query_select.bind(1, pos.x);
	query_select.bind(2, pos.y);
	query_select.bind(3, zoom);

	if(query_select.executeStep()) {
		int chunk_id = query_select.getColumn(0);
		SQLite::Statement query(*db, "UPDATE previews SET x=?, y=?, zoom=?, data=? WHERE rowid=?");
		query.bind(1, pos.x);
		query.bind(2, pos.y);
		query.bind(3, zoom);
		query.bind(4, data, size);
		query.bind(5, chunk_id);
		query.exec();
	} else {
		SQLite::Statement query(*db, "INSERT INTO previews (x,y,zoom,data) VALUES (?,?,?,?)");
		query.bind(1, pos.x);
		query.bind(2, pos.y);
		query.bind(3, zoom);
		query.bind(4, data, size);
		query.exec();
	}
}

PreviewDatabaseRecord DatabaseConnector::previewLoadData(Int2 pos, u8 zoom) {
	PreviewDatabaseRecord rec;

	SQLite::Statement query(*db, "SELECT data FROM previews WHERE x=? AND y=? AND zoom=?");
	query.bind(1, pos.x);
	query.bind(2, pos.y);
	query.bind(3, zoom);

	if(query.executeStep()) {
		const auto &col = query.getColumn(0);
		auto *blob = col.getBlob();
		auto blob_size = col.size();

		rec.data = createSharedVector<u8>(blob_size);
		memcpy(rec.data->data(), blob, blob_size);
	}

	return rec;
}

auto DatabaseConnector::listSnapshots(Int2 pos) -> uniqdata<DatabaseListElement> {
	SQLite::Statement query(*db, "SELECT rowid, modified FROM chunk_data WHERE x = ? AND y = ? ORDER BY modified DESC");
	query.bind(1, pos.x);
	query.bind(2, pos.y);

	uniqdata<DatabaseListElement> timestamps;
	while(query.executeStep()) {
		timestamps.push_back({query.getColumn(0).getInt64(), query.getColumn(0).getInt64()});
	}

	return timestamps;
}

auto DatabaseConnector::insert(Int2 pos, const void *data, size_t size, CompressionType type) -> void {
	SQLite::Statement query(*db, "INSERT INTO chunk_data (x,y,data,modified,created,compression) VALUES(?,?,?,?,?,?)");
	query.bind(1, pos.x);
	query.bind(2, pos.y);
	query.bind(3, data, size);
	query.bind(4, time(nullptr));
	query.bind(5, time(nullptr));
	query.bind(6, (int)type);
	query.exec();
}

auto DatabaseConnector::getSnapshotInerval() -> s64 {
	return seconds_between_snapshot;
}

auto DatabaseConnector::setSnapshotInerval(s64 seconds) -> void {
	seconds_between_snapshot = seconds;
}

Transaction::~Transaction() {
	if(!connector) return;
	connector->transactionCommit();
	connector = nullptr;
}

void Transaction::commit() {
	if(!connector) return;
	connector->transactionCommit();
	connector = nullptr;
	lock.free();
}

void Transaction::rollback() {
	if(!connector) return;
	connector->transactionRollback();
	connector = nullptr;
}

std::shared_ptr<Transaction> DatabaseConnector::transactionBegin() {
	auto transaction = std::make_shared<Transaction>();
	transaction->lock.setMutex(mtx_access);
	transaction->connector = this;

	{
		SQLite::Statement query(*db, "BEGIN");
		query.exec();
	}

	return transaction;
}

void DatabaseConnector::transactionCommit() {
	SQLite::Statement query(*db, "COMMIT");
	query.exec();
}

void DatabaseConnector::transactionRollback() {
	SQLite::Statement query(*db, "ROLLBACK");
	query.exec();
}