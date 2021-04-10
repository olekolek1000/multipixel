project('Server', 'c', 'cpp', default_options : ['c_std=c11', 'cpp_std=c++17'])

src_root = './src/'
inc = [
	include_directories(src_root)
]

src = [
	src_root + 'util/types.cpp',
	src_root + 'util/timestep.cpp',
	src_root + 'main.cpp',
	src_root + 'ws_server.cpp',
	src_root + 'server.cpp',
	src_root + 'session.cpp',
	src_root + 'command.cpp',
	src_root + 'chunk.cpp',
	src_root + 'chunk_system.cpp',
]

deps = [
	dependency('threads'),
	dependency('websocketpp'),
	dependency('liblz4')
]

executable(
	'multipixel_server',
	sources: src,
	include_directories: inc,
	dependencies: deps,
	cpp_pch: meson.source_root() + '/pch.hpp'
)