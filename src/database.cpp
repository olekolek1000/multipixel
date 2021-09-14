#include "database.hpp"
#include "util/smartptr.hpp"
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include <sqlite3.h>
#include <stdexcept>
#include <stdio.h>
#include <string>

Statement::~Statement() {
	if(statement) {
		sqlite3_finalize(statement);
		statement = nullptr;
	}
}

void Statement::load(sqlite3 *database, const char *sql) {
	this->database = database;

	if(statement)
		sqlite3_finalize(statement);

	sqlite3_prepare_v2(database, sql, -1, &statement, nullptr);

	if(!statement)
		throw std::runtime_error("Invalid statement: " + std::string(sql));
}

void Statement::done() {
	sqlite3_reset(statement);
}

DatabaseConnector::DatabaseConnector()
		: DatabaseConnector("chunks.db") {
}

DatabaseConnector::DatabaseConnector(const char *dbpath) {
	if(sqlite3_open_v2(dbpath, &database, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE, nullptr) != SQLITE_OK) {
		fprintf(stderr, "Can't open database: %s\n", sqlite3_errmsg(database));
	}
	if(sqlite3_exec(
				 database, "CREATE TABLE IF NOT EXISTS chunk_data(x INT  NOT NULL , y INT    NOT NULL , data BLOB,modified INT64 NOT NULL,created INT64 NOT NULL, compression INT); ",
				 NULL, NULL, NULL) == SQLITE_OK) {

	} else {
		fprintf(stderr, "Can't prepare SQL statement (DatabaseConnector constructor): %s\n", sqlite3_errmsg(database));
	}

	statement_loadBytes.load(database, "SELECT data, compression, modified, created FROM chunk_data WHERE x=? AND y=? ORDER BY rowid DESC");

	statement_saveBytes_select.load(database, "SELECT created , rowid FROM chunk_data WHERE x = ? AND y = ? ORDER BY created DESC");
	statement_saveBytes_update.load(database, "UPDATE chunk_data SET modified = ? , data = ? , compression = ? WHERE rowid = ?");

	statement_insert.load(database, "INSERT INTO chunk_data (x,y,data,modified,created,compression) VALUES(?,?,?,?,?,?)");
}

DatabaseConnector::~DatabaseConnector() {
	if(database) {
		sqlite3_close(database);
		database = nullptr;
	}
}

auto DatabaseConnector::saveBytes(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void {
	auto *statement_select = statement_saveBytes_select.statement;
	sqlite3_bind_int(statement_select, 1, x);
	sqlite3_bind_int(statement_select, 2, y);
	auto res_code = sqlite3_step(statement_select);
	if(res_code == SQLITE_ROW) {
		s64 timestamp = sqlite3_column_int64(statement_select, 0);

		if(time(NULL) - timestamp > seconds_between_snapshot) {
			statement_saveBytes_select.done();
			insert(x, y, data, size, type);
		} else {
			auto chunk_id = sqlite3_column_int64(statement_select, 1);
			statement_saveBytes_select.done();

			auto *statement_update = statement_saveBytes_update.statement;
			sqlite3_bind_int64(statement_update, 1, time(NULL));
			sqlite3_bind_blob(statement_update, 2, data, size, NULL);
			sqlite3_bind_int(statement_update, 3, (int)type);
			sqlite3_bind_int64(statement_update, 4, chunk_id);
			if(sqlite3_step(statement_update) == SQLITE_DONE) {
			} else {
				fprintf(stderr, "Can't prepare SQL statement (saveBytes): %s\n", sqlite3_errmsg(database));
			}

			statement_saveBytes_update.done();
		}
	} else if(res_code == SQLITE_DONE) {
		insert(x, y, data, size, type);
	} else {
		fprintf(stderr, "Can't prepare SQL statement (saveBytes): %s\n", sqlite3_errmsg(database));
	}
}

auto DatabaseConnector::loadBytes(s32 x, s32 y) -> DatabaseRecord {
	DatabaseRecord rec;

	auto *st = statement_loadBytes.statement;

	sqlite3_bind_int(st, 1, x);
	sqlite3_bind_int(st, 2, y);

	if(sqlite3_step(st) == SQLITE_ROW) {
		u32 blob_size = sqlite3_column_bytes(st, 0);
		u8 *ptr = (u8 *)sqlite3_column_blob(st, 0);
		rec.compression_type = (CompressionType)sqlite3_column_int(st, 1);
		rec.modified = sqlite3_column_int64(st, 2);
		rec.created = sqlite3_column_int64(st, 3);
		rec.data = createSharedVector<u8>(blob_size);
		memcpy(rec.data->data(), ptr, blob_size);
	} else if(sqlite3_step(st) == SQLITE_DONE) {
		//nothing
	} else {
		fprintf(stderr, "Can't prepare SQL statement (loadBytes): %s\n", sqlite3_errmsg(database));
	}

	statement_loadBytes.done();

	return rec;
}

auto DatabaseConnector::listSnapshots(s32 x, s32 y) -> uniqdata<DatabaseListElement> {
	sqlite3_stmt *statement;
	sqlite3_prepare_v2(
			database,
			"SELECT rowid,  modified FROM chunk_data WHERE x= ? AND y = ? ORDER BY modified DESC",
			-1,
			&statement, 0);
	sqlite3_bind_int(statement, 1, x);
	sqlite3_bind_int(statement, 2, y);
	uniqdata<DatabaseListElement> timestamps;
	while(sqlite3_step(statement) == SQLITE_ROW) {
		timestamps.push_back({(u64)sqlite3_column_int64(statement, 0), (u64)sqlite3_column_int64(statement, 1)});
	}
	return timestamps;
}

auto DatabaseConnector::insert(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void {
	auto *st = statement_insert.statement;

	sqlite3_bind_int(st, 1, x);
	sqlite3_bind_int(st, 2, y);
	sqlite3_bind_blob(st, 3, data, size, NULL);
	sqlite3_bind_int64(st, 4, time(NULL));
	sqlite3_bind_int64(st, 5, time(NULL));
	sqlite3_bind_int(st, 6, (int)type);
	if(sqlite3_step(st) == SQLITE_DONE) {
		statement_insert.done();
	} else {
		fprintf(stderr, "Can't prepare SQL statement (insert): %s\n", sqlite3_errmsg(database));
	}
}

auto DatabaseConnector::getSnapshotInerval() -> s64 {
	return seconds_between_snapshot;
}

auto DatabaseConnector::setSnapshotInerval(s64 seconds) -> void {
	seconds_between_snapshot = seconds;
}