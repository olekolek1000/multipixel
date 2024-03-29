project('Server', 'c', 'cpp', default_options : ['c_std=c11', 'cpp_std=c++17'])

src_root = './src/'
inc = [
	include_directories(src_root),
	include_directories(src_root + 'lib/SQLiteCpp')
]

src = [
	src_root + 'chunk_system.cpp',
	src_root + 'chunk.cpp',
	src_root + 'command.cpp',
	src_root + 'database.cpp',
	src_root + 'lib/ojson.cpp',
	src_root + 'lib/SQLiteCpp/Backup.cpp',
	src_root + 'lib/SQLiteCpp/Column.cpp',
	src_root + 'lib/SQLiteCpp/Database.cpp',
	src_root + 'lib/SQLiteCpp/Exception.cpp',
	src_root + 'lib/SQLiteCpp/Savepoint.cpp',
	src_root + 'lib/SQLiteCpp/Statement.cpp',
	src_root + 'lib/SQLiteCpp/Transaction.cpp',
	src_root + 'main.cpp',
	src_root + 'plugin.cpp',
	src_root + 'preview_system.cpp',
	src_root + 'room.cpp',
	src_root + 'server.cpp',
	src_root + 'session.cpp',
	src_root + 'settings.cpp',
	src_root + 'util/logs.cpp',
	src_root + 'util/timestep.cpp',
	src_root + 'util/types.cpp',
	src_root + 'ws_server.cpp',
]

cc = meson.get_compiler('cpp')

deps = [
	dependency('threads'),
	dependency('websocketpp', fallback : ['websocketpp']),
	dependency('liblz4'),
	dependency('sqlite3'),
	dependency('lua')
]

if host_machine.system() == 'windows'
	deps += [cc.find_library('ws2_32'), cc.find_library('mswsock')]
endif

global_link_args = []

if get_option('buildtype') == 'release'
	add_global_arguments(['-fvisibility=hidden'], language : ['c', 'cpp'])
	add_global_arguments('-DNDEBUG', language : 'cpp')
	global_link_args += '-flto'
endif

executable(
	'multipixel_server',
	sources: src,
	include_directories: inc,
	dependencies: deps,
	link_args: global_link_args,
	cpp_pch: meson.source_root() + '/pch.hpp'
)
