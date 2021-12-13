#pragma once

#include "session.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"

struct Server;
struct Room;
struct PluginManager;

struct Plugin {
	struct P;
	uniqptr<P> p;

	Plugin(PluginManager *manager, const char *name, const char *dir);
	~Plugin();

	const char *getName();
};

struct PluginManager {
	struct P;
	uniqptr<P> p;

	Room *room;

	PluginManager(Room *room);
	~PluginManager();

	void passMessage(SessionID session_id, const char *message);
	void passCommand(SessionID session_id, const char *command);
	void passUserJoin(SessionID session_id);
	void passUserLeave(SessionID session_id);
	bool passUserMouseDown(SessionID session_id); // true = cancel
	void passUserMouseUp(SessionID session_id);
};