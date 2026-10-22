# wiggl3 (developer build) - Linux and macOS:   make          (Windows: use CMake, see source/README.md)
# Ordinary users do not need this: they download a finished file (see README.md).
#
# This is a thin wrapper around cargo. It installs nothing, never uses sudo and does not download
# anything itself (cargo fetches only the Rust libraries pinned in source/Cargo.lock).
# Run "make help" for the list of targets.

SRC        := source
CARGO      ?= cargo
CARGO_ARGS ?= --locked
BIN        := $(SRC)/target/release/wiggl3
DIST       := dist
OS         := $(shell uname -s)
ARCH       := $(shell uname -m)
# make SKIP_DOCTOR=1 skips the prerequisite check
DOCTOR     := $(if $(filter 1,$(SKIP_DOCTOR)),,doctor)

.PHONY: all build test run doctor mac-app dist clean help

all: build

help:
	@echo "wiggl3 make targets"
	@echo "  make            build the release binary ($(BIN))"
	@echo "  make test       run the unit tests"
	@echo "  make run        build, then start wiggl3"
	@echo "  make doctor     check prerequisites (installs nothing)"
	@echo "  make mac-app    macOS only: wrap the binary as $(DIST)/wiggl3.app"
	@echo "  make dist       package the result (Linux: .tar.gz with installer; Mac: .zip) plus a SHA-256 file into $(DIST)/"
	@echo "  make clean      remove build output"
	@echo "Options: CARGO_ARGS=\"--locked\" (default), SKIP_DOCTOR=1"

doctor:
	@sh $(SRC)/tools/doctor.sh

build: $(DOCTOR)
	cd $(SRC) && $(CARGO) build --release $(CARGO_ARGS)

test: $(DOCTOR)
	cd $(SRC) && $(CARGO) test $(CARGO_ARGS)

run: build
	$(BIN)

mac-app: build
ifeq ($(OS),Darwin)
	mkdir -p $(DIST)
	bash $(SRC)/tools/make_mac_app.sh $(BIN) $(DIST)/wiggl3.app
else
	@echo "mac-app only works on macOS."; exit 1
endif

dist: build
	mkdir -p $(DIST)
ifeq ($(OS),Darwin)
	bash $(SRC)/tools/make_mac_app.sh $(BIN) $(DIST)/wiggl3.app
	cd $(DIST) && rm -f wiggl3-mac-$(ARCH).zip && ditto -c -k --keepParent wiggl3.app wiggl3-mac-$(ARCH).zip \
	  && shasum -a 256 wiggl3-mac-$(ARCH).zip > wiggl3-mac-$(ARCH).zip.sha256
else
	bash $(SRC)/tools/package_linux.sh $(BIN) $(ARCH) $(DIST)
	cd $(DIST) && sha256sum wiggl3-linux-$(ARCH).tar.gz > wiggl3-linux-$(ARCH).tar.gz.sha256
endif

clean:
	cd $(SRC) && $(CARGO) clean
	rm -rf $(DIST)
