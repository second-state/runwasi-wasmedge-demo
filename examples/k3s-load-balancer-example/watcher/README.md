### 1. Building
```sh
cargo build
# or build image using Dockerfile :
docker build -t vatsalkeshav/watcher:0.09 . 
# or some other name - just remember to mention it in kubernetes configuration : `/watcher/yaml/watcher.yaml`
```

### 2. Or use pre-built image
```sh
docker image pull docker.io/vatsalkeshav/watcher:0.91
# or
ctr image pull dokcer.io/vatsalkeshav/watcher:0.91
```

### 3. Deploy the service-watcher
```sh
kubectl apply -f watcher_yaml/watcher.yaml
```

### what this service-watcher does

It's main function is `service dns resolution` and `syncing related service-data with the load-balancer`.

#### Startup Phase
- Connects to the Kubernetes cluster
- Discovers all services with the `llamaedge/target: "true"` label
- Registers each matching service with the load balancer

#### Continuous Watching
- Listens for any changes to services (additions, updates, deletions)
- When a service is added or updated:
    - Retrieves service details (name, IP, port)
    - Reads the `llamaedge/weight` annotation to determine traffic allocation
    - Updates the load balancer configuration accordingly
- When a service is deleted:
    - Removes it from the load balancer's routing table

#### Health Monitoring
- **Every 60 seconds**: Verifies synchronization between Kubernetes and load balancer
- **Every 5 minutes**: Performs full reconciliation to catch any missed changes

This approach allows for zero-downtime service registration and traffic management without requiring load balancer restarts.
