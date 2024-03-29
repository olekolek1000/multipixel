#pragma once

#include "command.hpp"
#include "database.hpp"
#include "session.hpp"
#include "settings.hpp"
#include "util/listener.hpp"
#include "util/mutex.hpp"
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <map>
#include <string_view>

struct BrushShape {
	u8 size; // Width and height
	uniqdata<u8> shape;
};

struct GlobalPixel;

struct Session;
struct ChunkSystem;
struct PreviewSystem;
struct Settings;
struct PluginManager;
struct WsConnection;

struct Room {
	MultiDispatcher<void(Session *)> dispatcher_session_remove;
	EventQueue queue;
	Server *server;
	DatabaseConnector database;
	Settings settings;

private:
	Mutex mtx_sessions;
	std::map<SessionID, Session *> session_map_id; // For fast session lookup
	std::vector<std::weak_ptr<Session>> sessions;

public:
	Room(Server *server, std::string_view name);
	~Room();

	bool tick();

	const std::string &getName();

	void log(const char *name, const char *format, ...);

	// Locking function, do not operate on heavy loads
	void forEverySessionExcept(Session *except, std::function<void(Session *)> callback);

	// Broadcast packet for everyone except one session (optional)
	void broadcast(const Packet &packet, Session *except = nullptr);
	void broadcast_nolock(const Packet &packet, Session *except = nullptr);

	// Returns monochrome brush bitmap
	BrushShape *getBrushShape(u8 size, bool filled);

	ChunkSystem *getChunkSystem() const;
	PreviewSystem *getPreviewSystem() const;
	PluginManager *getPluginManager() const;

	Session *getSession_nolock(SessionID session_id);

	bool addSession(const std::shared_ptr<Session> &session);
	void removeSession_nolock(const std::shared_ptr<Session> &session);
	void removeSession(const std::shared_ptr<Session> &session);
	size_t getSessionCount();

	void setPixels_nolock(GlobalPixel *pixels, u32 count);

private:
	struct P;
	uniqptr<P> p;

	SessionID findFreeSessionID_nolock();
	void freeRemovedSessions();
};