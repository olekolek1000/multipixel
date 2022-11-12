#include "settings.hpp"
#include "lib/ojson.hpp"
#include "room.hpp"
#include "server.hpp"
#include "util/logs.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <fstream>

void Settings::load() {
	std::ifstream file;
	file.open("settings.json", std::ios::ate | std::ios::binary);
	if(!file.good()) {
		throwf("settings.json not found");
	}

	auto size_bytes = file.tellg();
	if(size_bytes < 0) {
		throwf("Invalid file");
	}

	file.seekg(0);

	uniqdata<u8> data;
	data.resize(size_bytes);
	file.read((char *)data.data(), data.size_bytes());

	uniqptr<ojson::Element> parsed;

	parsed = ojson::parseJSON(data.data(), data.size());
	if(!parsed) {
		throwf("Invalid JSON");
	}

	auto *obj = parsed->castObject();
	if(!obj) {
		throwf("Invalid JSON");
	}

	loadParams(*obj);
}

void Settings::loadParams(ojson::Object &obj) {
	if(auto *json = obj.getNumber("autosave_interval")) {
		this->autosave_interval = json->getInt();
	}

	if(auto *arr = obj.getArray("plugin_list")) {
		arr->foreach([&](ojson::Element *e) {
			auto *str = e->castString();
			if(!str) return;
			plugin_list.push_back(str->get());
		});
	}

	if(auto *preview_system = obj.getObject("preview_system")) {
		auto &ps = this->preview_system;

		if(auto *json = preview_system->getBoolean("process_all_at_start"))
			ps.process_all_at_start = json->get();
	}
}

Settings::Settings(Room *room)
		: room(room) {
	try {
		load();
	} catch(std::exception &e) {
		room->log("Settings", "Failed to load settings file: %s", e.what());
	}
}

Settings::~Settings() {
}