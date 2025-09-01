# Generating custom k3s_deployment_op.yaml template with wasi-nn plugin and dependencies volume mounts for different linux based OS'es using `helm`

### 1. Install helm 
```sh
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

### 2. Generate helm chart (or use the wasi-nn-chart provided)
```sh
helm create wasi-nn-chart

# the provided wasi-nn-chart has the unnecessary stuff removed so as to keep the output yaml clean
cd k3s/wasi-nn-chart
```

### 2. values-generator.sh usage

#### Why is this needed?
The yaml config for Ubuntu 22.04 running on ARM64 platform and Ubuntu 22.04 running on x86_64 platform is different due to the paths for system libs (like)
```
`/lib/aarch64-linux-gnu/libm.so.6`
`/lib/aarch64-linux-gnu/libpthread.so.0`
`/lib/aarch64-linux-gnu/libc.so.6`
`/lib/ld-linux-aarch64.so.1`
`/lib/aarch64-linux-gnu/libdl.so.2`
`/lib/aarch64-linux-gnu/libstdc++.so.6`
`/lib/aarch64-linux-gnu/libgcc-s.so.1`
```

So, for a different platform, all libs in output of 
`~/.wasmedge/plugin/libwasmedgePluginWasiNN.so`
should be mounted as files to exact same paths at which they were in host machine.

For this purpose, `values-generator.sh` is here

```sh
chmod +x values_generator.sh
./values_generator.sh # creates ./values.yaml
```

NOTE :
This script leverages 

1. `wasmedge -v`
    ```sh
    wasmedge -v
    # o/p :
    # wasmedge version 0.14.1
    #  (plugin "wasi_logging") version 0.1.0.0
    # /home/dev/.wasmedge/lib/../plugin/libwasmedgePluginWasiNN.so (plugin "wasi_nn") version 0.1.28.0
    ```
    to know the path of `.wasmedge/` directory which comprises of the  `wasi-nn plugin` itself and most of the necessary libs(`.so`s) it needs
    So, make sure to install `wasmedge` and `wasi-nn plugin` using
    ```sh
    curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml -v 0.14.1
    ```
    or other suitable method.
    otherwise the output of `wasmedge -v` might not contain this part :
    ```sh
    /home/dev/.wasmedge/lib/../plugin/libwasmedgePluginWasiNN.so (plugin "wasi_nn") version 0.1.28.0
    ```
2. `ldd`

    use of `ldd` command to know the dependencies of `libwasmedgePluginWasiNN.so`

### 2. generate final k3s_deployment_op.yaml using helm
```sh
# (exemplar) : this was run on ubuntu:22.04 on ARM64 platform
helm template wasi-nn ./ -f values.yaml > k3s_deployment_op.yaml
```