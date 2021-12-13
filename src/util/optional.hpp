#pragma once

#include <cassert>

template <typename T>
struct Optional {
	T val{};
	bool has;

	Optional()
			: has(false) {
	}

	Optional(const T &value)
			: val(value),
				has(true) {
	}

	operator bool() const {
		return has;
	}

	const T &value() const {
		assert(has);
		return val;
	}

	void operator=(const T &val) {
		this->val = val;
		this->has = true;
	}

	void operator=(const Optional<T> &another) {
		val = another.val;
		has = another.has;
	}

	bool same_as(const Optional<T> &another) const {
		if(has == another.has) {
			if(val == another.val) {
				return true;
			}
		}
		return false;
	}

	bool operator==(const Optional<T> &another) const {
		return same_as(another);
	}

	T *operator->() {
		return &val;
	}

	bool has_value() const {
		return has;
	}

	void reset() {
		if(has) {
			has = false;
			val = {};
		}
	}
};