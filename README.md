# Runwasi with WasmEdge runtime demo reportriy

> For now, only llama-api-server has been added as a plugin demo. In the future, some basic tests will be included as part of regular CI testing.

> For `Runwasi with WasmEdge runtime demo in k3s`, refer [./examples](https://github.com/second-state/runwasi-wasmedge-demo/examples)

1. Manually removed the dependency on wasi_logging due to issue [#4003](https://github.com/WasmEdge/WasmEdge/issues/4003).

```bash
git -C apps/llamaedge apply $PWD/disable_wasi_logging.patch
```

2. Build and import demo image

```bash
OPT_PROFILE=release RUSTFLAGS="--cfg wasmedge --cfg tokio_unstable" make apps/llamaedge/llama-api-server
```

3. Download the WasmEdge plugin from the installer and place it, along with its dependent libraries, into containerd's LD_LIBRARY_PATH.

```bash
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml -v 0.14.1
./inject_dependencise.sh ~/.wasmedge/plugin/libwasmedgePluginWasiNN.so /opt/containerd/lib
```

4. Download LLM model

```bash
curl -LO https://huggingface.co/second-state/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q5_K_M.gguf
```

5. Run llama-api-server

> You should build and install the runwasi with wasmedge shim that supports plugins first.

```bash
sudo ctr run --rm --runtime=io.containerd.wasmedge.v1 \
  --net-host \
  --mount type=bind,src=/opt/containerd/lib,dst=/opt/containerd/lib,options=bind:ro \
  --env WASMEDGE_PLUGIN_PATH=/opt/containerd/lib \
  --mount type=bind,src=$PWD,dst=/resource,options=bind:ro \
  --env WASMEDGE_WASINN_PRELOAD=default:GGML:CPU:/resource/Llama-3.2-1B-Instruct-Q5_K_M.gguf \
  ghcr.io/second-state/llama-api-server:latest testggml llama-api-server.wasm \
  --prompt-template llama-3-chat \
  --ctx-size 4096 \
  --model-name llama-3-1b
```

---

Open another session

1. Query to api server

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
    -H 'accept:application/json' \
    -H 'Content-Type: application/json' \
    -d '{"messages":[{"role":"system", "content": "You are a helpful assistant."}, {"role":"user", "content": "Who is Robert Oppenheimer?"}], "model":"llama-3-8b"}'
```

> **Output**  
 ```{"id":"chatcmpl-f670ab3a-a578-4ed9-a56f-dc380acff882","object":"chat.completion","created":1740398087,"model":"llama-3-1b","choices":[{"index":0,"message":{"content":"Robert Oppenheimer (1904-1967) was a renowned American theoretical physicist who played a crucial role in the development of the atomic bomb during World War II. He is widely regarded as one of the most influential scientists of the 20th century.\n\nBorn in New York City, Oppenheimer grew up in a family of intellectuals and showed an early interest in mathematics and physics. He studied at Harvard University, where he earned his undergraduate degree and later worked with Leo Szilard on the development of nuclear fission theories.\n\nAfter World War I, Oppenheimer moved to the University of California, Berkeley, where he became interested in theoretical physics and began working on Einstein's theory of relativity. He then joined the faculty at Los Alamos Laboratory in New Mexico, where he led a team that developed the first nuclear reactor and made significant contributions to the Manhattan Project's development of the atomic bomb.\n\nIn 1945, Oppenheimer was appointed director of the Los Alamos Laboratory and became the chief scientist responsible for leading the team working on the atomic bomb project. However, he faced intense scrutiny and criticism from the US government due to his involvement with the project, which raised concerns about the ethics of developing a deadly weapon.\n\nIn 1945, Oppenheimer was transferred from the Los Alamos Laboratory to the United States Atomic Energy Commission (AEC), where he worked until his death in 1967. He was criticized for his perceived nuclear hawkishness and involvement with communism, which led to his security clearance being revoked due to \"national security concerns.\"\n\nDespite these controversies, Oppenheimer's contributions to physics are still widely recognized. His work on the development of quantum mechanics, thermodynamics, and statistical mechanics laid the foundation for many advances in our understanding of the universe. He was also a pioneer in the field of nuclear physics and made significant contributions to the understanding of neutron scattering.\n\nOppenheimer was awarded the Nobel Prize in Physics in 1937 for his work on beta-ray spectroscopy. He was also awarded the Enrico Fermi Award by the National Academy of Sciences in 1954.\n\nTragically, Oppenheimer's life was cut short when he died on February 18, 1967, at the age of 62, from a self-inflicted gunshot wound. However, his legacy continues to be celebrated for its significance in shaping our understanding of the universe and humanity's place within it.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":28,"completion_tokens":492,"total_tokens":520}}%```

2. Close api server

```bash
sudo ctr task rm testggml --force
```
