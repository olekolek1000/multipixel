#pragma once

#include "util/smartptr.hpp"
#include "util/types.hpp"

struct Server;
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

	Server *server;

	PluginManager(Server *server);
	~PluginManager();

	void passMessage(u16 session_id, const char *message);
	void passCommand(u16 session_id, const char *command);
	void passUserJoin(u16 session_id);
	void passUserLeave(u16 session_id);
	bool passUserMouseDown(u16 session_id); //true = cancel
	void passUserMouseUp(u16 session_id);
	void passTick();
};