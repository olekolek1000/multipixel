#include "plugin.hpp"
#include "lib/ojson.hpp"
#include "server.hpp"
#include "session.hpp"
#include "src/chunk.hpp"
#include "src/chunk_system.hpp"
#include "util/listener.hpp"
#include "util/logs.hpp"
#include "util/mutex.hpp"
#include <fstream>
#include <stdexcept>
#include <string>

#define SOL_ALL_SAFETIES_ON 1
#include "lib/sol/sol.hpp"

static const char *LOG_PMAN = "PluginManager";

struct PluginManager::P {
	PluginManager *plugman;
	Server *server;

	Mutex mtx;

	MultiDispatcher<void(u16, const char *)> dispatcher_message; //session_id, message
	MultiDispatcher<void(u16, const char *)> dispatcher_command; //session_id, command
	MultiDispatcher<void(u16)> dispatcher_user_join;						 //session_id
	MultiDispatcher<void(u16)> dispatcher_user_leave;						 //session_id
	MultiDispatcher<bool(u16)> dispatcher_user_mouse_down;			 //cancelled(session_id)
	MultiDispatcher<void(u16)> dispatcher_user_mouse_up;				 //session_id
	MultiDispatcher<void()> dispatcher_tick;

	std::vector<uniqptr<Plugin>> plugins;

	P(PluginManager *plugman, Server *server);
	void init();
	bool loadPlugins();
	bool loadPlugin(const char *name);
};

PluginManager::P::P(PluginManager *plugman, Server *server)
		: plugman(plugman), server(server) {
}

void PluginManager::P::init() {
	loadPlugins();
}

bool PluginManager::P::loadPlugins() {
	server->log(LOG_PMAN, "Loading plugins");

	bool ok = false;

	do {
		std::ifstream file;
		file.open("plugins/list.json", std::ios::ate | std::ios::binary);
		if(!file.good())
			break;

		auto size_bytes = file.tellg();
		if(size_bytes < 0)
			break;

		file.seekg(0);

		uniqdata<u8> data;
		data.resize(size_bytes);
		file.read((char *)data.data(), data.size_bytes());

		uniqptr<ojson::Element> parsed;

		try {
			parsed = ojson::parseJSON(data.data(), data.size());
		} catch(std::exception &e) {
			server->log(LOG_PMAN, "JSON error: %s", e.what());
			break;
		}

		if(!parsed)
			break;

		//For every plugin name in list
		auto *arr = parsed->castArray();
		if(!arr)
			break;

		arr->foreach([&](ojson::Element *e) {
			auto *str = e->castString();
			if(!str) return;

			loadPlugin(str->get().c_str());
		});

		ok = true;
	} while(0);

	if(!ok) {
		server->log(LOG_PMAN, "Cannot load plugin list or invalid format (Array of JSON strings expected)");
		return false;
	}

	server->log(LOG_PMAN, "Plugins loaded");

	return true;
}

bool PluginManager::P::loadPlugin(const char *name) {
	server->log(LOG_PMAN, "Loading plugin [%s]", name);

	auto name_len = strlen(name);
	for(u32 i = 0; i < name_len; i++) {
		char c = name[i];
		if((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' || c == '-') {
			continue;
		}
		server->log(LOG_PMAN, "Plugin name contains invalid characters. Only Aa-Zz, 0-9, _- are allowed.");
		return false;
	}

	char dir[256];
	snprintf(dir, sizeof(dir), "plugins/%s/", name);

	try {
		auto &plugin = plugins.emplace_back();
		plugin.create(plugman, name, dir);
	} catch(std::exception &e) {
		server->log(LOG_PMAN, "Failed to load plugin [%s]: %s", name, e.what());
		plugins.pop_back();
	}

	return true;
}

PluginManager::PluginManager(Server *server)
		: server(server) {
	p.create(this, server);
	p->init();
}

PluginManager::~PluginManager() {
}

void PluginManager::passMessage(u16 session_id, const char *message) {
	p->dispatcher_message.triggerAll(session_id, message);
}

void PluginManager::passCommand(u16 session_id, const char *command) {
	p->dispatcher_command.triggerAll(session_id, command);
}

void PluginManager::passUserJoin(u16 session_id) {
	p->dispatcher_user_join.triggerAll(session_id);
}

void PluginManager::passUserLeave(u16 session_id) {
	p->dispatcher_user_leave.triggerAll(session_id);
}

bool PluginManager::passUserMouseDown(u16 session_id) {
	for(auto &l : p->dispatcher_user_mouse_down.listeners) {
		if(l->callback(session_id))
			return true;
	}
	return false;
}

void PluginManager::passUserMouseUp(u16 session_id) {
	p->dispatcher_user_mouse_up.triggerAll(session_id);
}

void PluginManager::passTick() {
	p->dispatcher_tick.triggerAll();
}

//##############################################################
//##############################################################
//##############################################################

struct Plugin::P {
	PluginManager *plugman;
	Server *server;
	bool loaded = false;

	std::string name;

	sol::state lua;

	Listener<void(u16, const char *)> listener_message; //session_id, message
	Listener<void(u16, const char *)> listener_command; //session_id, command
	Listener<void(u16)> listener_user_join;							//session_id
	Listener<void(u16)> listener_user_leave;						//session_id
	Listener<bool(u16)> listener_user_mouse_down;				//cancelled(session_id)
	Listener<void(u16)> listener_user_mouse_up;					//session_id
	Listener<void()> listener_tick;

	P(PluginManager *plugman, const char *name, const char *dir);
	void callFunction(const char *name, bool required);
	void populateAPI();
	~P();
};

Plugin::Plugin(PluginManager *plugman, const char *name, const char *dir) {
	p.create(plugman, name, dir);
}

Plugin::~Plugin() {
}

const char *Plugin::getName() {
	return p->name.c_str();
}

void Plugin::P::callFunction(const char *name, bool required) {
	sol::function func = lua[name];
	if(!func) {
		if(required) {
			throwf("Failed to call required function %s", name);
		}
		return;
	}
	func();
}

Plugin::P::P(PluginManager *plugman, const char *name, const char *dir)
		: plugman(plugman),
			server(plugman->server) {
	this->name = name;

	char init_path[256];
	snprintf(init_path, sizeof(init_path), "%s/init.lua", dir);

	lua.open_libraries(sol::lib::base, sol::lib::package, sol::lib::table, sol::lib::io);
	populateAPI();

	lua.script_file(init_path);

	callFunction("onLoad", true);

	loaded = true;
}

auto resGetBoolean = [](auto res, bool *result) -> bool {
	auto type = res.get_type();
	if(type == sol::type::boolean) {
		*result = (bool)res;
		return true;
	}
	return false;
};

void Plugin::P::populateAPI() {
	lua.set_function("print", [this](const char *text) {
		server->log(name.c_str(), "%s", text);
	});

	auto tab_server = lua.create_table("server");

	tab_server.set_function("addEvent", [this](const char *event_name, sol::function func) {
		auto *pm = plugman->p.get();

		if(!strcmp(event_name, "tick")) {
			pm->dispatcher_tick.add(listener_tick, [func{std::move(func)}] {
				func();
			});
		} else if(!strcmp(event_name, "message")) {
			pm->dispatcher_message.add(listener_message, [func{std::move(func)}](u16 session_id, const char *message) {
				func(session_id, message);
			});
		} else if(!strcmp(event_name, "command")) {
			pm->dispatcher_command.add(listener_command, [func{std::move(func)}](u16 session_id, const char *command) {
				func(session_id, command);
			});
		} else if(!strcmp(event_name, "user_join")) {
			pm->dispatcher_user_join.add(listener_user_join, [func{std::move(func)}](u16 session_id) {
				func(session_id);
			});
		} else if(!strcmp(event_name, "user_leave")) {
			pm->dispatcher_user_leave.add(listener_user_leave, [func{std::move(func)}](u16 session_id) {
				func(session_id);
			});
		} else if(!strcmp(event_name, "user_mouse_down")) {
			pm->dispatcher_user_mouse_down.add(listener_user_mouse_down, [func{std::move(func)}](u16 session_id) -> bool {
				bool result;
				if(resGetBoolean(func(session_id), &result)) {
					return result;
				}
				return false;
			});
		} else if(!strcmp(event_name, "user_mouse_up")) {
			pm->dispatcher_user_mouse_up.add(listener_user_mouse_up, [func{std::move(func)}](u16 session_id) {
				func(session_id);
			});
		} else {
			server->log(name.c_str(), "Unknown event name: %s", event_name);
		}
	});

	tab_server.set_function("chatBroadcast", [this](const char *text) {
		server->broadcast_nolock(preparePacketMessage(MessageType::plain_text, text));
	});

	tab_server.set_function("chatBroadcastHTML", [this](const char *text) {
		server->broadcast_nolock(preparePacketMessage(MessageType::html, text));
	});

	tab_server.set_function("userSendMessage", [this](u16 session_id, const char *text) {
		auto *s = server->getSession_nolock(session_id);
		if(!s) return;
		s->pushPacket(preparePacketMessage(MessageType::plain_text, text));
	});

	tab_server.set_function("userSendMessageHTML", [this](u16 session_id, const char *text) {
		auto *s = server->getSession_nolock(session_id);
		if(!s) return;
		s->pushPacket(preparePacketMessage(MessageType::html, text));
	});

	tab_server.set_function("userGetName", [this](u16 session_id) {
		auto *s = server->getSession_nolock(session_id);
		if(!s) return "";
		return s->getNickname().c_str();
	});

	tab_server.set_function("userGetPosition", [this](u16 session_id) {
		auto *s = server->getSession_nolock(session_id);
		//TODO return nil if failed
		if(!s)
			return std::pair(0, 0);
		s32 x, y;
		s->getMousePosition(&x, &y);
		return std::pair(x, y);
	});

	tab_server.set_function("mapSetPixel", [this](s32 global_x, s32 global_y, u8 r, u8 g, u8 b) {
		Int2 global{global_x, global_y};
		auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(global);
		auto *chunk = server->getChunkSystem()->getChunk(chunk_pos);
		if(!chunk) return;
		ChunkPixel pixel;
		pixel.pos = ChunkSystem::globalPixelPosToLocalPixelPos(global);
		pixel.r = r;
		pixel.g = g;
		pixel.b = b;
		chunk->setPixelQueued(&pixel);
	});
}

Plugin::P::~P() {
	if(loaded) {
		callFunction("onUnload", false);
	}
}
