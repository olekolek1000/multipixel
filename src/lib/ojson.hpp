#pragma once

#include "util/smartptr.hpp"
#include <functional>
#include <map>
#include <memory>
#include <stdint.h>
#include <string>
#include <string_view>
#include <vector>

struct BinaryReader;

namespace ojson {
	enum class Type {
		String,
		Number,
		Object,
		Array,
		Boolean,
		Binary,
		Null
	};

	const char *getNameFromType(Type type);

	class Element;
	class String;
	class Number;
	class Object;
	class Array;
	class Boolean;
	class Binary;
	class Null;

	uniqptr<Element> parseJSON(const std::string_view &str);
	uniqptr<Element> parseJSON(const void *data, size_t size);

	std::shared_ptr<Element> parseJSONShared(const std::string_view &str);
	std::shared_ptr<Element> parseJSONShared(const void *data, size_t size);

	uniqptr<Element> parseMsgPack(BinaryReader &reader);
	uniqptr<Element> parseMsgPack(const void *data, size_t size);

	class Element {
	public:
		virtual Type getType() = 0;
		virtual std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const = 0;
		virtual void serializeMsgPack(std::vector<uint8_t> &out_bin) = 0;
		bool is(Type type);
		String *castString();
		Number *castNumber();
		Object *castObject();
		Binary *castBinary();
		Array *castArray();
		Boolean *castBoolean();
		Null *castNull();
		template <typename T>
		T *cast() {
			return is(T::getTypeStatic()) ? (T *)this : nullptr;
		}
		virtual std::string_view parseJSON(const std::string_view &str) = 0;
		virtual bool parseMsgPack(BinaryReader &reader) = 0;
		virtual ~Element() {
		}
	};

	class String : public Element {
		std::string str;

	public:
		Type getType() override;
		static Type getTypeStatic() {
			return Type::String;
		}
		std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		const std::string &get();
		void setString(std::string_view str);
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;
	};

	class Binary : public Element {
	public:
		std::vector<uint8_t> binary;

		Type getType() override;
		static Type getTypeStatic() {
			return Type::Binary;
		}
		std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		const std::string &get();
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;
	};

	class Number : public Element {
		union {
			double val_double;
			int64_t val_int;
			uint64_t val_bytes;
		};

		bool floating;

	public:
		Type getType() override;
		static Type getTypeStatic() {
			return Type::Number;
		}
		std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		void setValue(float val);
		void setValue(double val);
		void setValue(int32_t val);
		void setValue(uint32_t val);
		void setValue(int64_t val);
		int64_t getInt();
		double getFloat();
		bool isFloating();
		bool isInt();
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;
	};

	class Object : public Element {
		std::map<std::string, std::shared_ptr<ojson::Element>, std::less<>> content;

	public:
		Object() = default;
		Object(Object &&rhs) = default;
		Object(const Object &rhs) = delete;

		Type getType() override;
		static Type getTypeStatic() {
			return Type::Object;
		}
		std::string serializeJSON(bool lint = false, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		template <typename T>
		T &add(const std::string_view &name);
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;

		Element *get(const std::string_view &str) const;

		/// @returns nullptr on failure
		String *getString(const std::string_view &str) const;

		/// @returns nullptr on failure
		Number *getNumber(const std::string_view &str) const;

		/// @returns nullptr on failure
		Object *getObject(const std::string_view &str) const;

		/// @returns nullptr on failure
		Binary *getBinary(const std::string_view &str) const;

		/// @returns nullptr on failure
		Array *getArray(const std::string_view &str) const;

		/// @returns nullptr on failure
		Boolean *getBoolean(const std::string_view &str) const;

		/// @returns nullptr on failure
		Null *getNull(const std::string_view &str) const;

		/// @throws std::exception on failure
		String &fetchString(const std::string_view &str) const;

		/// @throws std::exception on failure
		Binary &fetchBinary(const std::string_view &str) const;

		/// @throws std::exception on failure
		Number &fetchNumber(const std::string_view &str) const;

		/// @throws std::exception on failure
		Object &fetchObject(const std::string_view &str) const;

		/// @throws std::exception on failure
		Array &fetchArray(const std::string_view &str) const;

		/// @throws std::exception on failure
		Boolean &fetchBoolean(const std::string_view &str) const;

		/// @throws std::exception on failure
		Null &fetchNull(const std::string_view &str) const;

		void foreach(std::function<void(const std::string_view, Element *)> callback) const;
		void foreachr(std::function<void(const std::string_view, Element *)> callback) const;
		void move(Object &&obj);
	};

	class Array : public Element {
		std::vector<uniqptr<Element>> content;

	public:
		Array() = default;
		Array(Array &&rhs) = default;
		Array(const Array &rhs) = delete;

		Type getType() override;
		static Type getTypeStatic() {
			return Type::Array;
		}
		std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		template <typename T>
		T &add();
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;
		void foreach(std::function<void(Element *)> callback) const;
		void foreachr(std::function<void(Element *)> callback) const;

		template <typename T>
		T *getAt(uint32_t index) {
			if(index >= content.size())
				return nullptr;
			return content[index]->cast<T>();
		}
	};

	class Boolean : public Element {
		bool state;

	public:
		Type getType() override;
		static Type getTypeStatic() {
			return Type::Boolean;
		}
		std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		void set(bool n);
		const bool &get();
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;
	};

	class Null : public Element {
	public:
		Type getType() override;
		static Type getTypeStatic() {
			return Type::Null;
		}
		std::string serializeJSON(bool lint = true, uint32_t deepness = 0) const override;
		void serializeMsgPack(std::vector<uint8_t> &out_bin) override;
		std::string_view parseJSON(const std::string_view &str) override;
		bool parseMsgPack(BinaryReader &reader) override;
	};

	void bin_write(std::vector<uint8_t> &bin, const void *data, uint32_t size);
} // namespace ojson