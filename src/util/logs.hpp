#pragma once

void errorf(const char *format, ...);
[[noreturn]] void throwf(const char *format, ...);
void print(const char *format, ...);