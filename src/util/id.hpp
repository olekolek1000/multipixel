#pragma once

//Strongly-typed ID
#define DECLARE_ID(X, Y)                           \
	struct X {                                       \
		Y id;                                          \
		X() = default;                                 \
		inline X(Y id) {                               \
			set(id);                                     \
		}                                              \
		inline void set(Y id) {                        \
			this->id = id;                               \
		}                                              \
		inline Y get() const {                         \
			return this->id;                             \
		}                                              \
		inline bool operator==(const X &other) const { \
			return other.id == this->id;                 \
		}                                              \
		inline bool operator!=(const X &other) const { \
			return other.id != this->id;                 \
		}                                              \
		inline bool operator<(const X &other) const {  \
			return this->id < other.id;                  \
		}                                              \
	};
