"""Build inline single-definition storage independently of the local-hit shortcut."""

from importlib import import_module

from stage import inline_first

build = import_module('build-local-hit')
build.OUT = build.HERE / 'first-definition/build'
build.SOURCE = build.OUT / 'source'
build.local_hit = inline_first


if __name__ == '__main__':
    build.main()
