#pragma once
#include "util/smartptr.hpp"
#include "util/types.hpp"
#include <functional>
#include <memory>
#include <string>

struct WsConnection {
	struct P;
	uniqptr<P> p;

	WsConnection();
	~WsConnection();

	void send(const void *data, size_t size);
	void send(const uniqdata<u8> &data);
	void close();
	const char *getIP();
};

typedef std::shared_ptr<WsConnection> SharedWsConnection;

struct WsMessage {
	SharedWsConnection connection;
	std::string data; //Raw message data
};

struct WsServer {
	struct P;
	uniqptr<P> p;

	WsServer();
	~WsServer();

	/// @returns true on success
	bool run(u16 port, std::function<void(std::shared_ptr<WsMessage>)> message_callback, std::function<void(SharedWsConnection &)> close_callback);
};