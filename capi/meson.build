project('examples', 'c')

c = meson.get_compiler('c')

inc = include_directories('.')

dirs = [
    meson.current_source_dir() + '/../target/debug',
    meson.current_source_dir() + '/../target/release',
]

extra = []

if host_machine.system() == 'windows'
    extra = [
        c.find_library('ws2_32', required: true),
        c.find_library('bcrypt', required: true),
        c.find_library('userenv', required: true),
    ]
endif

rune_dep = c.find_library('rune', dirs: dirs, required: false)

if not rune_dep.found()
    error('Could not find rune library, try: cargo build --package rune-capi')
endif

deps = [rune_dep] + extra

executable('function', 'examples/function.c', dependencies: deps, include_directories: inc)
executable('minimal', 'examples/minimal.c', dependencies: deps, include_directories: inc)
executable('type_hash', 'examples/type_hash.c', dependencies: deps, include_directories: inc)
