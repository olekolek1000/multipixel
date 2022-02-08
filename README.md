![Splash](contrib/splash.webp)

## **Main repository located at** [GitLab website](https://gitlab.com/olekolek1000/multipixel)

# MultiPixel
## **An app for multiplayer drawing on an infinite canvas.**
### Created by olekolek1000 and KuczaRacza

Server written in C++, client written in Typescript.

# **[Try it out! (Public server)](https://multipixl.art)**


![Preview](contrib/preview.webp)

#

## Features
- Infinite canvas
- Lua Plugin support
- Multiple rooms support
- Chat
- Undo support
- Brush smoothing

Tools:
- Brush
- Bucket fill
- Gradient editor
#

## Technical features (Server)
- Full multithreading (per-session)
- Up to 65535 clients supported
- Up to 18446744073709551616 pixels ((2^32)*(2^32)) in one room
- LZ4 chunk compression
- WebSockets
- SQLite3 room storage

#

## Technical features (Client)
- Written in Typescript
- Powered by WebGL 2
- LZ4 chunk decompression
- WebSockets

# Installation

## Launching server
### Requirements:

- Anything but Windows
- Meson build system
- Compiler with full C++17 support (Clang recommended)

Required libraries: 

- [liblz4](https://github.com/lz4/lz4)
- [lua](https://www.lua.org/)
- [sqlite3](https://sqlite.org/index.html)
- [websocketpp](https://github.com/zaphoyd/websocketpp)

Build commands:
```bash
# Project configuration
meson build --buildtype=release

# Compilation
ninja -C build

# Running
./build/multipixel_server

```

## Preparing client
### Requirements:
- npm with required packages

Build commands: 
```bash
#Change directory
cd web

#Install dependencies
npm install

#Build web application
npm run build:prod
```

Built web app is located in `./web/dist` directory.

### Changing server address
1. Locate file `./web/index.ts
2. Go to end of file
3. Modify address `ws://127.0.0.1:59900` to your preference

#
### Pull requests are welcome.
