#pragma once

#include "smartptr.hpp"
#include <functional>
#include <vector>

template <typename T>
struct Dispatcher;

template <typename T = void()>
struct Listener {
	std::function<T> callback;

	Dispatcher<T> *dispatcher = nullptr;

	void disconnect() {
		if(!dispatcher) return;
		dispatcher->remove(this);
		dispatcher = nullptr;
	}

	~Listener() {
		disconnect();
	}

	explicit operator bool() const {
		return dispatcher != nullptr;
	}
};

template <typename T = void()>
struct Dispatcher {
	virtual void remove(Listener<T> *listener) = 0;
	virtual ~Dispatcher(){};
};

template <typename T = void()>
struct SingleDispatcher : Dispatcher<T> {
	Listener<T> *listener = nullptr;

	~SingleDispatcher() override {
		//Remove reference to this object
		if(listener)
			listener->dispatcher = nullptr;
	}

	void add(Listener<T> &listener, std::function<T> callback) {
		if(listener.dispatcher)
			listener.dispatcher->remove(&listener);
	}

	void remove(Listener<T> *listener) override {
		listener->dispatcher = nullptr;
		this->listener = nullptr;
	}

	void trigger() {
		listener->callback();
	}

	template <typename... Args>
	void trigger(Args... args) {
		if(listener)
			listener->callback(args...);
	}
};

template <typename T = void()>
struct MultiDispatcher : Dispatcher<T> {
	//Can be used directly, do not modify.
	std::vector<Listener<T> *> listeners;

	~MultiDispatcher() override {
		while(!listeners.empty()) {
			auto *back = listeners.back();
			//Remove reference to this object
			back->dispatcher = nullptr;
			listeners.pop_back();
		}
	}

	//Used by client
	void add(Listener<T> &listener, std::function<T> callback) {
		if(listener.dispatcher)
			listener.dispatcher->remove(&listener);

		listener.dispatcher = this;
		listener.callback = callback;
		listeners.emplace_back(&listener); //Store pointer
	}

	void remove(Listener<T> *listener) override {
		//Reverse iterator; temporary listeners are more likely to be destroyed soon
		for(auto it = listeners.rbegin(); it != listeners.rend(); it++) {
			if(*it == listener) {
				listener->dispatcher = nullptr;
				listeners.erase(std::next(it).base()); //Remove element at iterator position
				return;
			}
		}
	}

	void triggerAll() {
		//Do not use iterator here
		for(size_t i = 0; i < listeners.size(); i++) {
			listeners[i]->callback();
		}
	}

	template <typename... Args>
	void triggerAll(Args... args) {
		for(size_t i = 0; i < listeners.size(); i++) {
			listeners[i]->callback(args...);
		}
	}
};

struct DestructorCallback {
	std::function<void()> callback;

	DestructorCallback(std::function<void()> callback) {
		this->callback = callback;
	}

	DestructorCallback(const DestructorCallback &rhs) = delete;

	~DestructorCallback() {
		if(callback)
			callback();
	}
};