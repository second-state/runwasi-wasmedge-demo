## Runwasi-WasmEdge-Demo - Running LlamaEdge's llama-api-server in k3s with load-balancer

This demo features `LlamaEdge's llama-api-server` (as WASM-pods) runnning different gguf models in a multi-pod environment - all managed by a load-balancer (also a WASM-pod) - assisted by a service-watcher utilizing `kube-rs` client (a regular non-WASM pod)

### How it works :
![Architecture Diagram](docs/diagrams/loadbalancer-watcher-architecture.png)

### Load-Balancer and Service-Watcher Duo

- The `service-watcher` is there for service dns resolution and syncing related service-data with the load-balancer - and it tries to mimic k8s service management using kube-rs for automated dynamic service registration/deletion/updation

    Register a new service with 
    ```yaml
    labels:
    llamaedge/target: "true"
    annotations:
    llamaedge/weight: "2" # or some other weight
    ```
    in it's `metadata` field and the service is good to recieve requests from the load-balancer

    ```yaml
    kubectl apply -f load-balancer/yaml/test-service.yaml
    # apiVersion: v1
    # kind: Service
    # metadata:
    #   name: llama-test-service
    #   labels:
    #     lb/target: "true"
    #   annotations:
    #     lb/weight: "2"
    # spec:
    #   selector:
    #     app: llama-test
    #   ports:
    #     - port: 8080
    #       targetPort: 8080
    #   type: ClusterIP
    ```
    > for more on service-watcher, refer - `./watcher/README.md`

 - The `load balancer` distributes requests to the llamaedge-servers running different models in back-end (as pods) based on the weight meant to be handled by a model - eg. a server-pod running a higher cost model has weight 1 (hence getting to handle lesser/complex prompts) and a server-pod running a lower-cost model with weight 3 gets to handle more number of requests. The backend features several llamaedge-server-pods (running different models)

### Setting up the project
See `./docs/SETUP.md`

### Exemplar usage
See `./docs/EXAMPLES.md`

### Future Work
`WASM loves more WASM`

The `service-watcher` is still run a non-WASM pod because it uses `kube-rs` and `k8s-opensapi` as dependencies which in turn depend on 

`reqwest → hyper → tokio → socket2`

all of which assume native sockets, threads, and system TLS (Refer cargo tree of `kube-rs` and `k8s-openapi` : https://github.com/vatsalkeshav/load-bal-llamaedge-demo/blob/master/watcher/cargo.md)

While WasmEdge provides forks like `tokio-wasi`, `reqwest-wasi`, `hyper-wasi`, `socket` etc. - kube-rs has hard dependencies on the native crates, so they won’t link without patching kube-rs itself.
Given this, running the watcher as a normal pod seems more practical for now but there'd be nothing better if the service-watcher would complile to `wasm32-wasip1` target (:

### Troubleshooting
The quick-start guide is pretty comprehensive, but if you're still facing issues, try referring 

https://github.com/vatsalkeshav/runwasi-wasmedge-intructions 

it's a set of instructions for setting up the environment to run this demo - raise an issue otherwise