# Running `github.com/second-state/runwasi-wasmedge-demo` in k3s

### 1. Installing dependencies 
```sh
# apt installable
sudo apt update && sudo apt upgrade -y && sudo apt install -y llvm-14-dev liblld-14-dev software-properties-common gcc g++ asciinema containerd cmake zlib1g-dev build-essential python3 python3-dev python3-pip git clang

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source $HOME/.cargo/env
rustup target add wasm32-wasip1
exec $SHELL

# WasmEdge + WASINN plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml -v 0.14.1 # binaries and plugin in $HOME/.wasmedge
source $HOME/.bashrc

# Runwasi's containerd-shim-wasmedge-v1
cd
git clone https://github.com/containerd/runwasi.git
cd runwasi
./scripts/setup-linux.sh
make build-wasmedge
INSTALL="sudo install" LN="sudo ln -sf" make install-wasmedge
which containerd-shim-wasmedge-v1 # verify

# deps - k3s installation
cd
curl -sfL https://get.k3s.io | sh - 
sudo chmod 777 /etc/rancher/k3s/k3s.yaml # hack
```

### 2. Configuring containerd to use containerd-shim-wasmedge-v1
k3s' containerd supports wasmedge and crun as runtimes, so when we restart k3s using 

`sudo systemctl restart k3s`

it finds those binaries in `/usr/local/bin` automatically and automatically writes a config.toml to `/var/lib/rancher/k3s/agent/etc/containerd/config.toml` reflecting that it now has the ability to successfully utilize those
```sh
sudo systemctl restart k3s
```


### 3. Building image ghcr.io/second-state/llama-api-server:latest
This step builds the `ghcr.io/second-state/llama-api-server:latest` image and imports it to the k3s' containerd's local image store

> same as `Build and import demo image` from [README.md]](https://github.com/second-state/runwasi-wasmedge-demo/README.md)

```sh
# build llama-server-wasm
cd

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
sudo k3s ctr images ls # verify the import
```

### 4. Download the gguf model needed by llama-api-server
```sh 
cd
sudo mkdir -p models
sudo chmod 777 models  # ensure it's readable by k3s
cd models
curl -LO https://huggingface.co/second-state/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q5_K_M.gguf
```

### 5. Create the kubernetes configuration yaml ( ref [k3s_deployment.yaml](https://github.com/second-state/runwasi-wasmedge-demo/k3s/k3s_deployment.yaml) )
This config is for Ubuntu:25.05 running on x84_64 platform, so paths for system libs might be different.
For example, on Ubuntu 22.04 running on ARM64 platform, the paths for system libs will look like :

`/lib/aarch64-linux-gnu/libm.so.6`
`/lib/aarch64-linux-gnu/libpthread.so.0`
`/lib/aarch64-linux-gnu/libc.so.6`
`/lib/ld-linux-aarch64.so.1`
`/lib/aarch64-linux-gnu/libdl.so.2`
`/lib/aarch64-linux-gnu/libstdc++.so.6`
`/lib/aarch64-linux-gnu/libgcc-s.so.1`

So, for a different platform, all libs in output of 
`~/.wasmedge/plugin/libwasmedgePluginWasiNN.so`
should be mounted as files to exact same paths at which they were in host machine

```sh
sudo k3s kubectl apply -f k3s/k3s_deployment.yaml

#verify
sudo k3s kubectl get pods
sudo k3s kubectl describe pod llama-api-***-***
```

### 5. Query the llama-api-server
```sh
# In terminal 1:
sudo k3s kubectl port-forward svc/llama-api-server-service 8080:8080
# o/p:
# Forwarding from 127.0.0.1:8080 -> 8080
# Forwarding from [::1]:8080 -> 8080
# Handling connection for 8080

# In terminal 2:
curl -X POST http://localhost:8080/v1/chat/completions \
    -H 'accept:application/json' \
    -H 'Content-Type: application/json' \
    -d '{
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "Who is Robert Oppenheimer?"}
        ],
        "model": "llama-3-1b"
    }'
# o/p :
# {"id":"chatcmpl-10af011c-a33b-4d36-9dad-b136253e204d","object":"chat.completion","created":1752085233,"model":"llama-3-1b","choices":[{"index":0,"message":{"content":"Robert Oppenheimer (1904-1967) was an American theoretical physicist who played a key role in the development of the atomic bomb during World War II. He is widely regarded as one of the most influential scientists of the 20th century.\n\nOppenheimer was born on April 22, 1904, in New York City to Jewish parents from Russia. His early life was marked by his family's emigration to the United States after anti-Semitic violence against Jews in Germany. He received his Ph.D. in physics from Princeton University in 1927 and worked at various research institutions, including the University of California, Berkeley.\n\nIn 1933, Oppenheimer joined Los Alamos Laboratory (now Los Alamos National Laboratory) as a young physicist working on a top-secret project to develop an atomic bomb under J. Robert Oppenheimer's direction. This project was code-named \"Manhattan Project.\" In 1942, the United States dropped the first atomic bombs on Hiroshima and Nagasaki, Japan, leading to his involvement in the development of the atomic bomb.\n\nAfter the war, Oppenheimer continued his work at Los Alamos and later became the director of the Manhattan Project's secret research division. He also played a key role in shaping the post-war nuclear policy, particularly through his presidency of the Advisory Committee on Nuclear Energy (ACNE), which helped to establish the United States' nuclear energy industry.\n\nDespite his crucial contributions to the development of atomic energy and national security, Oppenheimer was haunted by the moral implications of his work. He believed that his involvement in the Manhattan Project had made him complicit in the bombings, and he struggled with personal demons throughout his life. In 1954, he was subjected to a series of \"security clearance examinations\" due to allegations of subversive activities, which damaged his reputation and led to his eventual expulsion from government service.\n\nOppenheimer's views on science were also criticized for being too nuanced and scientifically skeptical. He believed that scientists should be guided by principles of ethics rather than simply pursuing power. After his retirement in 1955, he became an advocate for disarmament and arms control.\n\nThe 1986 book \"American Prometheus: The Triumph and Tragedy of J. Robert Oppenheimer\" was a critical biography written by Kai Bird and Martin J. Sherwin that helped to redeem Oppenheimer's reputation and provide a nuanced understanding of his complex legacy.\n\nDespite the controversy surrounding him, Robert Oppenheimer remains an important figure in science history and continues to be celebrated for his contributions to physics and his role in shaping our understanding of atomic energy.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":28,"completion_tokens":534,"total_tokens":562}}
```
```sh
sudo k3s kubectl logs llama-api-server-d87d7b4dd-z24xv
# o/p:
# [2025-07-09 18:09:09.781] [info] [WASI-NN] GGML backend: LLAMA_COMMIT 2e89f76b
# [2025-07-09 18:09:09.781] [info] [WASI-NN] GGML backend: LLAMA_BUILD_NUMBER 5640
# .
# .
# .
# [2025-07-09 18:20:33.566] [info] [WASI-NN] llama.cpp: llama_perf_context_print:        eval time =       0.00 ms /   533 runs   (    0.00 ms per token,      inf tokens per second)
# [2025-07-09 18:20:33.566] [info] [WASI-NN] llama.cpp: llama_perf_context_print:       total time =  683783.35 ms /   561 tokens
```

### 6. Cleanup
```sh
cd
sudo k3s kubectl delete -f deployment.yaml
```