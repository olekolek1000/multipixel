#include "command.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <cstddef>

struct sqlite3;
struct sqlite3_stmt;

enum struct CompressionType : s32 {
	NONE,
	LZ4
};

struct DatabaseRecord {
	//compresion type enum
	CompressionType compression_type;
	//unix timestamp
	s64 created;
	//unix timestamp
	s64 modified;
	//blob from sqlite
	SharedVector<u8> data;
};

struct DatabaseListElement {
	u64 id;
	u64 modified;
};

struct Statement {
	sqlite3 *database = nullptr;
	sqlite3_stmt *statement = nullptr;
	void load(sqlite3 *databse, const char *sql);
	void done();
	~Statement();
};

class DatabaseConnector {
public:
	//construct connection with default file chunk.db
	DatabaseConnector();
	//construct connection
	DatabaseConnector(const char *dbpath);
	~DatabaseConnector();
	//saves blob to db ; creates snaphot automatically
	auto saveBytes(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void;
	auto loadBytes(s32 x, s32 y) -> DatabaseRecord;
	auto listSnapshots(s32 x, s32 y) -> uniqdata<DatabaseListElement>;
	auto setSnapshotInerval(s64 seconds) -> void;
	auto getSnapshotInerval() -> s64;

private:
	auto insert(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void;
	u32 seconds_between_snapshot = 14400;
	sqlite3 *database = nullptr;

	Statement statement_loadBytes;
	Statement statement_saveBytes_select;
	Statement statement_saveBytes_update;
	Statement statement_insert;
};