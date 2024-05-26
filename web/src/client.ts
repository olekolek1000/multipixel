import { Chat } from "./chat";
import { Multipixel } from "./multipixel";
import { CHUNK_SIZE } from "./chunk_map";

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

const size_float = 4;

const MessageType = {
	plain_text: 0,
	html: 1
}

enum ClientCmd {
	message = 1,	// utf-8 text
	announce = 2, // u8 room_name_size, utf-8 room_name, u8 nickname_size, utf-8 nickname
	ping = 4,
	cursor_pos = 100, // s32 x, s32 y
	cursor_down = 101,
	cursor_up = 102,
	boundary = 103,
	chunks_received = 104,
	preview_request = 105, // s32 previewX, s32 previewY, u8 zoom
	tool_size = 200,			 // u8 size
	tool_color = 201,			 // u8 red, u8 green, u8 blue
	tool_type = 202,			 // u8 type
	undo = 203
}

enum ServerCmd {
	message = 1,						// u8 type, utf-8 text
	your_id = 2,						// u16 id
	kick = 3,								// u16 text size, utf-8 reason
	chunk_image = 100,			// complex data
	chunk_pixel_pack = 101, // complex data
	chunk_create = 110,			// s32 chunkX, s32 chunkY
	chunk_remove = 111,			// s32 chunkX, s32 chunkY
	preview_image = 200,		// s32 previewX, s32 previewY, u8 zoom, complex data
	user_create = 1000,			// u16 id, utf-8 nickname
	user_remove = 1001,			// u16 id
	user_cursor_pos = 1002, // u16 id, s32 x, s32 y
	processing_status_text = 1100, // utf-8 text
};

function createMessage(command_id: number, command_size: number) {
	let buf = new ArrayBuffer(header_offset + command_size);
	let headerview = new DataView(buf, 0);
	headerview.setInt16(0, command_id);
	return buf;
}

function textToUTF8(text: string) {
	return new TextEncoder().encode(text);
}

export class User {
	id = 0;
	nickname = "";
	cursor_x = 0;
	cursor_y = 0;

	constructor(id: number, nickname: string) {
		this.id = id;
		this.nickname = nickname;
	}
};

import { Buffer } from "buffer";
var LZ4 = eval('require')("lz4")


export class Client {
	multipixel: Multipixel;
	users: Array<User> = [];
	socket: WebSocket | null = null;
	chunks_received = 0;
	id: number = -1;
	chat: Chat | null = null;
	connection_callback: (error_str?: string) => void;

	constructor(params: {
		multipixel: Multipixel;
		address: string;
		nickname: string;
		room_name: string;
		connection_callback: (error_str?: string) => void;
	}) {
		this.connection_callback = params.connection_callback;
		this.multipixel = params.multipixel;
		this.socket = new WebSocket(params.address);
		this.socket.binaryType = "arraybuffer";

		let c = this;
		this.socket.onopen = function (e) {
			e;
			console.log("Socket connected");
			c.socketSendAnnouncement(params.room_name, params.nickname);
			params.connection_callback(undefined);

		}
	}

	initProtocol() {
		let c = this;
		c.socketSendBoundary();

		c = this;
		this.socket!.onmessage = function (e) {
			c.onmessage(e);
		}

		this.socket!.onclose = function (e) {
			if (e.wasClean) {
				console.log("Socket disconnected");
			}
			else {
				c.connection_callback("Connection failed: " + e.code);
				console.log("Socket error: " + e.code);
			}
		}
	}

	setChatObject(chat: Chat) {
		this.chat = chat;
	}

	socketSendAnnouncement(room_name: string, nickname: string) {
		let room_name_utf8 = textToUTF8(room_name);
		let room_name_utf8_size = room_name_utf8.length;

		let nickname_utf8 = textToUTF8(nickname);
		let nickname_utf8_size = nickname_utf8.length;

		let buf = createMessage(ClientCmd.announce,
			1 + room_name_utf8_size + 1 + nickname_utf8_size);

		let buf_u8 = new Uint8Array(buf);

		let offset = header_offset;

		//Fill room name
		buf_u8[offset++] = room_name_utf8_size;
		for (let i = 0; i < room_name_utf8_size; i++) {
			buf_u8[offset++] = room_name_utf8[i];
		}

		//Fill nickname
		buf_u8[offset++] = nickname_utf8_size;
		for (let i = 0; i < nickname_utf8_size; i++) {
			buf_u8[offset++] = nickname_utf8[i];
		}

		this.socket!.send(buf);
	}

	socketSendMessage(text: any) {
		let utf8 = textToUTF8(text);
		let buf = createMessage(ClientCmd.message, utf8.length);
		let buf_u8 = new Uint8Array(buf);

		for (let i = 0; i < utf8.length; i++) {
			buf_u8[i + header_offset] = utf8[i];
		}

		this.socket!.send(buf);
	}

	socketSendBrushSize(size: number) {
		let buf = createMessage(ClientCmd.tool_size, size_u8);
		let dataview = new DataView(buf, header_offset);
		dataview.setUint8(0, size);
		this.socket!.send(buf);
	}

	socketSendBrushColor(red: number, green: number, blue: number) {
		let buf = createMessage(ClientCmd.tool_color, size_u8 * 3);
		let dataview = new DataView(buf, header_offset);

		dataview.setUint8(0, red);
		dataview.setUint8(1, green);
		dataview.setUint8(2, blue);

		this.socket!.send(buf);
	}

	socketSendCursorPos(x: number, y: number) {
		let buf = createMessage(ClientCmd.cursor_pos, size_s32 * 2);
		let dataview = new DataView(buf, header_offset);
		dataview.setInt32(size_s32 * 0, x);
		dataview.setInt32(size_s32 * 1, y);
		this.socket!.send(buf);
	}

	socketSendPing() {
		let buf = createMessage(ClientCmd.ping, 0);
		this.socket!.send(buf);
	}

	socketSendChunksReceived() {
		let buf = createMessage(ClientCmd.chunks_received, size_u32);
		let dataview = new DataView(buf, header_offset);
		dataview.setUint32(0, this.chunks_received);
		this.socket!.send(buf);
	}

	socketSendUndo() {
		let buf = createMessage(ClientCmd.undo, 0);
		this.socket!.send(buf);
	}

	socketSendCursorDown() {
		let buf = createMessage(ClientCmd.cursor_down, 0);
		this.socket!.send(buf);
	}

	socketSendCursorUp() {
		let buf = createMessage(ClientCmd.cursor_up, 0);
		this.socket!.send(buf);
	}

	socketSendBoundary() {
		let buf = createMessage(ClientCmd.boundary, size_s32 * 4 + size_float);
		let dataview = new DataView(buf, header_offset);
		let boundary = this.multipixel.map.getChunkBoundariesReal();
		let zoom = this.multipixel.map.getZoom();

		let offset = 0;
		dataview.setInt32(offset, boundary.start_x); offset += 4;
		dataview.setInt32(offset, boundary.start_y); offset += 4;
		dataview.setInt32(offset, boundary.end_x); offset += 4;
		dataview.setInt32(offset, boundary.end_y); offset += 4;
		dataview.setFloat32(offset, zoom); offset += 4;
		this.socket!.send(buf);
	}

	socketSendPreviewRequest(x: number, y: number, zoom: number) {
		let buf = createMessage(ClientCmd.preview_request, size_s32 * 2 + size_s8);
		let dataview = new DataView(buf, header_offset);
		dataview.setInt32(0, x);
		dataview.setInt32(4, y);
		dataview.setUint8(8, zoom);
		this.socket!.send(buf);
	}

	socketSendToolType(tool_id: number) {
		let buf = createMessage(ClientCmd.tool_type, size_u8 * 1);
		let dataview = new DataView(buf, header_offset);
		dataview.setUint8(0, tool_id);
		this.socket!.send(buf);
	}

	onmessage(e: MessageEvent<any>) {
		let raw_data = e.data;
		let headerview = new DataView(raw_data, 0);

		function createView(offset: number) {
			return new DataView(raw_data, header_offset + offset);
		}

		function createViewSize(offset: number, size: number) {
			return new DataView(raw_data, header_offset + offset, size);
		}

		let dataview = createView(0);

		let command = headerview.getInt16(0);

		let map = this.multipixel.map;

		switch (command) {
			case ServerCmd.message: {
				let type = dataview.getUint8(0);
				let view_str = createView(1);
				let str = new TextDecoder().decode(view_str);
				if (type == MessageType.plain_text)
					this.chat!.addMessage(str, false);
				else if (type == MessageType.html)
					this.chat!.addMessage(str, true);
				break;
			}
			case ServerCmd.your_id: {
				let id = dataview.getInt16(0);
				this.id = id;
				console.log("This user ID: ", id);
				this.multipixel.updatePlayerList();
				break;
			}
			case ServerCmd.kick: {
				let text_size = dataview.getInt16(0);
				let view_str = createViewSize(2, text_size);
				let str = "Kicked. Reason: " + new TextDecoder().decode(view_str);
				this.connection_callback(str);
				console.error(str);
				this.socket!.close();
				break;
			}
			case ServerCmd.chunk_image: {
				let offset = 0;
				let chunk_x = dataview.getInt32(offset); offset += 4;
				let chunk_y = dataview.getInt32(offset); offset += 4;
				let raw_size = dataview.getUint32(offset); offset += 4;

				let uncompressed_buffer = Buffer.alloc(raw_size);
				let compressed_data = raw_data.slice(header_offset + offset);
				let decoded_bytes = LZ4.decodeBlock(Buffer.from(compressed_data), uncompressed_buffer);
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
				let decoded_bytes = LZ4.decodeBlock(Buffer.from(compressed_data), uncompressed_buffer);
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

					let global_x = chunk_x * CHUNK_SIZE + local_x;
					let global_y = chunk_y * CHUNK_SIZE + local_y;
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
			case ServerCmd.preview_image: {
				let offset = 0;
				let previewX = dataview.getInt32(offset); offset += 4;
				let previewY = dataview.getInt32(offset); offset += 4;
				let zoom = dataview.getUint8(offset); offset += 1;

				let rgb = Buffer.alloc(CHUNK_SIZE * CHUNK_SIZE * 3);
				let compressed = raw_data.slice(header_offset + offset);
				LZ4.decodeBlock(Buffer.from(compressed), rgb);

				let preview = this.multipixel.preview_system.getOrCreateLayer(zoom).getOrCreatePreview(previewX, previewY);
				preview.setData(new Uint8Array(rgb));
				map.triggerRerender();
				break;
			}
			case ServerCmd.user_create: {
				let id = dataview.getUint16(0);
				let nickname = new TextDecoder().decode(new DataView(e.data, header_offset + size_u16));
				this.users[id] = new User(id, nickname);
				this.multipixel.updatePlayerList();
				map.triggerRerender();
				break;
			}
			case ServerCmd.user_remove: {
				let id = dataview.getUint16(0);
				delete this.users[id];
				this.multipixel.updatePlayerList();
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
			case ServerCmd.processing_status_text: {
				let status_text = new TextDecoder().decode(new DataView(e.data, header_offset));
				this.multipixel.setProcessingStatusText(status_text);
				break;
			}
			default: {
				console.log("Got unknown command " + command + " from the server.");
				break;
			}
		}
	}
};