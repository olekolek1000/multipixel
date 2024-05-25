![Splash](contrib/splash.webp)

## **Main repository located at** [GitLab website](https://gitlab.com/oo8dev/multipixel)

# MultiPixel

## **An app for multiplayer drawing on an infinite canvas.**

Server written in Rust (formerly in C++, see "legacy_cpp" branch), client written in Typescript.

This is the final C++ release of the Multipixel server. See `rust` branch for the most current development status.

# **[Try it out! (Public server)](https://multipixel.oo8.dev)**

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
- Up to 18446744073709551616 pixels ((2^32)\*(2^32)) in one room
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

- Rust compiler + functional cargo env

Build commands:

```bash
cargo run --manifest-path server/Cargo.toml
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
