# Makefile for Lua library

LUA_TOPDIR= /usr/local
LUA_INCDIR= $(LUA_TOPDIR)/include
LUA_BINDIR= $(LUA_TOPDIR)/bin
    LIBDIR= $(LUA_TOPDIR)/lib/lua/5.4
       LUA= $(LUA_BINDIR)/lua

CC= gcc -std=c99
CFLAGS= -Wall -Wextra -Wfatal-errors -Wno-deprecated-declarations -O2
MYCFLAGS= $(CFLAGS) -I$(LUA_INCDIR)

MYNAME= bn
MYLIBS= -lcrypto
MYFILE= l$(MYNAME).c
MYMOD= $(MYNAME).$(LIBEXT)

all:	so test

so:
	@$(MAKE) `uname`

test:
	$(LUA) test.lua

install:
	cp $(MYMOD) $(LIBDIR)

clean:
	rm -f *.o *.so

doc:
	@echo "$(MYNAME) library:"
	@fgrep '/**' $(MYFILE) | cut -f2 -d/ | tr -d '* ' | sort | column

Linux:
	$(CC) $(MYCFLAGS) -o $(MYMOD) -shared -fPIC $(MYFILE) $(MYLIBS)

Darwin:
	$(CC) $(MYCFLAGS) -o $(MYMOD) -bundle -undefined dynamic_lookup $(MYFILE) $(MYLIBS)

build:
	$(CC) $(MYCFLAGS) -o $(MYMOD) $(LIBFLAG) $(MYFILE) $(MYLIBS)

LIBFLAG= -shared -fPIC
LIBEXT= so

.PHONY: all so test install clean doc Linux Darwin build
