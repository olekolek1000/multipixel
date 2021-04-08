#include "util/types.hpp"
#include "util/smartptr.hpp"
struct sqlite3;
 class DatabaseConnector{
	public:
	 DatabaseConnector();
	 ~DatabaseConnector();
	 auto saveBytes(s32 x,s32 y,uniqdata<u8> data) -> void;
	 auto loadBytes(s32 x,s32 y, bool createIfNotExits = true) -> uniqdata<u8>;
	 uniqptr<sqlite3> database;
	 
 };