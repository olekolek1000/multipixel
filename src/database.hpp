
#include "command.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <cstddef>
#include <memory>

enum struct CompressionType : s32 {
	NONE,
	LZ4
};

struct DatabaseRecord {
	// compresion type enum
	CompressionType compression_type;
	// unix timestamp
	s64 created;
	// unix timestamp
	s64 modified;
	// blob from sqlite
	SharedVector<u8> data;
};

struct DatabaseListElement {
	s64 rowid;
	s64 modified;
};

namespace SQLite {
	class Database;
}

struct DatabaseConnector;

struct Transaction {
	DatabaseConnector *connector = nullptr;
	void commit();
	void rollback();
	~Transaction();
};

struct DatabaseConnector {
public:
	void init(const char *dbpath);
	DatabaseConnector();
	~DatabaseConnector();
	// saves blob to db ; creates snaphot automatically
	auto saveBytes(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void;
	auto loadBytes(s32 x, s32 y) -> DatabaseRecord;
	auto listSnapshots(s32 x, s32 y) -> uniqdata<DatabaseListElement>;
	auto setSnapshotInerval(s64 seconds) -> void;
	auto getSnapshotInerval() -> s64;

	std::shared_ptr<Transaction> transactionBegin();
	void transactionCommit();
	void transactionRollback();

private:
	auto insert(s32 x, s32 y, const void *data, size_t size, CompressionType type) -> void;
	u32 seconds_between_snapshot = 14400;

	uniqptr<SQLite::Database> db;
};