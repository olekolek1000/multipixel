#include "ws_server.hpp"
#include <atomic>
#include <map>
#include <thread>
#include <websocketpp/common/connection_hdl.hpp>
#include <websocketpp/frame.hpp>

#define ASIO_STANDALONE 1
#include <websocketpp/config/asio_no_tls.hpp>
#include <websocketpp/server.hpp>

typedef websocketpp::server<websocketpp::config::asio> ws_server;
typedef ws_server::message_ptr message_ptr;

struct WsServer::P {
	ws_server server;
	std::atomic<bool> stopped = false;

	std::map<void *, uniqptr<WsConnection>> connections;

	std::function<void(std::shared_ptr<WsMessage>)> message_callback;
	std::function<void(WsConnection *)> close_callback;

	std::thread thr_runner;

	///@returns false on failure
	void removeConnection(const websocketpp::connection_hdl &hdl);
	WsConnection *getConnection(const websocketpp::connection_hdl &hdl);
	bool run(u16 port);
	~P();
};

struct WsConnection::P {
	websocketpp::connection_hdl hdl;
	WsServer::P *server;
	std::string ip;
};

WsConnection::WsConnection() {
	websocketpp::connection_hdl hdl;

	p.create();
}

WsConnection::~WsConnection() {
}

void WsConnection::send(const void *data, size_t size) {
	p->server->server.send(p->hdl, data, size, websocketpp::frame::opcode::binary);
}

void WsConnection::send(const uniqdata<u8> &data) {
	WsConnection::send(data.ptr, data._size);
}

void WsConnection::close() {
	p->server->server.close(p->hdl, {}, {});
}

const char *WsConnection::getIP() {
	return p->ip.c_str();
}

void onMessage(WsServer::P *p, websocketpp::connection_hdl hdl, message_ptr data) {
	auto msg = std::make_shared<WsMessage>();
	auto *con = p->getConnection(hdl);

	if(!con) {
		return;
	}

	msg->data = data->get_raw_payload();
	msg->connection = con;

	p->message_callback(msg);
}

void onClose(WsServer::P *p, websocketpp::connection_hdl hdl) {
	auto *con = p->getConnection(hdl);
	p->close_callback(con);
	p->removeConnection(hdl);
}

void WsServer::P::removeConnection(const websocketpp::connection_hdl &hdl) {
	auto *ptr = hdl.lock().get();
	if(!ptr)
		return;

	auto it = connections.find(ptr);
	if(it == connections.end())
		return;

	connections.erase(it);
}

WsConnection *WsServer::P::getConnection(const websocketpp::connection_hdl &hdl) {
	auto *ptr = hdl.lock().get();
	if(!ptr)
		return nullptr;

	auto &connection = connections[ptr];
	if(!connection) {
		connection.create();
		connection->p->server = this;
		connection->p->hdl = hdl;

		{
			//https://github.com/zaphoyd/websocketpp/issues/694#issuecomment-454623641
			auto con = server.get_con_from_hdl(hdl);
			const asio::basic_stream_socket<asio::ip::tcp> &raw_socket = con->get_raw_socket();
			const asio::ip::basic_endpoint<asio::ip::tcp> &rep = raw_socket.remote_endpoint();
			asio::ip::address address = rep.address();

			//Convert IP address to string
			connection->p->ip = address.to_string();
		}
	}

	return connection.get();
}

bool WsServer::P::run(u16 port) {
	stopped = false;

	try {
		// Set logging settings
		server.set_access_channels(websocketpp::log::alevel::none);
		server.clear_access_channels(websocketpp::log::alevel::none);
		server.set_reuse_addr(true);

		// Initialize Asio
		server.init_asio();

		// Register our message handler
		server.set_message_handler(std::bind(&onMessage, this, std::placeholders::_1, std::placeholders::_2));
		server.set_close_handler(std::bind(&onClose, this, std::placeholders::_1));

		server.listen(asio::ip::tcp::v4(), port);

		// Start the server accept loop
		server.start_accept();

		thr_runner = std::thread([this]() {
			// Start the ASIO io_service run loop
			while(!stopped) {
				try {
					server.run_one();
				} catch(std::exception &e) {
					fprintf(stderr, "websocket runner exception: %s\n", e.what());
				}
			}
		});
	} catch(std::exception &e) {
		fprintf(stderr, "Exception: %s\n", e.what());
		return false;
	}

	return true;
}

WsServer::P::~P() {
	stopped = true;
	server.stop();
	server.stop_listening();
	if(thr_runner.joinable())
		thr_runner.join();
}

WsServer::WsServer() {
	p.create();
}

WsServer::~WsServer() {
}

bool WsServer::run(u16 port, std::function<void(std::shared_ptr<WsMessage>)> message_callback, std::function<void(WsConnection *)> close_callback) {
	p->message_callback = message_callback;
	p->close_callback = close_callback;
	return p->run(port);
}