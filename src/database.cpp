#include "database.hpp"
#include "SQLiteCpp/Statement.h"
#include "src/server.hpp"
#include "util/smartptr.hpp"
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include <stdexcept>
#include <stdio.h>
#include <string>

DatabaseConnector::DatabaseConnector()
		: DatabaseConnector("chunks.db") {
}

DatabaseConnector::DatabaseConnector(const char *dbpath) {
	db.create(dbpath, SQLite::OPEN_CREATE | SQLite::OPEN_READWRITE);

	{
			//SQLite::Statement query(*db, "PRAGMA synchronous=OFF");
			//query.exec();
	}

	{
		SQLite::Statement query(*db, "CREATE TABLE IF NOT EXISTS chunk_data(x INT NOT NULL, y INT NOT NULL, data BLOB, modified INT64 NOT NULL, created INT64 NOT NULL, compression INT);");
		query.exec();
	}

	//X index
	{
		SQLite::Statement query(*db, "CREATE INDEX IF NOT EXISTS index_x on chunk_data(x)");
		query.exec();
	}

	//Y index
	{
		SQLite::Statement query(*db, "CREATE INDEX IF NOT EXISTS index_y on chunk_data(y)");
		query.exec();
	}
}

DatabaseConnector::~DatabaseConnector() {
}

auto DatabaseConnector::saveBytes(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void {
	SQLite::Statement query_select(*db, "SELECT created, rowid FROM chunk_data WHERE x = ? AND y = ? ORDER BY created DESC");
	query_select.bind(1, x);
	query_select.bind(2, y);

	if(query_select.executeStep()) {
		//Chunk already exists, update chunk
		s64 timestamp = query_select.getColumn(0);

		if(time(nullptr) - timestamp > seconds_between_snapshot) {
			insert(x, y, data, size, type);
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
		//Chunk does not exist, create chunk
		insert(x, y, data, size, type);
	}
}

auto DatabaseConnector::loadBytes(s32 x, s32 y) -> DatabaseRecord {
	DatabaseRecord rec;

	SQLite::Statement query(*db, "SELECT data, compression, modified, created FROM chunk_data WHERE x=? AND y=? ORDER BY modified DESC");
	query.bind(1, x);
	query.bind(2, y);

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

auto DatabaseConnector::listSnapshots(s32 x, s32 y) -> uniqdata<DatabaseListElement> {
	SQLite::Statement query(*db, "SELECT rowid, modified FROM chunk_data WHERE x = ? AND y = ? ORDER BY modified DESC");
	query.bind(1, x);
	query.bind(2, y);

	uniqdata<DatabaseListElement> timestamps;
	while(query.executeStep()) {
		timestamps.push_back({query.getColumn(0).getInt64(), query.getColumn(0).getInt64()});
	}

	return timestamps;
}

auto DatabaseConnector::insert(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void {
	SQLite::Statement query(*db, "INSERT INTO chunk_data (x,y,data,modified,created,compression) VALUES(?,?,?,?,?,?)");
	query.bind(1, x);
	query.bind(2, y);
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
}

void Transaction::rollback() {
	if(!connector) return;
	connector->transactionRollback();
	connector = nullptr;
}

std::shared_ptr<Transaction> DatabaseConnector::transactionBegin() {
	auto transaction = std::make_shared<Transaction>();
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