#pragma once
#include "command.hpp"
#include "util/mutex.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <cstddef>
#include <functional>
#include <memory>

enum struct CompressionType : s32 {
	NONE,
	LZ4
};

struct ChunkDatabaseRecord {
	// compresion type enum
	CompressionType compression_type = CompressionType::NONE;
	// unix timestamp
	s64 created = 0;
	// unix timestamp
	s64 modified = 0;
	// blob from sqlite
	SharedVector<u8> data;
};

struct PreviewDatabaseRecord {
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
	LockGuard lock;
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
	void chunkSaveData(Int2 pos, const void *data, size_t size, CompressionType type);
	ChunkDatabaseRecord chunkLoadData(Int2 pos);
	void foreachChunk(std::function<void(Int2)> callback);

	void previewSaveData(Int2 pos, u8 zoom, const void *data, size_t size);
	PreviewDatabaseRecord previewLoadData(Int2 pos, u8 zoom);

	auto listSnapshots(Int2 pos) -> uniqdata<DatabaseListElement>;
	auto setSnapshotInerval(s64 seconds) -> void;
	auto getSnapshotInerval() -> s64;

	std::shared_ptr<Transaction> transactionBegin();
	void transactionCommit();
	void transactionRollback();

	inline void lock() {
		mtx_access.lock();
	}

	inline void unlock() {
		mtx_access.unlock();
	}

private:
	Mutex mtx_access;

	void initTableChunkData();
	void initTablePreviews();

	auto insert(Int2 pos, const void *data, size_t size, CompressionType type) -> void;
	u32 seconds_between_snapshot = 14400;

	uniqptr<SQLite::Database> db;
};