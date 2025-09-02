# generating custom default-services-op.yaml template with wasi-nn plugin and dependencies volume mounts for your linux based OS using `helm`

### 1. Install helm 
```sh
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

### 2. Generate helm chart (or use the wasi-nn-chart provided)
```sh
helm create wasi-nn-chart

# the provided wasi-nn-chart has the unnecessary stuff removed to keep the output yaml clean
cd ../helm-resources/wasi-nn-chart
```

### 2. values-generator.sh usage

Why is this needed?
The yaml config for Ubuntu 22.04 running on ARM64 platform and Ubuntu 22.04 running on x86_64 platform is different due to the paths for system libs (like)

`/lib/aarch64-linux-gnu/libm.so.6`
`/lib/aarch64-linux-gnu/libpthread.so.0`
`/lib/aarch64-linux-gnu/libc.so.6`
`/lib/ld-linux-aarch64.so.1`
`/lib/aarch64-linux-gnu/libdl.so.2`
`/lib/aarch64-linux-gnu/libstdc++.so.6`
`/lib/aarch64-linux-gnu/libgcc-s.so.1`

So, for a different platform, all libs in output of 
`~/.wasmedge/plugin/libwasmedgePluginWasiNN.so`
should be mounted as files to exact same paths at which they were in host machine.

For this purpose, `values-generator.sh` is there

```sh
chmod +x values_generator.sh
./values_generator.sh # creates ./values.yaml
```

### 2. generate final k3s_deployment_op.yaml using helm
```sh
# (exemplar) : this was run on ubuntu:22.04 on ARM64 platform
helm template wasi-nn ./ -f values.yaml --show-only templates/default-services.yaml > default-services-op.yaml
helm template wasi-nn ./ -f values.yaml --show-only templates/test-service.yaml > test-service-op.yaml
```