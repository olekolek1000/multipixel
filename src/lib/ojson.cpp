#include "ojson.hpp"
#include "lib/portable_endian.h"
#include "util/binary_reader.hpp"
#include <cmath>
#include <codecvt>
#include <cstring>
#include <ctype.h>
#include <inttypes.h>
#include <iterator>
#include <locale>
#include <math.h>
#include <stdarg.h>
#include <stdexcept>
#include <stdio.h>
#include <stdlib.h>
#include <string>
#include <utility>

namespace ojson {
	[[noreturn]] void ojson_throw(const char *format, ...) {
		char buf[4096];
		va_list arglist;
		va_start(arglist, format);
		vsnprintf(buf, sizeof(buf), format, arglist);
		va_end(arglist);
		throw std::runtime_error(std::string(buf));
	}

	void bin_write(std::vector<uint8_t> &bin, const void *data, uint32_t size) {
		auto prev_size = bin.size();
		bin.resize(bin.size() + size);
		memcpy(bin.data() + prev_size, data, size);
	}

#define INDENT_TAB_MODE

	std::string indent(bool lint, uint32_t count) {
		if(!lint) return "";
		std::string str;
#ifdef INDENT_TAB_MODE
		str.resize(count);
		memset(str.data(), '\t', count);
#else
		str.resize(count * 2);
		memset(str.data(), ' ', count * 2);
#endif
		return "\n" + str;
	}

	auto isnum(char c) -> bool {
		constexpr const char *value_chars = "0123456789.e-+";
		constexpr auto len = 14;
		for(size_t i = 0; i < len; i++) {
			if(c == value_chars[i]) return true;
		}
		return false;
	}

	inline bool isMsgPackString(const void *data, uint32_t size) {
		if(!size) return false;
		auto b = *(uint8_t *)data;
		return b == 0xd9 || b == 0xda || b == 0xdb;
	}

	inline bool isMsgPackNumber(const void *data, uint32_t size) {
		if(!size) return false;
		auto b = *(uint8_t *)data;
		return b == 0xd3 || b == 0xcb;
	}

	inline bool isMsgPackObject(const void *data, uint32_t size) {
		if(!size) return false;
		auto b = *(uint8_t *)data;
		return b == 0xde || b == 0xdf;
	}

	inline bool isMsgPackArray(const void *data, uint32_t size) {
		if(!size) return false;
		auto b = *(uint8_t *)data;
		return b == 0xdc || b == 0xdd;
	}

	inline bool isMsgPackBoolean(const void *data, uint32_t size) {
		if(!size) return false;
		auto b = *(uint8_t *)data;
		return b == 0xc3 || b == 0xc2;
	}

	inline bool isMsgPackBinary(const void *data, uint32_t size) {
		if(!size) return false;
		auto b = *(uint8_t *)data;
		return b == 0xc4 || b == 0xc5 || b == 0xc6;
	}

	inline bool isMsgPackNull(const void *data, uint32_t size) {
		if(!size) return false;
		return *(uint8_t *)data == 0xc0;
	}

	const char *getNameFromType(Type type) {
		switch(type) {
			case Type::Array: return "array";
			case Type::Boolean: return "boolean";
			case Type::Null: return "null";
			case Type::Number: return "number";
			case Type::Object: return "object";
			case Type::String: return "string";
			case Type::Binary: return "binary";
		}
		return "undefined";
	}

	Element *parseJSONInternal(const std::string_view &str) {
		uniqptr<Element> element;

		for(auto &c : str) {
			if(isspace(c)) continue;
			switch(c) {
				case '{': // Root: Object
					element.create<ojson::Object>();
					element->parseJSON(str);
					return element.release();
					break;

				case '[': // Root: Array
					element.create<ojson::Array>();
					element->parseJSON(str);
					return element.release();
					break;

				default:
					return {};
			}
		}
		return {};
	}

	std::shared_ptr<Element> parseJSONShared(const std::string_view &str) {
		return std::shared_ptr<ojson::Element>(parseJSONInternal(str));
	}

	std::shared_ptr<Element> parseJSONShared(const void *data, size_t size) {
		return parseJSONShared(std::string_view((char *)data, size));
	}

	uniqptr<Element> parseJSON(const std::string_view &str) {
		return parseJSONInternal(str);
	}

	uniqptr<Element> parseJSON(const void *data, size_t size) {
		return parseJSONInternal(std::string_view((const char *)data, size));
	}

	uniqptr<Element> parseMsgPack(const void *data, size_t size) {
		BinaryReader reader(data, size);
		return parseMsgPack(reader);
	}

	uniqptr<Element> parseMsgPack(BinaryReader &reader) {
		if(!reader.in_size)
			return {};

		uniqptr<Element> element;

		uint8_t first_byte;
		if(!reader.fetch(&first_byte))
			return {};

		if(first_byte == 0xdc || first_byte == 0xdd) {
			// Array
			element.create<ojson::Array>();
		} else if(first_byte == 0xde || first_byte == 0xdf) {
			// Object (map)
			element.create<ojson::Object>();
		} else {
			ojson_throw("Unsupported byte %02X", first_byte);
		}

		if(element) {
			auto result = element->parseMsgPack(reader);
			if(!result) return {};
		}

		return element;
	}

	// ================================ Base Element ================================
	bool Element::is(Type type) {
		return getType() == type;
	}

	String *Element::castString() {
		return is(Type::String) ? (String *)this : nullptr;
	}

	Number *Element::castNumber() {
		return is(Type::Number) ? (Number *)this : nullptr;
	}

	Object *Element::castObject() {
		return is(Type::Object) ? (Object *)this : nullptr;
	}

	Binary *Element::castBinary() {
		return is(Type::Binary) ? (Binary *)this : nullptr;
	}

	Array *Element::castArray() {
		return is(Type::Array) ? (Array *)this : nullptr;
	}

	Boolean *Element::castBoolean() {
		return is(Type::Boolean) ? (Boolean *)this : nullptr;
	}

	Null *Element::castNull() {
		return is(Type::Null) ? (Null *)this : nullptr;
	}

	// ================================ String ================================
	Type String::getType() {
		return Type::String;
	}

	// "test"
	// "something"
	std::string String::serializeJSON(bool lint, uint32_t deepness) const {
		return "\"" + str + "\"";
	}

	static void serializeMsgPackString(std::vector<uint8_t> &out_bin, const std::string &str) {
		// fixstr not implemented
		if(str.size() < 256) { // str8
			out_bin.push_back(0xd9);
			out_bin.push_back(str.size());
		} else if(str.size() < 65536) { // str16
			out_bin.push_back(0xda);
			uint16_t sizeBE = htobe16(str.size());
			bin_write(out_bin, &sizeBE, sizeof(sizeBE));
		} else { // str32
			out_bin.push_back(0xdb);
			uint32_t sizeBE = htobe32(str.size());
			bin_write(out_bin, &sizeBE, sizeof(sizeBE));
		}
		bin_write(out_bin, str.data(), str.size());
	}

	void String::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		serializeMsgPackString(out_bin, str);
	}

	std::string_view String::parseJSON(const std::string_view &str) {
		for(size_t i = 0; i < str.size(); i++) {
			char c = str[i];
			if(isspace(c)) continue;
			if(c == '"') {
				char prevchar = 0x00;
				for(size_t j = i + 1; j < str.size(); j++) {
					char cc = str[j];
					if(cc == '\"' && prevchar != '\\') { // end of string only if escape code wasn't triggered
						this->setString(std::string(str.substr(i + 1, j - i - 1)));
						return str.substr(i, j - i + 1);
					}
					prevchar = cc;
				}
			}
		}
		throw std::runtime_error("string expected");
	}

	static bool parseMsgPackString(BinaryReader &reader, std::string &str) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		uint32_t str_size;

		if(first_byte == 0xd9) { // str8
			uint8_t s_size;
			if(!reader.read(&s_size))
				return false;
			str_size = s_size;
		} else if(first_byte == 0xda) { // str16
			uint16_t sizeBE;
			if(!reader.read(&sizeBE))
				return false;
			str_size = be16toh(sizeBE);
		} else if(first_byte == 0xdb) { // str32
			uint32_t sizeBE;
			if(!reader.read(&sizeBE))
				return false;
			str_size = be32toh(sizeBE);
		} else {
			ojson_throw("msgpack string: unknown byte %02X", first_byte);
			return false;
		}

		str.resize(str_size);
		if(!reader.read(str.data(), str_size))
			return false;

		return true;
	}

	bool String::parseMsgPack(BinaryReader &reader) {
		return parseMsgPackString(reader, str);
	}

	Type Number::getType() {
		return Type::Number;
	}

	const std::string &String::get() {
		return str;
	}

	void String::setString(std::string_view str) {
		// Add backslash to "
		std::string newstr;
		char ch = 0;
		char prev_ch = 0;
		for(size_t i = 0; i < str.size(); i++) {
			prev_ch = ch;
			ch = str[i];
			if(prev_ch == '\\') {
				if(!newstr.empty()) newstr.pop_back(); // Remove backslash
				if(ch == 'n') {
					// Newline
					newstr.push_back('\n');
				} else if(ch == '/') {
					// Slash
					newstr.push_back('/');
				} else if(ch == '\"') {
					//"
					newstr.push_back('"');
				} else if(ch == 'u') {
					// Unicode character
					uint16_t wide_ch;

					auto unicode = str.substr(i, 5); // uXXXX

					int scanned = sscanf(std::string(unicode).c_str(), "u%hx", (uint16_t *)&wide_ch);

					if(scanned > 0) {
						std::wstring_convert<std::codecvt_utf8<char32_t>, char32_t> converter;
						std::string u8str = converter.to_bytes(wide_ch);
						newstr += u8str;
						// Jump forward
						i += 4;
					}
				}
			} else {
				newstr.push_back(ch);
			}
		}

		this->str = newstr;
	}

	// ================================ Binary ================================

	Type Binary::getType() {
		return Type::Binary;
	}

	std::string Binary::serializeJSON(bool lint, uint32_t deepness) const {
		return ""; // Not available in JSON
	}

	std::string_view Binary::parseJSON(const std::string_view &str) {
		return "";
	}

	static void serializeMsgPackBinary(std::vector<uint8_t> &out_bin, const void *bin_data, size_t bin_size) {
		if(bin_size < 256) { // bin8
			out_bin.push_back(0xc4);
			out_bin.push_back(bin_size);
		} else if(bin_size < 65536) { // bin16
			out_bin.push_back(0xc5);
			uint16_t sizeBE = htobe16(bin_size);
			bin_write(out_bin, &sizeBE, sizeof(sizeBE));
		} else { // bin32
			out_bin.push_back(0xc6);
			uint32_t sizeBE = htobe32(bin_size);
			bin_write(out_bin, &sizeBE, sizeof(sizeBE));
		}

		bin_write(out_bin, bin_data, bin_size);
	}

	void Binary::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		serializeMsgPackBinary(out_bin, binary.data(), binary.size());
	}

	static bool parseMsgPackBinary(BinaryReader &reader, std::vector<uint8_t> &out_data) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		uint32_t bin_size;

		if(first_byte == 0xc4) { // bin8
			uint8_t b_size;
			if(!reader.read(&b_size))
				return false;
			bin_size = b_size;
		} else if(first_byte == 0xc5) { // bin16
			uint16_t sizeBE;
			if(!reader.read(&sizeBE))
				return false;
			bin_size = be16toh(sizeBE);
		} else if(first_byte == 0xc6) { // bin32
			uint32_t sizeBE;
			if(!reader.read(&sizeBE))
				return false;
			bin_size = be32toh(sizeBE);
		} else {
			ojson_throw("msgpack binary: unknown byte %02X", first_byte);
			return false;
		}

		out_data.resize(bin_size);
		if(!reader.read(out_data.data(), bin_size))
			return false;

		return true;
	}

	bool Binary::parseMsgPack(BinaryReader &reader) {
		return parseMsgPackBinary(reader, binary);
	}

	// ================================ Number ================================
	void Number::setValue(float val) {
		floating = true;
		val_double = val;
	}

	void Number::setValue(double val) {
		floating = true;
		val_double = val;
	}

	// 42
	// 42.24
	std::string Number::serializeJSON(bool lint, uint32_t deepness) const {
		char buf[64];
		if(floating) {
			if(isnan(val_double) || isinf(val_double)) {
				snprintf(buf, sizeof(buf), "%lg", 0.0); // JSON does not allow NANs and INFs
			} else {
				snprintf(buf, sizeof(buf), "%lg", val_double);
			}
		} else {
			snprintf(buf, sizeof(buf), "%" PRId64, val_int);
		}
		return buf;
	}

	void Number::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		if(isFloating()) // float64
			out_bin.push_back(0xcb);
		else // int64
			out_bin.push_back(0xd3);

		uint64_t valBE = htobe64(val_bytes);
		bin_write(out_bin, &valBE, sizeof(valBE));
	}

	std::string_view Number::parseJSON(const std::string_view &str) {
		for(size_t i = 0; i < str.size(); i++) {
			auto &c = str[i];
			if(isspace(c)) continue;
			size_t length = 0;
			bool isfloat = false;
			for(size_t j = i; j < str.size(); j++) {
				if(str[j] == '.') isfloat = true;
				if(!isnum(str[j])) break;
				length++;
			}
			char buf[250];
			memcpy(buf, str.data() + i, length);
			buf[length] = 0x00;
			if(isfloat) {
				double n;
				sscanf(buf, "%lg", &n);
				this->setValue(n);
				return str.substr(i, length);
			} else {
				int64_t n;
				sscanf(buf, "%" PRId64, &n);
				this->setValue(n);
				return str.substr(i, length);
			}
			break;
		}
		throw std::runtime_error("number expected");
	}

	bool Number::parseMsgPack(BinaryReader &reader) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		if(first_byte == 0xd3) {
			floating = false;
		} else if(first_byte == 0xcb) {
			floating = true;
		} else {
			ojson_throw("msgpack number: unknown byte %02X", first_byte);
			return false;
		}

		uint64_t val_bytesBE;
		if(!reader.read(&val_bytesBE))
			return false;

		val_bytes = be64toh(val_bytesBE);
		return true;
	}

	void Number::setValue(uint32_t val) {
		floating = false;
		val_int = val;
	}

	void Number::setValue(int32_t val) {
		floating = false;
		val_int = val;
	}

	void Number::setValue(int64_t val) {
		floating = false;
		val_int = val;
	}

	int64_t Number::getInt() {
		if(floating)
			return val_double;
		else
			return val_int;
	}

	double Number::getFloat() {
		if(floating)
			return val_double;
		else
			return val_int;
	}

	bool Number::isFloating() {
		return floating;
	}

	bool Number::isInt() {
		return !floating;
	}

	// ================================ Object ================================
	Type Object::getType() {
		return Type::Object;
	}

	// { "name":"abcdef", "count":42 }
	// { "content":{ "color":"red", "brightness": 255 } }
	std::string Object::serializeJSON(bool lint, uint32_t deepness) const {
		std::string buf = "{";
		for(auto it = content.begin(); it != content.end();) {
			auto &name = it->first;
			auto &value = it->second;
			buf += indent(lint, deepness + 1) + "\"" + name + "\":" + value->serializeJSON(lint, deepness + 1);
			it++;
			if(it != content.end()) {
				buf += ",";
			}
		}
		buf += indent(lint, deepness) + "}";
		return buf;
	}

	void Object::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		if(content.size() < 65536) { // map16
			out_bin.push_back(0xde);
			uint16_t countBE = htobe16(content.size());
			bin_write(out_bin, &countBE, sizeof(countBE));
		} else { // map32
			out_bin.push_back(0xdf);
			uint32_t countBE = htobe32(content.size());
			bin_write(out_bin, &countBE, sizeof(countBE));
		}

		for(auto &pair : content) {
			serializeMsgPackString(out_bin, pair.first);
			pair.second->serializeMsgPack(out_bin);
		}
	}

	std::string_view Object::parseJSON(const std::string_view &str) {
		for(size_t i = 0; i < str.size(); i++) {
			auto &c = str[i];
			if(isspace(c)) continue;
			if(c == '{') {
				auto sub = str.substr(i + 1);
				bool lvalue = true;
				std::string_view name(0x0, 0);
				for(size_t j = 0; j < sub.size(); j++) {
					auto &cc = sub[j];
					if(isspace(cc)) continue;
					if(lvalue) {
						if(cc == '"') {
							auto sub2 = sub.substr(j + 1);
							// Find end of string
							char prevchar = 0x00;
							size_t length = 0;
							for(size_t k = 0; k < sub2.size(); k++) {
								auto &d = sub2[k];
								if(d == '"' && prevchar != '\\') {
									break;
								}
								prevchar = d;
								length++;
							}
							name = sub2.substr(0, length);
							j += name.size() + 1;
							lvalue = false;
						} else if(cc == '}') {
							return str.substr(0, j + 2);
						} else {
							throw std::runtime_error("\" expected");
						}
					} else { // rvalue
						if(cc == ':') {
							// Skip spaces and control characters after ':' character
							size_t spacecount = 0;
							for(size_t i = j + 1; i < sub.size(); i++) {
								auto &ch = sub[i];
								if(isspace(ch)) {
									spacecount++;
									continue;
								}
								break;
							}
							j += spacecount;

							auto sub2 = sub.substr(j + 1);
							if(sub2.empty()) throw std::runtime_error("EOF");
							auto &ch = sub2[0];

							std::string_view valstr;
							if(ch == '"') {
								valstr = this->add<ojson::String>(name).parseJSON(sub2);
							} else if(ch == '{') {
								valstr = this->add<ojson::Object>(name).parseJSON(sub2);
							} else if(ch == '[') {
								valstr = this->add<ojson::Array>(name).parseJSON(sub2);
							} else if(ch == 'f' || ch == 't') {
								valstr = this->add<ojson::Boolean>(name).parseJSON(sub2);
							} else if(ch == 'n') {
								valstr = this->add<ojson::Null>(name).parseJSON(sub2);
							} else if((ch >= '0' && ch <= '9') || ch == '-') {
								valstr = this->add<ojson::Number>(name).parseJSON(sub2);
							}
							j += valstr.size() + 1;

							// Find ',' or '}
							for(size_t k = j; k < sub.size(); k++) {
								auto &ch = sub[k];
								if(isspace(ch)) continue;
								if(ch == '}') {
									return str.substr(0, k + 2);
								}
								if(ch != ',') throw std::runtime_error(", expected");
								break;
							}
							lvalue = true;
						} else {
							throw std::runtime_error(": expected");
						}
					}
				}
			} else {
				break;
			}
		}
		throw std::runtime_error("object expected");
	}

	bool Object::parseMsgPack(BinaryReader &reader) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		uint32_t count;

		if(first_byte == 0xde) { // map16
			uint16_t countBE;
			if(!reader.read(&countBE))
				return false;
			count = be16toh(countBE);
		} else if(first_byte == 0xdf) { // map32
			uint32_t countBE;
			if(!reader.read(&countBE))
				return false;
			count = be32toh(countBE);
		} else {
			ojson_throw("msgpack object: unknown byte %02X", first_byte);
			return false;
		}

		for(uint32_t i = 0; i < count; i++) {
			std::string key;

			if(!parseMsgPackString(reader, key)) {
				ojson_throw("msgpack object: invalid key");
				return false;
			}

			auto *data_read_pos = reader.getDataAtReadPos();
			auto remaining_size = reader.getRemainingSize();

			if(isMsgPackObject(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Object>(key).parseMsgPack(reader))
					return false;
			} else if(isMsgPackArray(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Array>(key).parseMsgPack(reader))
					return false;
			} else if(isMsgPackNumber(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Number>(key).parseMsgPack(reader))
					return false;
			} else if(isMsgPackString(data_read_pos, remaining_size)) {
				if(!this->add<ojson::String>(key).parseMsgPack(reader))
					return false;
			} else if(isMsgPackBinary(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Binary>(key).parseMsgPack(reader))
					return false;
			} else if(isMsgPackBoolean(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Boolean>(key).parseMsgPack(reader))
					return false;
			} else if(isMsgPackNull(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Null>(key).parseMsgPack(reader))
					return false;
			} else {
				ojson_throw("msgpack object: invalid object");
				return false;
			}
		}

		return true;
	}

	Element *Object::get(const std::string_view &str) const {
		auto it = content.find(str);
		if(it != content.end()) {
			return it->second.get();
		} else {
			return nullptr;
		}
	}

	String *Object::getString(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castString() : nullptr;
	}

	Number *Object::getNumber(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castNumber() : nullptr;
	}

	Object *Object::getObject(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castObject() : nullptr;
	}

	Binary *Object::getBinary(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castBinary() : nullptr;
	}

	Array *Object::getArray(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castArray() : nullptr;
	}

	Boolean *Object::getBoolean(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castBoolean() : nullptr;
	}

	Null *Object::getNull(const std::string_view &str) const {
		auto *e = get(str);
		return e ? e->castNull() : nullptr;
	}

	template <typename T>
	static T &fetch(const Object *parent, const std::string_view &str) {
		auto *element = parent->get(str);
		if(!element)
			ojson_throw("No such JSON element named %s", std::string(str).c_str());
		if(!element->is(T::getTypeStatic())) {
			auto param_name = std::string(str);
			ojson_throw("Unexpected JSON type in parameter \"%s\" (wanted %s, got %s)", param_name.c_str(), getNameFromType(T::getTypeStatic()), getNameFromType(element->getType()));
		}

		return *static_cast<T *>(element);
	}

	String &Object::fetchString(const std::string_view &str) const {
		return fetch<String>(this, str);
	}

	Binary &Object::fetchBinary(const std::string_view &str) const {
		return fetch<Binary>(this, str);
	}

	Number &Object::fetchNumber(const std::string_view &str) const {
		return fetch<Number>(this, str);
	}

	Object &Object::fetchObject(const std::string_view &str) const {
		return fetch<Object>(this, str);
	}

	Array &Object::fetchArray(const std::string_view &str) const {
		return fetch<Array>(this, str);
	}

	Boolean &Object::fetchBoolean(const std::string_view &str) const {
		return fetch<Boolean>(this, str);
	}

	Null &Object::fetchNull(const std::string_view &str) const {
		return fetch<Null>(this, str);
	}

	void Object::foreach(std::function<void(const std::string_view, Element *)> callback) const {
		for(auto &i : content) {
			callback(i.first, i.second.get());
		}
	}

	void Object::foreachr(std::function<void(const std::string_view, Element *)> callback) const {
		for(auto it = content.rbegin(); it != content.rend(); it++) {
			callback(it->first, it->second.get());
		}
	}

	template <typename T>
	T &Object::add(const std::string_view &name) {
		auto it = content.find(name);
		if(it != content.end()) {
			return *((T *)it->second.get());
		}
		auto &n = content[std::string(name)];
		n = std::make_shared<T>();
		return *((T *)n.get());
	}

	void Object::move(Object &&obj) {
		this->content = std::move(obj.content);
	}

	template ojson::Array &Object::add<ojson::Array>(const std::string_view &name);
	template ojson::Boolean &Object::add<ojson::Boolean>(const std::string_view &name);
	template ojson::Null &Object::add<ojson::Null>(const std::string_view &name);
	template ojson::Number &Object::add<ojson::Number>(const std::string_view &name);
	template ojson::Object &Object::add<ojson::Object>(const std::string_view &name);
	template ojson::String &Object::add<ojson::String>(const std::string_view &name);

	// ================================ Array ================================
	Type Array::getType() {
		return Type::Array;
	}

	// [ "one", "two", "three" ]
	// [ { "name":"one", "count":42 }, "two", "three" ]
	std::string Array::serializeJSON(bool lint, uint32_t deepness) const {
		std::string buf = "[";

		for(auto it = content.begin(); it != content.end();) {
			auto *obj = it->get();
			if(lint && obj->getType() == Type::Object) {
				buf += indent(lint, deepness) + obj->serializeJSON(lint, deepness);
			} else {
				buf += obj->serializeJSON(lint, deepness);
			}
			it++;
			if(it != content.end()) {
				buf += ",";
			}
		}

		buf += indent(false, deepness) + "]";
		return buf;
	}

	void Array::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		if(content.size() < 65536) { // array16
			out_bin.push_back(0xdc);
			uint16_t sizeBE = htobe16(content.size());
			bin_write(out_bin, &sizeBE, sizeof(sizeBE));
		} else { // array32
			out_bin.push_back(0xdd);
			uint32_t sizeBE = htobe32(content.size());
			bin_write(out_bin, &sizeBE, sizeof(sizeBE));
		}
		for(auto &cell : content) {
			cell->serializeMsgPack(out_bin);
		}
	}

	std::string_view Array::parseJSON(const std::string_view &str) {
		for(size_t i = 0; i < str.size(); i++) {
			auto &c = str[i];
			if(isspace(c)) continue;
			if(c == '[') {
				auto sub = str.substr(i + 1);
				for(size_t j = 0; j < sub.size(); j++) {
					auto &ch = sub[j];
					if(isspace(ch)) continue;

					std::string_view valstr;
					auto sub2 = sub.substr(j);

					if(ch == 't' || ch == 'f') {
						valstr = this->add<ojson::Boolean>().parseJSON(sub2);
					} else if(ch == 'n') {
						valstr = this->add<ojson::Null>().parseJSON(sub2);
					} else if(ch == '{') {
						valstr = this->add<ojson::Object>().parseJSON(sub2);
					} else if(ch == '[') {
						valstr = this->add<ojson::Array>().parseJSON(sub2);
					} else if(ch == '"') {
						valstr = this->add<ojson::String>().parseJSON(sub2);
					} else if(ch == ']') {
						valstr = "";
					} else if((ch >= '0' && ch <= '9') || ch == '-') {
						valstr = this->add<ojson::Number>().parseJSON(sub2);
					} else {
						throw std::runtime_error("unknown token");
					}

					j += valstr.size();

					// Find ',' or ']
					for(size_t k = j; k < sub.size(); k++) {
						auto &ch = sub[k];
						if(isspace(ch)) continue;
						if(ch == ']') {
							return str.substr(0, k + 2);
						}
						if(ch != ',') throw std::runtime_error(", expected");
						break;
					}
				}
			}
			break;
		}
		throw std::runtime_error("[ expected");
	}

	bool Array::parseMsgPack(BinaryReader &reader) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		uint32_t count;

		if(first_byte == 0xdc) { // array16
			uint16_t countBE;
			if(!reader.read(&countBE))
				return false;
			count = be16toh(countBE);
		} else if(first_byte == 0xdd) { // array32
			uint32_t countBE;
			if(!reader.read(&countBE))
				return false;
			count = be32toh(countBE);
		} else {
			ojson_throw("msgpack array: unknown byte %02X", first_byte);
			return false;
		}

		for(uint32_t i = 0; i < count; i++) {
			auto *data_read_pos = reader.getDataAtReadPos();
			auto remaining_size = reader.getRemainingSize();

			if(isMsgPackObject(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Object>().parseMsgPack(reader))
					return false;
			} else if(isMsgPackArray(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Array>().parseMsgPack(reader))
					return false;
			} else if(isMsgPackNumber(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Number>().parseMsgPack(reader))
					return false;
			} else if(isMsgPackString(data_read_pos, remaining_size)) {
				if(!this->add<ojson::String>().parseMsgPack(reader))
					return false;
			} else if(isMsgPackBoolean(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Boolean>().parseMsgPack(reader))
					return false;
			} else if(isMsgPackNull(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Null>().parseMsgPack(reader))
					return false;
			} else if(isMsgPackBinary(data_read_pos, remaining_size)) {
				if(!this->add<ojson::Binary>().parseMsgPack(reader))
					return false;
			} else {
				ojson_throw("msgpack array: invalid object");
				return false;
			}
		}

		return true;
	}

	void Array::foreach(std::function<void(Element *)> callback) const {
		for(auto &element : content) {
			callback(element.get());
		}
	}

	void Array::foreachr(std::function<void(Element *)> callback) const {
		for(auto it = content.rbegin(); it != content.rend(); it++) {
			callback(it->get());
		}
	}

	template <typename T>
	T &Array::add() {
		return *((T *)content.emplace_back(new T).get());
	}

	template ojson::Array &Array::add<ojson::Array>();
	template ojson::Boolean &Array::add<ojson::Boolean>();
	template ojson::Null &Array::add<ojson::Null>();
	template ojson::Number &Array::add<ojson::Number>();
	template ojson::Object &Array::add<ojson::Object>();
	template ojson::String &Array::add<ojson::String>();

	// ================================ Boolean ================================
	Type Boolean::getType() {
		return Type::Boolean;
	}

	//	true
	//	false
	std::string Boolean::serializeJSON(bool lint, uint32_t deepness) const {
		return state ? "true" : "false";
	}

	void Boolean::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		out_bin.push_back(state ? 0xc3 : 0xc2);
	}

	std::string_view Boolean::parseJSON(const std::string_view &str) {
		for(size_t i = 0; i < str.size(); i++) {
			auto &c = str[i];
			if(isspace(c)) continue;
			if(str.substr(i, 5) == "false") {
				this->set(false);
				return str.substr(i, 5);
			} else if(str.substr(i, 4) == "true") {
				this->set(true);
				return str.substr(i, 4);
			} else
				break;
		}
		throw std::runtime_error("false or true expected");
	}

	bool Boolean::parseMsgPack(BinaryReader &reader) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		if(first_byte == 0xc3)
			state = true;
		else if(first_byte == 0xc2)
			state = false;
		else {
			ojson_throw("msgpack boolean: unknown byte %02X", first_byte);
			return false;
		}

		return true;
	}

	void Boolean::set(bool n) {
		this->state = n;
	}

	const bool &Boolean::get() {
		return this->state;
	}

	// ================================ Null ================================
	Type Null::getType() {
		return Type::Null;
	}

	// null
	std::string Null::serializeJSON(bool lint, uint32_t deepness) const {
		return "null";
	}

	void Null::serializeMsgPack(std::vector<uint8_t> &out_bin) {
		out_bin.push_back(0xc0);
	}

	std::string_view Null::parseJSON(const std::string_view &str) {
		for(size_t i = 0; i < str.size(); i++) {
			auto &c = str[i];
			if(isspace(c)) continue;
			auto sub = str.substr(i, 4);
			if(sub == "null") return sub;
			break;
		}
		throw std::runtime_error("null expected");
	}

	bool Null::parseMsgPack(BinaryReader &reader) {
		uint8_t first_byte;
		if(!reader.read(&first_byte))
			return false;

		if(first_byte != 0xc0) {
			ojson_throw("msgpack boolean: unknown byte");
			return false;
		}

		return true;
	}
} // namespace ojson