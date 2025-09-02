
### Setup Guide for Runwasi-WasmEdge-Demo - Running LlamaEdge's llama-api-server in k3s with load-balancer

#### 1. Installing dependencies 
```sh
# apt installable
sudo apt update && sudo apt upgrade -y && sudo apt install -y llvm-14-dev liblld-14-dev software-properties-common gcc g++ asciinema containerd cmake zlib1g-dev build-essential python3 python3-dev python3-pip git clang bc jq

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source $HOME/.cargo/env
rustup target add wasm32-wasip1

# WasmEdge + WASINN plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml -v 0.14.1 # binaries and plugin in $HOME/.wasmedge

# Runwasi's containerd-shim-wasmedge-v1
cd
git clone https://github.com/containerd/runwasi.git
cd runwasi
./scripts/setup-linux.sh
make build-wasmedge
INSTALL="sudo install" LN="sudo ln -sf" make install-wasmedge

# deps - k3s installation
cd
curl -sfL https://get.k3s.io | sh - 
sudo chmod 777 /etc/rancher/k3s/k3s.yaml # hack
```

#### 2. Building image ghcr.io/second-state/llama-api-server:latest
This step builds the `ghcr.io/second-state/llama-api-server:latest` image and imports it to the k3s' containerd's local image store

> same as `Build and import demo image` from `README.md` in https://github.com/second-state/runwasi-wasmedge-demo

```sh
git clone --recurse-submodules https://github.com/second-state/runwasi-wasmedge-demo.git

cd runwasi-wasmedge-demo

# edit makefile to eliminate containerd version error
sed -i -e '/define CHECK_CONTAINERD_VERSION/,/^endef/{
s/Containerd version must be/WARNING: Containerd version should be/
/exit 1;/d
}' Makefile

# Manually removed the dependency on wasi_logging due to issue #4003.
git -C apps/llamaedge apply $PWD/disable_wasi_logging.patch

OPT_PROFILE=release RUSTFLAGS="--cfg wasmedge --cfg tokio_unstable" make apps/llamaedge/llama-api-server

# place llama-server-img in k3s' containerd local store
cd $HOME/runwasi-wasmedge-demo/apps/llamaedge/llama-api-server
oci-tar-builder --name llama-api-server \
    --repo ghcr.io/second-state \
    --tag latest \
    --module target/wasm32-wasip1/release/llama-api-server.wasm \
    -o target/wasm32-wasip1/release/img-oci.tar # Create OCI image from the WASM binary
sudo k3s ctr image import --all-platforms $HOME/runwasi-wasmedge-demo/apps/llamaedge/llama-api-server/target/wasm32-wasip1/release/img-oci.tar 
```

#### 3. Build the load-balancer-app
```sh
cd ./load-balancer
cargo build --target wasm32-wasip1 --release

oci-tar-builder --name load-balancer \
    --repo ghcr.io/second-state \
    --tag latest \
    --module target/wasm32-wasip1/release/load-balancer.wasm \
    -o target/wasm32-wasip1/release/img-oci.tar # Create OCI image from the WASM binary
sudo k3s ctr image import --all-platforms target/wasm32-wasip1/release/img-oci.tar
sudo k3s ctr images ls # verify the import
```


#### 4. Download the gguf model needed by llama-api-server
```sh 
# preferably in the home directory
sudo mkdir -p models
sudo chmod 777 models  # ensure it's readable by k3s
cd models
curl -LO https://huggingface.co/second-state/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q5_K_M.gguf
curl -LO https://huggingface.co/second-state/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q5_K_M.gguf
curl -LO https://huggingface.co/second-state/Llama-3.2-3B-Instruct-Uncensored-GGUF/resolve/main/Llama-3.2-3B-Instruct-Uncensored-Q5_K_M.gguf

```

#### 5. Apply the kubernetes configuration yaml's
```sh
cd load-balancer
kubectl apply -f load-balancer/yaml/default-services.yaml
kubectl apply -f load-balancer/yaml/load-balancer.yaml
kubectl apply -f watcher/yaml/watcher.yaml
```
> Refer `./watcher/README.md` regarding info on `watcher` and watcher.yaml

these yaml configs are for Ubuntu 22.04 running on ARM64 platform, so paths for system libs (like)

`/lib/aarch64-linux-gnu/libm.so.6`
`/lib/aarch64-linux-gnu/libpthread.so.0`
`/lib/aarch64-linux-gnu/libc.so.6`
`/lib/ld-linux-aarch64.so.1`
`/lib/aarch64-linux-gnu/libdl.so.2`
`/lib/aarch64-linux-gnu/libstdc++.so.6`
`/lib/aarch64-linux-gnu/libgcc-s.so.1`

might be different

So, for a different platform, all libs in output of 
`~/.wasmedge/plugin/libwasmedgePluginWasiNN.so`
should be mounted as files to exact same paths at which they were in host machine.

> For this purpose, `wasi-nn-chart/values-generator.sh` is there

> For `generating custom default-services-op.yaml template with wasi-nn plugin and dependencies volume mounts for your linux based OS`, refer [./helm.SETUP.md)(https://github.com/vatsalkeshav/runwasi-wasmedge-demo/helm.SETUP.md)

Some yaml's also available at hand :
```sh
./load-balancer/yaml
├── default-services-gh.yaml   # for github actions (Ubuntu, x86_64)
├── default-services.yaml      # for Ubuntu:22.04 (ARM64)
├── load-balancer.yaml         # platform independent (:
├── test-service-gh.yaml       # for github actions (Ubuntu, x86_64)
└── test-service.yaml          # for Ubuntu:22.04 (ARM64)
```