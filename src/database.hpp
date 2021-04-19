#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <cstddef>

struct sqlite3;
enum COMPRESSION_TYPE : s32 {
	NONE,
	LZ4
};

struct DatabaseRecord {
	//compresion type enum
	COMPRESSION_TYPE compression_type;
	//unix timestamp
	u64 created;
	//unix timestamp
	u64 modified;
	//blob from sqlite
	uniqdata<u8> data;
};

struct DatabseListElement {
	u64 id;
	u64 modified;
};

class DatabaseConnector {
public:
	//construct connection with default file chunk.db
	DatabaseConnector();
	//construct connection
	DatabaseConnector(const char *dbpath);
	~DatabaseConnector();
	//saves blob to db ; creates snaphot automatically
	auto saveBytes(s32 x, s32 y, const void *data, size_t size, COMPRESSION_TYPE type) -> void;
	auto loadBytes(s32 x, s32 y, u32 id = 0) -> DatabaseRecord;
	auto listSnapshots(s32 x, s32 y) -> uniqdata<DatabseListElement>;
	auto setSnapshotInerval(u32 seconds) -> void;
	auto getSnapshotInerval() -> u32;

protected:
	auto insert(s32 x, s32 y, const void *data, size_t size, COMPRESSION_TYPE type) -> void;
	u32 seconds_between_snapshot = 14400;
	sqlite3 *database = nullptr;
};