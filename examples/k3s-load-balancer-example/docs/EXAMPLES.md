
# Examples

Port forward first (or use a NodePort service) :
```sh
sudo k3s kubectl port-forward svc/load-balancer-service 8080:8080 &
PORT_FORWARD_PID=$!

# kill later when finished
kill $PORT_FORWARD_PID
```

#### 1. Sequential requests
```sh
# send some empty requests to save resources and time
for i in {1..10}; do
    echo "=== Request $i ==="
    curl --max-time 60 -X POST http://localhost:8080/v1/chat/completions \
        -H 'Content-Type: application/json' \
        -d "{\"messages\": [{\"role\": \"user\", \"content\": \"\"}], \"model\": \"llama-3-1b\"}" \
        --silent --show-error
    echo -e "\n---\n"
    sleep 0.5
done
# o/p :
# === Request 1 ===
# {"id":"chatcmpl-8221efcf-049d-4eb8-8f99-3398a2768958","object":"chat.completion","created":1753813105,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I'm ready to help.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":8,"total_tokens":50}}
# ---

# === Request 2 ===
# {"id":"chatcmpl-f21d6c94-5df0-4602-a9ab-336d058d30c1","object":"chat.completion","created":1753813110,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I'm sorry, I can't respond to that.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":13,"total_tokens":55}}
# ---

# === Request 3 ===
# {"id":"chatcmpl-043ecc73-d212-4219-90b7-7e7b9c1d57d3","object":"chat.completion","created":1753813115,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I can't respond 3.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":9,"total_tokens":51}}
# ---

# === Request 4 ===
# {"id":"chatcmpl-ba301e5a-ae67-4d55-8fc1-dcc03397258b","object":"chat.completion","created":1753813120,"model":"llama-3-3b-high","choices":[{"index":0,"message":{"content":"I can't fulfill that request.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":9,"total_tokens":51}}
# ---

# === Request 5 ===
# {"id":"chatcmpl-bff2c2c6-007e-438e-8201-7ecaea5b6187","object":"chat.completion","created":1753813128,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I cannot provide information or guidance on illegal or harmful activities, especially those that involve non-consensual or exploitative behavior towards children. Is there anything else I can help you with?","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":38,"total_tokens":80}}
# ---

# === Request 6 ===
# {"id":"chatcmpl-10c2a134-a8bf-4344-a771-2cc7a968f747","object":"chat.completion","created":1753813137,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I cannot provide information or guidance on illegal or harmful activities, especially those that involve non-consensual or exploitative behavior towards children. Is there anything else I can help you with?","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":38,"total_tokens":80}}
# ---

# === Request 7 ===
# {"id":"chatcmpl-ef128fed-5c77-4c9c-9439-7a8eee9d73bc","object":"chat.completion","created":1753813145,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I cannot provide information or guidance on illegal or harmful activities, especially those that involve non-consensual or exploitative behavior towards children. Can I help you with something else?","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":36,"total_tokens":78}}
# ---

# === Request 8 ===
# {"id":"chatcmpl-3c01a6ec-ae57-425d-85e6-789a92e08990","object":"chat.completion","created":1753813149,"model":"llama-3-3b-high","choices":[{"index":0,"message":{"content":"I can't fulfill that request.","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":9,"total_tokens":51}}
# ---

# === Request 9 ===
# {"id":"chatcmpl-09249335-3a6f-44fc-901b-f0cf280e93c4","object":"chat.completion","created":1753813157,"model":"llama-3-3b-high","choices":[{"index":0,"message":{"content":"I cannot provide a response that includes information or guidance on illegal or harmful activities, especially those that involve children. Is there anything else I can help you with?","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":34,"total_tokens":76}}
# ---

# === Request 10 ===
# {"id":"chatcmpl-20de3377-f344-4f2e-bb2d-e5b4459693a0","object":"chat.completion","created":1753813163,"model":"llama-3-1b-low","choices":[{"index":0,"message":{"content":"I'm here to help. What's on your mind?","role":"assistant"},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":42,"completion_tokens":14,"total_tokens":56}}
# ---

# check load balancer logs - to see that the load balancing works
LB_POD=$(sudo k3s kubectl get pods -l app=load-balancer -o jsonpath='{.items[0].metadata.name}')
sudo k3s kubectl logs -f $LB_POD
# Services configured: [Service { name: "llama-low-cost-service", weight: 3 }, Service { name: "llama-high-cost-service", weight: 1 }]
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
# Selected service: llama-high-cost-service
# Connecting to: 10.43.136.132:8080
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
# Selected service: llama-high-cost-service
# Connecting to: 10.43.136.132:8080
# Selected service: llama-high-cost-service
# Connecting to: 10.43.136.132:8080
# Selected service: llama-low-cost-service
# Connecting to: 10.43.14.226:8080
```

#### 2. Concurrency requests
```sh
TEST_DIR="load_test_small_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$TEST_DIR"

for i in {1..10}; do
    (
        request_start=$(date +%s.%3N)
        echo "Request $i started at $(date +%H:%M:%S.%3N)" > "$TEST_DIR/request_${i}_log.txt"
        
        response=$(curl --max-time 300 -w "\nHTTP_CODE:%{http_code}\nTIME_TOTAL:%{time_total}\n" \
            -X POST http://localhost:8080/v1/chat/completions \
            -H 'Content-Type: application/json' \
            -d "{\"messages\": [{\"role\": \"user\", \"content\": \"Short answer: Tell me a fun fact\"}], \"model\": \"llama-3-1b\"}" \
            --silent --show-error 2>"$TEST_DIR/request_${i}_error.log")
        
        request_end=$(date +%s.%3N)
        duration=$(echo "$request_end - $request_start" | bc -l)
        
        echo "$response" | head -n -2 > "$TEST_DIR/response_${i}.json"
        
        echo "Request $i completed at $(date +%H:%M:%S.%3N)" >> "$TEST_DIR/request_${i}_log.txt"
        echo "Duration: ${duration}s" >> "$TEST_DIR/request_${i}_log.txt"
        echo "$response" | tail -n 2 >> "$TEST_DIR/request_${i}_log.txt"
    ) &
done

# check responses/logs
cd $TEST_DIR
```