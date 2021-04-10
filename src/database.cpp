#include "database.hpp"
#include "util/smartptr.hpp"
#include <cstdio>
#include <sqlite3.h>
#include <stdio.h>

DatabaseConnector::DatabaseConnector() {
	DatabaseConnector("chunks.db");
}
DatabaseConnector::DatabaseConnector(const char * dbpath) {
		if(sqlite3_open(dbpath, &database) != SQLITE_OK) {
		fprintf(stderr, "Can't open database: %s\n", sqlite3_errmsg(database));
	}
	if(sqlite3_exec(
				 database, "CREATE TABLE IF NOT EXISTS chunk_data(x INT  NOT NULL , y INT    NOT NULL , data BLOB,modified TIMESTAMP, PRIMARY KEY(x ,y)); ",
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

auto DatabaseConnector::saveBytes(s32 x, s32 y, const void *data, size_t size) -> void {
	sqlite3_stmt *statment;
	sqlite3_prepare_v2(
			database,
			"INSERT OR REPLACE INTO chunk_data (x,y,data,modified) VALUES(?,?,?,(SELECT strftime('%s','now')))",
			-1,
			&statment, 0);
	sqlite3_bind_int(statment, 1, x);
	sqlite3_bind_int(statment, 2, y);
	sqlite3_bind_blob(statment, 3, data, size, NULL);

	if(sqlite3_step(statment) == SQLITE_DONE) {
		sqlite3_finalize(statment);
	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
}
auto DatabaseConnector::loadBytes(s32 x, s32 y, bool createIfNotExits) -> uniqdata<u8> {
	sqlite3_stmt *statment;
	sqlite3_prepare_v2(
			database,
			"SELECT data FROM chunk_data WHERE x= ? AND y = ? ",
			-1,
			&statment, 0);
	sqlite3_bind_int(statment, 1, x);
	sqlite3_bind_int(statment, 2, y);
	uniqdata<u8> blob;	
	if(sqlite3_step(statment) == SQLITE_ROW) {
		u32 blob_size =  sqlite3_column_bytes(statment,0);
		u8 * ptr = (u8 *)sqlite3_column_blob(statment, 0);
		blob.resize(blob_size);
		memcpy(blob.ptr,ptr,blob_size);
		sqlite3_finalize(statment);
	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
	return blob;
}
