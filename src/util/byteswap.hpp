#if defined(__clang__) || defined(__GNUC__)
#undef bswap_16
#define bswap_16(x) __builtin_bswap16(x)
#undef bswap_32
#define bswap_32(x) __builtin_bswap32(x)
#undef bswap_64
#define bswap_64(x) __builtin_bswap64(x)
#else
// Other compilers might not implement gcc's __builtin_* family, so for clarity
// better to make it an error.
// MSVC has alternative for it, _byteswap_*, though.
#error Not supported compiler detected.
#endif