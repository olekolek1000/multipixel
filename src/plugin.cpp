#include "plugin.hpp"
#include "lib/ojson.hpp"
#include "room.hpp"
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

#define SOL_ALL_SAFETIES_ON							1
#define SOL_EXCEPTIONS_SAFE_PROPAGATION 1
#include "lib/sol/sol.hpp"

static const char *LOG_PMAN = "PluginManager";

struct PluginManager::P {
	PluginManager *plugman;
	Room *room;

	Mutex mtx;

	MultiDispatcher<void(SessionID, const char *)> dispatcher_message; // session_id, message
	MultiDispatcher<void(SessionID, const char *)> dispatcher_command; // session_id, command
	MultiDispatcher<void(SessionID)> dispatcher_user_join;						 // session_id
	MultiDispatcher<void(SessionID)> dispatcher_user_leave;						 // session_id
	MultiDispatcher<bool(SessionID)> dispatcher_user_mouse_down;			 // cancelled(session_id)
	MultiDispatcher<void(SessionID)> dispatcher_user_mouse_up;				 // session_id
	MultiDispatcher<void()> dispatcher_tick;

	std::vector<uniqptr<Plugin>> plugins;

	P(PluginManager *plugman, Room *room);
	void init();
	bool loadPlugins();
	bool loadPlugin(const char *name);
};

PluginManager::P::P(PluginManager *plugman, Room *room)
		: plugman(plugman), room(room) {
}

void PluginManager::P::init() {
	loadPlugins();
}

bool PluginManager::P::loadPlugins() {
	room->log(LOG_PMAN, "Loading plugins");

	for(auto &plugin_name : room->settings.plugin_list) {
		loadPlugin(plugin_name.c_str());
	}

	room->log(LOG_PMAN, "Plugins loaded");

	return true;
}

bool PluginManager::P::loadPlugin(const char *name) {
	room->log(LOG_PMAN, "Loading plugin [%s]", name);

	auto name_len = strlen(name);
	for(u32 i = 0; i < name_len; i++) {
		char c = name[i];
		if((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' || c == '-') {
			continue;
		}
		room->log(LOG_PMAN, "Plugin name contains invalid characters. Only Aa-Zz, 0-9, _- are allowed.");
		return false;
	}

	char dir[256];
	snprintf(dir, sizeof(dir), "plugins/%s/", name);

	try {
		auto &plugin = plugins.emplace_back();
		plugin.create(plugman, name, dir);
	} catch(std::exception &e) {
		room->log(LOG_PMAN, "Failed to load plugin [%s]: %s", name, e.what());
		plugins.pop_back();
	}

	return true;
}

PluginManager::PluginManager(Room *room)
		: room(room) {
	p.create(this, room);
	p->init();
}

PluginManager::~PluginManager() {
}

void PluginManager::passMessage(SessionID session_id, const char *message) {
	p->dispatcher_message.triggerAll(session_id, message);
}

void PluginManager::passCommand(SessionID session_id, const char *command) {
	p->dispatcher_command.triggerAll(session_id, command);
}

void PluginManager::passUserJoin(SessionID session_id) {
	p->dispatcher_user_join.triggerAll(session_id);
}

void PluginManager::passUserLeave(SessionID session_id) {
	p->dispatcher_user_leave.triggerAll(session_id);
}

bool PluginManager::passUserMouseDown(SessionID session_id) {
	for(auto &l : p->dispatcher_user_mouse_down.listeners) {
		if(l->callback(session_id))
			return true;
	}
	return false;
}

void PluginManager::passUserMouseUp(SessionID session_id) {
	p->dispatcher_user_mouse_up.triggerAll(session_id);
}

void PluginManager::passTick() {
	p->dispatcher_tick.triggerAll();
}

// ##############################################################
// ##############################################################
// ##############################################################

struct Plugin::P {
	PluginManager *plugman;
	Room *room;
	bool loaded = false;

	std::string name;

	sol::state lua;

	Listener<void(SessionID, const char *)> listener_message; // session_id, message
	Listener<void(SessionID, const char *)> listener_command; // session_id, command
	Listener<void(SessionID)> listener_user_join;							// session_id
	Listener<void(SessionID)> listener_user_leave;						// session_id
	Listener<bool(SessionID)> listener_user_mouse_down;				// cancelled(session_id)
	Listener<void(SessionID)> listener_user_mouse_up;					// session_id
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
			room(plugman->room) {
	this->name = name;

	char init_path[256];
	snprintf(init_path, sizeof(init_path), "%s/init.lua", dir);

	lua.open_libraries(sol::lib::base, sol::lib::package, sol::lib::table, sol::lib::io, sol::lib::string);
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

template <typename... Args>
static auto executeFunction(const sol::function &func, Args... args) {
	auto ret = func(args...);
	if(!ret.valid()) {
		sol::error err = ret;
		throwf("Failed to call function: %s", err.what());
	}
	return ret;
}

void Plugin::P::populateAPI() {
	lua.set_function("print", [this](const char *text) {
		room->log(name.c_str(), "%s", text);
	});

	auto tab_server = lua.create_table("server");

	tab_server.set_function("addEvent", [this](const char *event_name, sol::function func) {
		auto *pm = plugman->p.get();

		if(!strcmp(event_name, "tick")) {
			pm->dispatcher_tick.add(listener_tick, [func{std::move(func)}] {
				executeFunction(func);
			});
		} else if(!strcmp(event_name, "message")) {
			pm->dispatcher_message.add(listener_message, [func{std::move(func)}](SessionID session_id, const char *message) {
				executeFunction(func, session_id.get(), message);
			});
		} else if(!strcmp(event_name, "command")) {
			pm->dispatcher_command.add(listener_command, [func{std::move(func)}](SessionID session_id, const char *command) {
				executeFunction(func, session_id.get(), command);
			});
		} else if(!strcmp(event_name, "user_join")) {
			pm->dispatcher_user_join.add(listener_user_join, [func{std::move(func)}](SessionID session_id) {
				executeFunction(func, session_id.get());
			});
		} else if(!strcmp(event_name, "user_leave")) {
			pm->dispatcher_user_leave.add(listener_user_leave, [func{std::move(func)}](SessionID session_id) {
				executeFunction(func, session_id.get());
			});
		} else if(!strcmp(event_name, "user_mouse_down")) {
			pm->dispatcher_user_mouse_down.add(listener_user_mouse_down, [func{std::move(func)}](SessionID session_id) -> bool {
				bool result;
				if(resGetBoolean(executeFunction(func, session_id.get()), &result)) {
					return result;
				}
				return false;
			});
		} else if(!strcmp(event_name, "user_mouse_up")) {
			pm->dispatcher_user_mouse_up.add(listener_user_mouse_up, [func{std::move(func)}](SessionID session_id) {
				executeFunction(func, session_id.get());
			});
		} else {
			room->log(name.c_str(), "Unknown event name: %s", event_name);
		}
	});

	tab_server.set_function("chatBroadcast", [this](const char *text) {
		room->broadcast_nolock(preparePacketMessage(MessageType::plain_text, text));
	});

	tab_server.set_function("chatBroadcastHTML", [this](const char *text) {
		room->broadcast_nolock(preparePacketMessage(MessageType::html, text));
	});

	tab_server.set_function("userSendMessage", [this](u16 session_id /*Needs to be u16, called by sol library*/, const char *text) {
		auto *s = room->getSession_nolock(session_id);
		if(!s) return;
		s->pushPacket(preparePacketMessage(MessageType::plain_text, text));
	});

	tab_server.set_function("userSendMessageHTML", [this](u16 session_id, const char *text) {
		auto *s = room->getSession_nolock(session_id);
		if(!s) return;
		s->pushPacket(preparePacketMessage(MessageType::html, text));
	});

	tab_server.set_function("userGetName", [this](u16 session_id) {
		auto *s = room->getSession_nolock(session_id);
		if(!s) return "";
		return s->getNickname().c_str();
	});

	tab_server.set_function("userGetPosition", [this](u16 session_id) {
		auto *s = room->getSession_nolock(session_id);
		// TODO return nil if failed
		if(!s)
			return std::pair(0, 0);
		s32 x, y;
		s->getMousePosition(&x, &y);
		return std::pair(x, y);
	});

	tab_server.set_function("mapSetPixel", [this](s32 global_x, s32 global_y, u8 r, u8 g, u8 b) {
		Int2 global{global_x, global_y};
		auto chunk_pos = ChunkSystem::globalPixelPosToChunkPos(global);
		auto *chunk = room->getChunkSystem()->getChunk(chunk_pos);
		if(!chunk) return;
		ChunkPixel pixel;
		pixel.pos = ChunkSystem::globalPixelPosToLocalPixelPos(global);
		pixel.color = Color(r, g, b);
		chunk->setPixelQueued(&pixel);
	});

	tab_server.set_function("mapBlitGray", [this](s32 posX, s32 posY, u32 width, u32 height, const std::string &data) {
		if(data.size() != width * height) {
			throwf("mapBlitGray: Data size mismatch");
		}

		uniqdata<GlobalPixel> pixels(width * height);

		Int2 global;
		u32 offset = 0;
		for(u32 y = 0; y < height; y++) {
			for(u32 x = 0; x < width; x++) {
				global.x = posX + x;
				global.y = posY + y;

				auto &pixel = pixels[offset];
				pixel.pos = global;
				u8 gray = data[offset];
				pixel.color = Color(gray, gray, gray);
				offset++;
			}
		}

		room->setPixels_nolock(pixels.data(), pixels.size());
	});

	tab_server.set_function("mapBlitRGB", [this](s32 posX, s32 posY, u32 width, u32 height, const std::string &data) {
		if(data.size() != width * height * 3) {
			throwf("mapBlitRGB: Data size mismatch");
		}

		uniqdata<GlobalPixel> pixels(width * height);

		Int2 global;
		u32 offset = 0;
		for(u32 y = 0; y < height; y++) {
			for(u32 x = 0; x < width; x++) {
				global.x = posX + x;
				global.y = posY + y;

				auto &pixel = pixels[offset];
				pixel.pos = global;
				pixel.color.r = data[offset * 3 + 0];
				pixel.color.g = data[offset * 3 + 1];
				pixel.color.b = data[offset * 3 + 2];
				offset++;
			}
		}

		room->setPixels_nolock(pixels.data(), pixels.size());
	});
}

Plugin::P::~P() {
	if(loaded) {
		callFunction("onUnload", false);
	}
}
