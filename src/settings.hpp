#pragma once
#include "util/types.hpp"
#include <vector>

struct Room;

namespace ojson {
	class Object;
}

struct Settings {
private:
	void load();
	void loadParams(ojson::Object &obj);

	Room *room;

public:
	std::vector<std::string> plugin_list;
	u32 autosave_interval = 30000; // in milliseconds

	struct {
		bool process_all_at_start = false;
	} preview_system;

	Settings(Room *room);
	~Settings();
};