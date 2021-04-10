#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <cstddef>
struct sqlite3;
class DatabaseConnector {
public:
	DatabaseConnector();
	DatabaseConnector(const char * dbpath);
	~DatabaseConnector();
	auto saveBytes(s32 x, s32 y, const void *data ,size_t size) -> void;
	auto createEnteryIfNotExists(s32 x, s32 y) -> void;
	auto loadBytes(s32 x, s32 y, bool createIfNotExits = true) -> uniqdata<u8>;
	sqlite3 *database = nullptr;
};