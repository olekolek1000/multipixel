#include "database.hpp"
#include "util/smartptr.hpp"
#include <cstdio>
#include <sqlite3.h>
#include <stdio.h>

DatabaseConnector::DatabaseConnector() {
	if(sqlite3_open("chunks.db", &database) != SQLITE_OK) {
		fprintf(stderr, "Can't open database: %s\n", sqlite3_errmsg(database));
	}
}

DatabaseConnector::~DatabaseConnector() {
	if(database) {
		sqlite3_close(database);
		database = nullptr;
	}
}

auto DatabaseConnector::saveBytes(s32 x, s32 y, uniqdata<u8> data) -> void {
	sqlite3_stmt *statment;
	sqlite3_prepare_v2(
			database,
			"INSERT INTO chunks (xkey,ykey,data) VALUES(?,?,?)",
			-1, &statment, NULL);

	sqlite3_bind_int(statment, 1, x);
	sqlite3_bind_int(statment, 2, y);
	sqlite3_bind_blob(statment, 3, data.data(), data.size_bytes(), NULL);

	if(sqlite3_step(statment) == SQLITE_OK) {
		sqlite3_finalize(statment);
	} else {
		fprintf(stderr, "Can't prepare SQL statment: %s\n", sqlite3_errmsg(database));
	}
}