#include "server.hpp"

int main() {
	Server server;
	u16 port = 59900;
	printf("Hosting on port %u\n", port);
	server.run(port);
	printf("Goodbye\n");
	return 0;
}
