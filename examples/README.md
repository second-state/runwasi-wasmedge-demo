# Runwasi with WasmEdge runtime - *More Examples*

## Overview

This is a collection of __*more*__ demos about `runwasi` with `wasmedge runtime`.
Each demo is validated through a daily CI so as to streamline and validate the stability of both runwasi and wasmedge in container and cloud environments.

> Current demos include :
>  1. *Running `LlamaEdge's llama-api-server` `standalone`*
>  2. *Running `LlamaEdge's llama-api-server` inside `k8s`*

## Demos

### 1.  *Running `LlamaEdge's llama-api-server` `standalone`*
Here, we standalone run LlamaEdge's [llama-api-server](https://github.com/LlamaEdge/LlamaEdge/tree/main/llama-api-server) using `ctr` with the help of runwasi's `wasmedge-runtime`, wasmedge's `WASI-NN plugin`

  1. `../README.md`
  2. `./.github/workflows/ci.yml`

> expanded instructions (for local machine) : https://github.com/vatsalkeshav/instructions-wasmedge-runwasi/blob/master/docs/runwasi-wasmedge-demo_outside_k3s.md

### 2. *Running `LlamaEdge's llama-api-server` inside `k8s` environment*
Same as the `1`st demo, just inside k8s environment (*k3s used here*)

  1. `./k3s-example`
  2. `./.github/workflows/k3s-ci.yml`
