#include "logs.hpp"
#include <stdarg.h>
#include <stdexcept>
#include <string>

void errorf(const char *format, ...) {
	char buf[4096];
	va_list arglist;
	va_start(arglist, format);
	vsnprintf(buf, sizeof(buf), format, arglist);
	va_end(arglist);

	fprintf(stderr, "Error: %s\n", buf);
}

[[noreturn]] void throwf(const char *format, ...) {
	char buf[4096];
	va_list arglist;
	va_start(arglist, format);
	vsnprintf(buf, sizeof(buf), format, arglist);
	va_end(arglist);
	throw std::runtime_error(std::string(buf));
}

void print(const char *format, ...) {
	va_list arglist;
	va_start(arglist, format);
	vprintf(format, arglist);
	va_end(arglist);
}
