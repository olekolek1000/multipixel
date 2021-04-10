#pragma once

#include <cstring>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

template <typename T>
struct uniqptr {
	T *ptr = nullptr;

public:
	uniqptr(const uniqptr &rhs) = delete;
	uniqptr &operator=(const uniqptr &rhs) = delete;

	uniqptr(uniqptr &&rhs) {
		if(this == &rhs)
			return;

		reset();
		ptr = rhs.ptr;
		rhs.ptr = nullptr;
	}

	uniqptr &operator=(uniqptr &&rhs) {
		if(this == &rhs)
			return *this;

		reset();
		this->ptr = rhs.ptr;
		rhs.ptr = nullptr;
		return *this;
	}

	uniqptr(T *newobj) {
		reset();
		ptr = newobj;
	}

	uniqptr() {
	}

	template <typename... Args>
	T *create(Args... args) {
		reset();
		ptr = new T(args...);
		return ptr;
	}

	template <typename Obj, typename... Args>
	T *create(Args... args) {
		reset();
		ptr = new Obj(args...);
		return ptr;
	}

	void reset() {
		if(ptr) {
			delete ptr;
			ptr = nullptr;
		}
	}

	~uniqptr() {
		reset();
	}

	T *get() const {
		return ptr;
	}

	T &operator*() const {
		return *ptr;
	}

	T *operator->() const {
		return ptr;
	}

	void operator=(T *ptr) {
		reset();
		this->ptr = ptr;
	}

	T *release() {
		auto *released = this->ptr;
		this->ptr = nullptr;
		return released;
	}

	explicit operator bool() const {
		return ptr != nullptr;
	}
};

template <typename T, typename... Args>
uniqptr<T> makeUniq(Args... args) {
	uniqptr<T> ptr;
	ptr.create(args...);
	return ptr;
}

// Inheritance support
template <typename base, typename derived, typename... Args>
uniqptr<base> makeUniq(Args... args) {
	uniqptr<base> ptr;
	ptr.template create<derived>(args...);
	return ptr;
}

// Raw data type object
template <typename obj>
struct uniqdata {
	obj *ptr = nullptr;
	size_t _size = 0;

public:
	uniqdata(const uniqdata &) = delete;

	void move_to(uniqdata *target) {
		target->reset();
		target->ptr = this->ptr;
		target->_size = this->_size;
		this->ptr = nullptr;
		this->_size = 0;
	}

	uniqdata(uniqdata &&a)
			: ptr(a.ptr) {
		ptr = nullptr;
	}
	uniqdata(void * ptr , size_t size) 
		:ptr(ptr),_size(size){}

	size_t size() {
		return _size;
	}

	size_t size_bytes() {
		return _size * sizeof(obj);
	}

	void reset() {
		if(ptr) {
			free(ptr);
			ptr = nullptr;
		}
		_size = 0;
	}

	void resize(size_t newsize) {
		if(newsize == _size) return;
		size_t bytesize = newsize * sizeof(obj);
		ptr = (obj *)realloc((void *)ptr, bytesize);
		_size = newsize;
	}

	void create(size_t s) {
		reset();
		_size = s;
		ptr = (obj *)malloc(_size * sizeof(obj));
	}

	inline void clear() {
		reset();
	}

	uniqdata(size_t size) {
		create(size);
	}

	obj *data() {
		return ptr;
	}

	obj &operator[](size_t index) {
		return ptr[index];
	}

	explicit operator bool() {
		return ptr != nullptr;
	}

	~uniqdata() {
		reset();
	}

	uniqdata() {
	}

	bool empty() {
		return _size == 0;
	}

	void push_back(const obj &o) {
		auto prev_size = size();
		resize(prev_size + 1);
		memcpy((void *)(ptr + prev_size), (void *)&o, sizeof(obj));
	}
};