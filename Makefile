CONTAINERD_NAMESPACE ?= default
LLAMAEDGE_SERVICE = apps/llamaedge/llama-api-server

OPT_PROFILE ?= debug
RELEASE_FLAG :=
ifeq ($(OPT_PROFILE),release)
RELEASE_FLAG = --release
endif

define CHECK_RUST_TOOLS
	@command -v cargo-get >/dev/null 2>&1 || { \
		echo "cargo-get not found, installing..."; \
		cargo install cargo-get; \
	}
	@command -v oci-tar-builder >/dev/null 2>&1 || { \
		echo "oci-tar-builder not found, installing..."; \
		cargo install oci-tar-builder; \
	}
endef

define CHECK_CONTAINERD_VERSION
	@CTR_VERSION=$$(sudo ctr version | sed -n -e '/Version/ {s/.*: *//p;q;}'); \
	if ! printf '%s\n%s\n%s\n' "$$CTR_VERSION" "v1.7.7" "v1.6.25" | sort -V | tail -1 | grep -qx "$$CTR_VERSION"; then \
		echo "Containerd version must be v1.7.7+ or v1.6.25+, but detected $$CTR_VERSION"; \
		exit 1; \
	fi
endef

.PHONY: .FORCE
.FORCE:

%.wasm: .FORCE
	@PACKAGE_PATH=$(firstword $(subst /target/, ,$@)) && \
	echo "Build WASM from $$PACKAGE_PATH" && \
	cd $$PACKAGE_PATH && cargo build --target-dir ./target --target=wasm32-wasip1 $(RELEASE_FLAG)

apps/%/img-oci.tar: apps/%/*.wasm
	$(CHECK_RUST_TOOLS)
	@PACKAGE_PATH=$(firstword $(subst /target/, ,$@)) && \
	PACKAGE_NAME=$$(cd $$PACKAGE_PATH && cargo-get package.name) && \
	echo "Build OCI image from $$PACKAGE_PATH" && \
	cd $$PACKAGE_PATH && \
	oci-tar-builder --name $$PACKAGE_NAME --repo ghcr.io/second-state --tag latest --module target/wasm32-wasip1/$(OPT_PROFILE)/$$PACKAGE_NAME.wasm -o target/wasm32-wasip1/$(OPT_PROFILE)/img-oci.tar

.DEFAULT_GOAL := all
all: $(LLAMAEDGE_SERVICE)/target/wasm32-wasip1/$(OPT_PROFILE)/img-oci.tar
	$(CHECK_CONTAINERD_VERSION)
	$(foreach var,$^,\
		sudo ctr -n $(CONTAINERD_NAMESPACE) image import --all-platforms $(var);\
	)

%: %/target/wasm32-wasip1/$(OPT_PROFILE)/img-oci.tar
	$(CHECK_CONTAINERD_VERSION)
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import --all-platforms $<

.PHONY: clean

clean:
	@echo "Remove all imported OCI images from Contained."
	@sudo ctr image ls -q | grep '^ghcr.io/second-state' | xargs -n 1 sudo ctr images rm
	@echo "Remove all built WASM files."
	@find . -type d -name 'target' | xargs rm -rf
