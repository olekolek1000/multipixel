var renderer;

//Size in bytes
const header_offset = 2;

const size_u8 = 1;
const size_u16 = 2;
const size_u32 = 4;
const size_u64 = 8;

const size_s8 = 1;
const size_s16 = 2;
const size_s32 = 4;
const size_s64 = 8;

const ClientCmd = {
	message: 1,			//utf-8 text
	announce: 2,			//utf-8 username
	ping: 4, //ping no args
	cursor_pos: 100, //s32 x, s32 y
	cursor_down: 101,
	cursor_up: 102,
	boundary: 103,
	chunks_received: 104, //u32 count
	tool_size: 200, //u8 size
	tool_color: 201, //u8 red, u8 green, u8 blue
	tool_type: 202,//u8 type
	undo: 203,
}

const ServerCmd = {
	message: 1,					 //utf-8 text
	your_id: 2,					 //u16 id
	kick: 3,						//utf-8 reason
	chunk_image: 100,			//complex data
	chunk_pixel_pack: 101, //complex data
	chunk_create: 110,			//s32 chunkX, s32 chunkY
	chunk_remove: 111,			//s32 chunkX, s32 chunkY
	user_create: 200,		 //u16 id, utf-8 nickname
	user_remove: 201,		 //u16 id
	user_cursor_pos: 202, //u16 id, s32 x, s32 y
}

function createMessage(command_id, command_size) {
	let buf = new ArrayBuffer(header_offset + command_size);
	let headerview = new DataView(buf, 0);
	headerview.setInt16(0, command_id);
	return buf;
}

function textToUTF8(text) {
	return new TextEncoder("utf-8").encode(text);
}

class User {
	id = 0;
	nickname = "";
	cursor_x = 0;
	cursor_y = 0;

	constructor(id, nickname) {
		this.id = id;
		this.nickname = nickname;

	}
};

var Buffer = require('buffer').Buffer;
var LZ4 = require('lz4');

class Client {
	users = [];//class User
	socket = null;
	id = -1;
	chunks_received = 0;

	constructor(multipixel, address, nickname, loaded_callback) {
		this.multipixel = multipixel;
		this.socket = new WebSocket(address);
		this.socket.binaryType = "arraybuffer";

		let c = this;
		this.socket.onopen = function (e) {
			e;
			console.log("Socket connected");
			c.socketSendAnnouncement(nickname);
			loaded_callback();

			c.socketSendBoundary();
		}

		c = this;
		this.socket.onmessage = function (e) {
			c.onmessage(e);
		}

		this.socket.onclose = function (e) {
			if (e.wasClean) {
				console.log("Socket disconnected");
			}
			else {
				console.log("Socket error: " + e.code);
			}
		}
	}

	setChatObject = function (chat) {
		this.chat = chat;
	}

	socketSendAnnouncement = function (nickname) {
		let utf8 = textToUTF8(nickname);

		let buf = createMessage(ClientCmd.announce, utf8.length);
		let buf_u8 = new Uint8Array(buf);

		for (let i = 0; i < utf8.length; i++) {
			buf_u8[i + header_offset] = utf8[i];
		}

		this.socket.send(buf);
	}

	socketSendMessage = function (text) {
		let utf8 = textToUTF8(text);
		let buf = createMessage(ClientCmd.message, utf8.length);
		let buf_u8 = new Uint8Array(buf);

		for (let i = 0; i < utf8.length; i++) {
			buf_u8[i + header_offset] = utf8[i];
		}

		this.socket.send(buf);
	}

	socketSendBrushSize = function (size) {
		let buf = createMessage(ClientCmd.tool_size, size_u8);
		let dataview = new DataView(buf, header_offset);
		dataview.setUint8(0, size);
		this.socket.send(buf);
	}

	socketSendBrushColor = function (red, green, blue) {
		let buf = createMessage(ClientCmd.tool_color, size_u8 * 3);
		let dataview = new DataView(buf, header_offset);

		dataview.setUint8(0, red);
		dataview.setUint8(1, green);
		dataview.setUint8(2, blue);

		this.socket.send(buf);
	}

	socketSendCursorPos = function (x, y) {
		let buf = createMessage(ClientCmd.cursor_pos, size_s32 * 2);
		let dataview = new DataView(buf, header_offset);
		dataview.setInt32(size_s32 * 0, x);
		dataview.setInt32(size_s32 * 1, y);
		this.socket.send(buf);
	}

	socketSendPing = function () {
		let buf = createMessage(ClientCmd.ping, 0);
		this.socket.send(buf);
	}

	socketSendChunksReceived = function () {
		let buf = createMessage(ClientCmd.chunks_received, size_u32);
		let dataview = new DataView(buf, header_offset);
		dataview.setUint32(0, this.chunks_received);
		this.socket.send(buf);
	}

	socketSendUndo = function () {
		let buf = createMessage(ClientCmd.undo, 0);
		this.socket.send(buf);
	}

	socketSendCursorDown = function () {
		let buf = createMessage(ClientCmd.cursor_down, 0);
		this.socket.send(buf);
	}

	socketSendCursorUp = function () {
		let buf = createMessage(ClientCmd.cursor_up, 0);
		this.socket.send(buf);
	}

	socketSendBoundary = function () {
		let buf = createMessage(ClientCmd.boundary, size_s32 * 4);
		let dataview = new DataView(buf, header_offset);
		let boundary = this.multipixel.map.getChunkBoundaries();
		dataview.setInt32(0, boundary.start_x);
		dataview.setInt32(4, boundary.start_y);
		dataview.setInt32(8, boundary.end_x);
		dataview.setInt32(12, boundary.end_y);
		this.socket.send(buf);
	}

	socketSendToolType = function (tool_id) {
		let buf = createMessage(ClientCmd.tool_type, size_u8 * 1);
		let dataview = new DataView(buf, header_offset);
		dataview.setUint8(0, tool_id);
		this.socket.send(buf);
	}

	onmessage = function (e) {
		let raw_data = e.data;
		let headerview = new DataView(raw_data, 0);

		function createView(offset) {
			return new DataView(raw_data, header_offset + offset);
		}

		let dataview = createView(0);

		let command = headerview.getInt16(0);

		let map = this.multipixel.map;

		switch (command) {
			case ServerCmd.message: {
				let str = new TextDecoder().decode(dataview);
				this.chat.addMessage(str);
				break;
			}
			case ServerCmd.your_id: {
				let id = dataview.getInt16(0);
				this.id = id;
				console.log("This user ID: ", id);
				this.multipixel.refreshPlayerList();
				break;
			}
			case ServerCmd.kick: {
				let str = "Kicked. Reason: " + new TextDecoder().decode(dataview);
				console.log(str);
				alert(str);
				this.socket.close();
				break;
			}
			case ServerCmd.chunk_image: {
				let offset = 0;
				let chunk_x = dataview.getInt32(offset); offset += 4;
				let chunk_y = dataview.getInt32(offset); offset += 4;
				let raw_size = dataview.getUint32(offset); offset += 4;

				let uncompressed_buffer = new Buffer(raw_size);
				let compressed_data = raw_data.slice(header_offset + offset);
				let decoded_bytes = LZ4.decodeBlock(new Buffer(compressed_data), uncompressed_buffer);
				decoded_bytes;

				let rgb_view = new DataView(uncompressed_buffer.buffer);
				let chunk = map.getChunk(chunk_x, chunk_y);
				if (chunk) {
					chunk.putImage(this.multipixel.getRenderer().getContext(), rgb_view);
				}

				break;
			}
			case ServerCmd.chunk_pixel_pack: {
				let offset = 0;

				let chunk_x = dataview.getInt32(offset); offset += 4;
				let chunk_y = dataview.getInt32(offset); offset += 4;
				let pixel_count = dataview.getUint32(offset); offset += 4;
				let raw_size = dataview.getUint32(offset); offset += 4;

				let uncompressed_buffer = Buffer.alloc(raw_size);
				let compressed_data = raw_data.slice(header_offset + offset);
				let decoded_bytes = LZ4.decodeBlock(new Buffer(compressed_data), uncompressed_buffer);
				decoded_bytes;

				let pixel_view = new DataView(uncompressed_buffer.buffer);

				//decoded_bytes;
				//console.log(compressed_size / decoded_bytes, compressed_size, decoded_bytes);//Ratio

				offset = 0;
				for (let i = 0; i < pixel_count; i++) {
					let local_x = pixel_view.getUint8(offset);
					let local_y = pixel_view.getUint8(offset + 1);
					let red = pixel_view.getUint8(offset + 2);
					let green = pixel_view.getUint8(offset + 3);
					let blue = pixel_view.getUint8(offset + 4);
					offset += 5;

					let global_x = chunk_x * chunk_size + local_x;
					let global_y = chunk_y * chunk_size + local_y;
					map.putPixel(global_x, global_y, red, green, blue);
				}

				break;
			}
			case ServerCmd.chunk_create: {
				let chunkX = dataview.getInt32(0);
				let chunkY = dataview.getInt32(4);
				this.chunks_received++;
				this.socketSendChunksReceived();
				map.createChunk(chunkX, chunkY);
				map.triggerRerender();
				break;
			}
			case ServerCmd.chunk_remove: {
				let chunkX = dataview.getInt32(0);
				let chunkY = dataview.getInt32(4);
				map.removeChunk(chunkX, chunkY);
				map.triggerRerender();
				break;
			}
			case ServerCmd.user_create: {
				let id = dataview.getUint16(0);
				let nickname = new TextDecoder().decode(new DataView(e.data, header_offset + size_u16));
				this.users[id] = new User(id, nickname);
				this.multipixel.refreshPlayerList();
				map.triggerRerender();
				break;
			}
			case ServerCmd.user_remove: {
				let id = dataview.getUint16(0);
				this.users[id] = null;
				this.multipixel.refreshPlayerList();
				map.triggerRerender();
				break;
			}
			case ServerCmd.user_cursor_pos: {
				let id = dataview.getUint16(0);
				let x = dataview.getInt32(2);
				let y = dataview.getInt32(6);

				let user = this.users[id];
				if (user) {
					user.cursor_x = x;
					user.cursor_y = y;
					map.triggerRerender();
				}
				break;
			}
			default: {
				console.log("Got unknown command " + command + " from the server.");
				break;
			}
		}
	}
};