#pragma once

#if defined(__unix__)

#	define MUTEX_PTHREAD
#	include <pthread.h>
#else

#	define MUTEX_STD
#	include <mutex>
#endif

struct Mutex {
private:
#if defined(MUTEX_STD)
	std::mutex native_mutex;
#elif defined(MUTEX_PTHREAD)
	pthread_mutex_t native_mutex;
	pthread_mutexattr_t attr;
#else
#	error not supported
#endif

public:
	Mutex() {
#if defined(MUTEX_PTHREAD)
		pthread_mutexattr_init(&attr);

#	if defined(__APPLE__)
		pthread_mutexattr_setpolicy_np(&attr, _PTHREAD_MUTEX_POLICY_FIRSTFIT);
#	endif

		pthread_mutex_init(&native_mutex, &attr);
#endif
	}

	~Mutex() {
#if defined(MUTEX_PTHREAD)
		pthread_mutex_destroy(&native_mutex);
		pthread_mutexattr_destroy(&attr);
#endif
	}

	void lock() {
#if defined(MUTEX_STD)
		native_mutex.lock();
#elif defined(MUTEX_PTHREAD)
		pthread_mutex_lock(&native_mutex);
#endif
	}

	void unlock() {
#if defined(MUTEX_STD)
		native_mutex.unlock();
#elif defined(MUTEX_PTHREAD)
		pthread_mutex_unlock(&native_mutex);
#endif
	}
};

struct LockGuard {
	Mutex *mut;

	LockGuard() {
		mut = nullptr;
	}

	LockGuard(Mutex &mut)
			: mut(&mut) {
		this->mut->lock();
	}

	~LockGuard() {
		free();
	}

	void setMutex(Mutex &mut) {
		if(this->mut)
			this->mut->unlock();
		this->mut = &mut;
		this->mut->lock();
	}

	void free() {
		if(this->mut) {
			this->mut->unlock();
			this->mut = nullptr;
		}
	}
};