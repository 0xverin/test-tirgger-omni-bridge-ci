# Copyright (C) 2023 Gramine contributors
# SPDX-License-Identifier: BSD-3-Clause

ARCH_LIBDIR ?= /lib/$(shell $(CC) -dumpmachine)

SELF_EXE = target/release/bridge-worker

.PHONY: all
all: $(SELF_EXE) bridge.manifest
ifeq ($(SGX),1)
all: bridge.manifest.sgx bridge.sig
endif

ifeq ($(DEBUG),1)
GRAMINE_LOG_LEVEL = debug
else
GRAMINE_LOG_LEVEL = error
endif

# Note that we're compiling in release mode regardless of the DEBUG setting passed
# to Make, as compiling in debug mode results in an order of magnitude's difference in
# performance that makes testing by running a benchmark with ab painful. The primary goal
# of the DEBUG setting is to control Gramine's loglevel.
-include $(SELF_EXE).d # See also: .cargo/config.toml
$(SELF_EXE): Cargo.toml
	cargo build --release

bridge.manifest: bridge.manifest.template
	gramine-manifest \
		-Dlog_level=$(GRAMINE_LOG_LEVEL) \
		-Darch_libdir=$(ARCH_LIBDIR) \
		-Dself_exe=$(SELF_EXE) \
		$< $@

# Make on Ubuntu <= 20.04 doesn't support "Rules with Grouped Targets" (`&:`),
# see the helloworld example for details on this workaround.
bridge.manifest.sgx bridge.sig: sgx_sign
	@:

.INTERMEDIATE: sgx_sign
sgx_sign: bridge.manifest $(SELF_EXE)
	gramine-sgx-sign \
		--manifest $< \
		--output $<.sgx

ifeq ($(SGX),)
GRAMINE = gramine-direct
else
GRAMINE = gramine-sgx
endif

.PHONY: start-gramine-server
start-gramine-server: all
	$(GRAMINE) bridge

.PHONY: clean
clean:
	$(RM) -rf *.token *.sig *.manifest.sgx *.manifest result-* OUTPUT

.PHONY: distclean
distclean: clean
	$(RM) -rf target/ Cargo.lock

.PHONY: build-docker
build-docker:
	docker build . --tag bridge:latest

.PHONY: start-local
start-local:
	docker-compose up --force-recreate --build --remove-orphans

.PHONY: start-local-e2e-test
start-local-e2e-test:
	./scripts/test-e2e-bridge.sh

.PHONY: stop-local
stop-local:
	docker-compose down

.PHONY: build-evm-contracts
build-evm-contracts:
	cd ethereum/bridge-contracts && forge build

.PHONY: get-bridge-pallet-metadata
get-bridge-pallet-metadata:
	subxt metadata --url http://localhost:9944 --allow-insecure --pallets PalletBridge > substrate/artifacts/rococo-bridge.scale
