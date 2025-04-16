# Copyright (C) 2023 Gramine contributors
# SPDX-License-Identifier: BSD-3-Clause

ARCH_LIBDIR ?= /lib/$(shell $(CC) -dumpmachine)

SELF_EXE = target/release/bridge-worker

.PHONY: all
all: $(SELF_EXE) omni-bridge.manifest
ifeq ($(SGX),1)
all: omni-bridge.manifest.sgx omni-bridge.sig
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

omni-bridge.manifest: omni-bridge.manifest.template
	gramine-manifest \
		-Dlog_level=$(GRAMINE_LOG_LEVEL) \
		-Darch_libdir=$(ARCH_LIBDIR) \
		-Dself_exe=$(SELF_EXE) \
		$< $@

# Make on Ubuntu <= 20.04 doesn't support "Rules with Grouped Targets" (`&:`),
# see the helloworld example for details on this workaround.
omni-bridge.manifest.sgx omni-bridge.sig: sgx_sign
	@:

.INTERMEDIATE: sgx_sign
sgx_sign: omni-bridge.manifest $(SELF_EXE)
	gramine-sgx-sign \
		--manifest $< \
		--key enclave-key.pem \
		--output $<.sgx

ifeq ($(SGX),)
GRAMINE = gramine-direct
else
GRAMINE = gramine-sgx
endif

.PHONY: start-gramine-server
start-gramine-server: all
	$(GRAMINE) omni-bridge

.PHONY: clean
clean:
	$(RM) -rf *.token *.sig *.manifest.sgx *.manifest result-* OUTPUT

.PHONY: distclean
distclean: clean
	$(RM) -rf target/

.PHONY: build-docker-dev
build-docker-dev:
	docker build -f Dockerfile.dev . --tag bridge:latest

.PHONY: start-local
start-local:
	docker compose -f docker/chains.yml -f docker/deployers.yml -f docker/explorer.yml -f docker/omni-bridge.yml up --force-recreate --remove-orphans

.PHONY: start-local-e2e-test
start-local-e2e-test:
	./scripts/test-e2e-bridge.sh

.PHONY: test-repeated-block-scanning
test-repeated-block-scanning:
	./scripts/test-repeated-block-scanning.sh

.PHONY: stop-local
stop-local:
	docker compose -f docker/chains.yml -f docker/deployers.yml -f docker/explorer.yml -f docker/omni-bridge.yml down

.PHONY: build-evm-contracts
build-evm-contracts:
	cd ethereum/chainbridge-contracts && forge build

.PHONY: get-bridge-pallet-metadata
get-local-bridge-pallet-metadata:
	subxt metadata --url http://localhost:9944 --allow-insecure --pallets OmniBridge,Sudo,System > substrate/artifacts/local.scale

get-paseo-bridge-pallet-metadata:
	subxt metadata --url https://rpc.paseo-parachain.heima.network --pallets OmniBridge,Sudo,System > substrate/artifacts/paseo.scale

get-heima-bridge-pallet-metadata:
	subxt metadata --url https://rpc.heima-parachain.heima.network --pallets OmniBridge,Sudo,System > substrate/artifacts/heima.scale
