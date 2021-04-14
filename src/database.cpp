#include "database.hpp"
#include "util/smartptr.hpp"
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include <sqlite3.h>
#include <stdio.h>

DatabaseConnector::DatabaseConnector() {
	DatabaseConnector("chunks.db");
}
DatabaseConnector::DatabaseConnector(const char *dbpath) {
	if(sqlite3_open(dbpath, &database) != SQLITE_OK) {
		fprintf(stderr, "Can't open database: %s\n", sqlite3_errmsg(database));
	}
	if(sqlite3_exec(
				 database, "CREATE TABLE IF NOT EXISTS chunk_data(x INT  NOT NULL , y INT    NOT NULL , data BLOB,modified INT64 NOT NULL,created INT64 NOT NULL, compression INT); ",
				 NULL, NULL, NULL) == SQLITE_OK) {

	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
}

DatabaseConnector::~DatabaseConnector() {
	if(database) {
		sqlite3_close(database);
		database = nullptr;
	}
}

auto DatabaseConnector::saveBytes(s32 x, s32 y, const void *data, size_t size, COMPRESSION_TYPE type) -> void {
	sqlite3_stmt *statment;
	sqlite3_prepare_v2(
			database,
			"SELECT created , rowid FROM chunk_data WHERE x = ? AND y = ? ORDER BY created DESC",
			-1,
			&statment, 0);
	sqlite3_bind_int(statment, 1, x);
	sqlite3_bind_int(statment, 2, y);
	auto res_code = sqlite3_step(statment);
	if(res_code == SQLITE_ROW) {

		u32 timestamp = sqlite3_column_int64(statment, 0);

		if(time(NULL) - timestamp > seconds_between_snapshot) {
			sqlite3_finalize(statment);
			insert(x, y, data, size, type);

		} else {
			auto chunk_id = sqlite3_column_int64(statment, 1);
			sqlite3_reset(statment);
			sqlite3_prepare_v2(
					database,
					"UPDATE chunk_data SET modified = ? , data = ? , compression = ? WHERE rowid = ?",
					-1,
					&statment, 0);
			sqlite3_bind_int64(statment, 1, time(NULL));
			sqlite3_bind_blob(statment, 2, data, size, NULL);
			sqlite3_bind_int(statment, 3, type);
			sqlite3_bind_int64(statment, 4, chunk_id);
			if(sqlite3_step(statment) == SQLITE_DONE) {
			} else {
				fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
			}
		}

	} else if(res_code == SQLITE_DONE) {
		insert(x, y, data, size, type);

	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
}
auto DatabaseConnector::loadBytes(s32 x, s32 y, u32 id) -> DatabaseRecord {
	sqlite3_stmt *statment;
	DatabaseRecord rec;
	if(id == 0) {

		sqlite3_prepare_v2(
				database,
				"SELECT data,compression,modified , created FROM chunk_data WHERE x= ? AND y = ? ORDER BY rowid DESC",
				-1,
				&statment, 0);
		sqlite3_bind_int(statment, 1, x);
		sqlite3_bind_int(statment, 2, y);
	} else {
		sqlite3_prepare_v2(
				database,
				"SELECT data,compression,modified , created FROM chunk_data WHERE x= ? AND y = ? AND rowid = ?",
				-1,
				&statment, 0);
		sqlite3_bind_int(statment, 1, x);
		sqlite3_bind_int(statment, 2, y);
		sqlite3_bind_int64(statment, 3, id);
	}

	if(sqlite3_step(statment) == SQLITE_ROW) {
		u32 blob_size = sqlite3_column_bytes(statment, 0);
		u8 *ptr = (u8 *)sqlite3_column_blob(statment, 0);
		rec.compression_type = sqlite3_column_int(statment, 1);
		rec.modified = sqlite3_column_int64(statment, 2);
		rec.created = sqlite3_column_int64(statment, 3);
		rec.data.resize(blob_size);
		memcpy(rec.data.ptr, ptr, blob_size);
	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
	sqlite3_finalize(statment);
	return rec;
}
auto DatabaseConnector::listSnapshots(s32 x, s32 y) -> uniqdata<DatabseListElement> {
	sqlite3_stmt *statment;
	sqlite3_prepare_v2(
			database,
			"SELECT rowid,  modified FROM chunk_data WHERE x= ? AND y = ? ORDER BY modified DESC",
			-1,
			&statment, 0);
	sqlite3_bind_int(statment, 1, x);
	sqlite3_bind_int(statment, 2, y);
	uniqdata<DatabseListElement> timestamps;
	while (sqlite3_step(statment) == SQLITE_ROW) {
		timestamps.push_back({(u64)sqlite3_column_int64(statment, 0),(u64)sqlite3_column_int64(statment, 1)});
	}
	return timestamps;
}
auto DatabaseConnector::insert(s32 x, s32 y, const void *data, size_t size, COMPRESSION_TYPE type) -> void {
	sqlite3_stmt *statment;
	sqlite3_prepare_v2(
			database,
			"INSERT INTO chunk_data (x,y,data,modified,created,compression) VALUES(?,?,?,?,?,?)",
			-1,
			&statment, 0);

	sqlite3_bind_int(statment, 1, x);
	sqlite3_bind_int(statment, 2, y);
	sqlite3_bind_blob(statment, 3, data, size, NULL);
	sqlite3_bind_int64(statment, 4, time(NULL));
	sqlite3_bind_int64(statment, 5, time(NULL));
	sqlite3_bind_int(statment, 6, type);
	if(sqlite3_step(statment) == SQLITE_DONE) {
		sqlite3_finalize(statment);
	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
}
auto DatabaseConnector::getSnapshotInerval() -> u32{
	return seconds_between_snapshot;
}
auto DatabaseConnector::setSnapshotInerval(u32 seconds) -> void{
	seconds_between_snapshot = seconds;
}