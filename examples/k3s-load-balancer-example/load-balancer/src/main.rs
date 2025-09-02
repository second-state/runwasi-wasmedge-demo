use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Service {
    name: String,
    weight: u32,
    ip: String,
    port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    name: String,
    weight: u32,
    ip: String,
    port: u16,
}

#[derive(Debug, Clone)]
struct ServiceRegistry {
    services: Arc<RwLock<Vec<Service>>>,
}

impl ServiceRegistry {
    fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn register_service(&self, service: Service) {
        println!(
            "registering service: {} (weight: {}) at {}:{}",
            service.name, service.weight, service.ip, service.port
        );

        // acquire write lock (blocks other writers, allows concurrent readers)
        let mut services = self.services.write().await;

        if let Some(existing) = services.iter_mut().find(|s| s.name == service.name) {
            println!(
                "updating existing service '{}': weight {} -> {}, address {}:{} -> {}:{}",
                service.name,
                existing.weight,
                service.weight,
                existing.ip,
                existing.port,
                service.ip,
                service.port
            );
            *existing = service;
        } else {
            println!(
                "registered new service: {} (weight: {}) at {}:{}",
                service.name, service.weight, service.ip, service.port
            );
            services.push(service);
        }

        println!("total services registered: {}", services.len());
        for service in services.iter() {
            println!(
                "  - {} (weight: {}) at {}:{}",
                service.name, service.weight, service.ip, service.port
            );
        }
    }

    async fn unregister_service(&self, name: &str) -> bool {
        // acquire write lock (blocks other writers, allows concurrent readers)
        let mut services = self.services.write().await;

        let initial_len = services.len();
        services.retain(|s| s.name != name);

        let removed = services.len() < initial_len;
        if removed {
            println!("unregistered service: {}", name);
        } else {
            println!("service not found for unregistration: {}", name);
        }

        println!("total services registered: {}", services.len());
        for service in services.iter() {
            println!(
                "  - {} (weight: {}) at {}:{}",
                service.name, service.weight, service.ip, service.port
            );
        }

        removed
    }

    async fn list_services(&self) -> Vec<Service> {
        let services = self.services.read().await;
        services.clone()
    }

    async fn get_service_address(&self, service_name: &str) -> Option<String> {
        let services = self.services.read().await;
        if let Some(service) = services.iter().find(|s| s.name == service_name) {
            let address = format!("{}:{}", service.ip, service.port);
            println!(
                "resolved service '{}' to address: {}",
                service_name, address
            );
            Some(address)
        } else {
            println!("service '{}' not found in registry", service_name);
            None
        }
    }
}

fn select_service(services: &[Service]) -> Option<&Service> {
    if services.is_empty() {
        println!("no services available for selection");
        return None;
    }

    let total_weight: u32 = services.iter().map(|s| s.weight).sum();
    if total_weight == 0 {
        println!(
            "all services have zero weight, selecting first service: {}",
            services[0].name
        );
        return services.first();
    }

    let mut rng = rand::rng();
    let mut choice = rng.random_range(0..total_weight);
    let original_choice = choice;

    for service in services {
        if choice < service.weight {
            println!(
                "selected service '{}' (choice: {}/{}, weight: {})",
                service.name, original_choice, total_weight, service.weight
            );
            return Some(service);
        }
        // choice -= service.weight; // improved to :
        choice = choice.saturating_sub(service.weight);
    }

    // fallback to first service (should be rare)
    println!(
        "A rare thing has happened and none of the services got selected\nLet's fallback to the first service: {}",
        services[0].name
    );
    services.first()
}

async fn read_request(
    stream: &mut TcpStream,
    peer_addr: std::net::SocketAddr,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    let mut temp_buf = [0; 1024];

    loop {
        let bytes_read = stream.read(&mut temp_buf).await?;
        // sanity check
        if bytes_read == 0 {
            println!(
                "client {} closed the connection gracefully - zero bytes read : EOF",
                peer_addr
            );
            break;
        }

        buffer.extend_from_slice(&temp_buf[..bytes_read]);

        // break loop - if found end of header
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    // separate headers and body from http request
    let request_str = String::from_utf8_lossy(&buffer);
    let (headers, _) = request_str
        .split_once("\r\n\r\n")
        .unwrap_or((&request_str, ""));

    let body_start = headers.len() + 4;
    let body = if body_start < buffer.len() {
        buffer[body_start..].to_vec()
    } else {
        Vec::new()
    };

    println!(
        "read request from {}: headers {} bytes, body {} bytes",
        peer_addr,
        headers.len(),
        body.len()
    );
    Ok((headers.to_string(), body))
}

async fn handle_api_request(
    mut stream: TcpStream,
    registry: Arc<ServiceRegistry>,
    method: &str,
    path: &str,
    body: &[u8],
    peer_addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "handling api request from {}: {} {}",
        peer_addr, method, path
    );

    match (method, path) {
        ("POST", "/api/register") => {
            if let Ok(req) = serde_json::from_slice::<RegisterRequest>(body) {
                println!(
                    "registration request from {}: {} (weight: {}) at {}:{}",
                    peer_addr, req.name, req.weight, req.ip, req.port
                );
                let service = Service {
                    name: req.name,
                    weight: req.weight,
                    ip: req.ip,
                    port: req.port,
                };
                registry.register_service(service).await;
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\n\r\nRegistered")
                    .await?;
            } else {
                println!("invalid json in registration request from {}", peer_addr);
                stream
                    .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nInvalid JSON")
                    .await?;
            }
        }
        ("DELETE", path) if path.starts_with("/api/unregister/") => {
            let service_name = path.strip_prefix("/api/unregister/").unwrap_or("");
            println!(
                "unregistration request from {} for service: {}",
                peer_addr, service_name
            );
            if registry.unregister_service(service_name).await {
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\n\r\nUnregistered")
                    .await?;
            } else {
                stream
                    .write_all(b"HTTP/1.1 404 Not Found\r\n\r\nService not found")
                    .await?;
            }
        }
        ("GET", "/api/services") => {
            let services = registry.list_services().await;
            println!(
                "listing {} services for request from {}",
                services.len(),
                peer_addr
            );
            let json = serde_json::to_string(&services)?;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
                json
            );
            stream.write_all(response.as_bytes()).await?;
        }
        _ => {
            println!(
                "unknown api request from {}: {} {}",
                peer_addr, method, path
            );
            stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await?;
        }
    }
    Ok(())
}

async fn handle_client(
    mut stream: TcpStream,
    registry: Arc<ServiceRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    // client's address - peer address - here for logging purposes only
    let peer_addr = stream
        .peer_addr()
        .unwrap_or_else(|_| "unknown".parse().unwrap());
    println!("handling connection from {}", peer_addr);

    // read the http request to a tuple
    let (headers, body) = read_request(&mut stream, peer_addr).await?;

    let request_line = headers.lines().next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() != 3 {
        println!("invalid request line from {}: {}", peer_addr, request_line);
        stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n")
            .await?;
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];
    println!("request from {}: {} {}", peer_addr, method, path);

    // handle api requests
    if path.starts_with("/api/") {
        return handle_api_request(stream, registry, method, path, &body, peer_addr).await;
    }

    // only handle chat completions for load balancing
    if method != "POST" || path != "/v1/chat/completions" {
        println!(
            "unsupported request from {}: {} {}",
            peer_addr, method, path
        );
        stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await?;
        return Ok(());
    }

    let services = registry.list_services().await;
    println!("available services for load balancing: {}", services.len());

    let selected_service = match select_service(&services) {
        Some(service) => service,
        None => {
            println!("no services available for request from {}", peer_addr);
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\n")
                .await?;
            return Ok(());
        }
    };

    let address = match registry.get_service_address(&selected_service.name).await {
        Some(addr) => addr,
        None => {
            println!(
                "failed to resolve address for service: {}",
                selected_service.name
            );
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\n")
                .await?;
            return Ok(());
        }
    };

    println!(
        "forwarding request from {} to service '{}' at {}",
        peer_addr, selected_service.name, address
    );

    match TcpStream::connect(&address).await {
        Ok(mut backend_stream) => {
            backend_stream.write_all(headers.as_bytes()).await?;
            backend_stream.write_all(b"\r\n\r\n").await?;
            backend_stream.write_all(&body).await?;

            let bytes_copied = tokio::io::copy(&mut backend_stream, &mut stream).await?;
            println!(
                "completed request from {} via '{}' - {} bytes returned",
                peer_addr, selected_service.name, bytes_copied
            );
        }
        Err(e) => {
            println!(
                "failed to connect to service '{}' at {}: {}",
                selected_service.name, address, e
            );
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\n")
                .await?;
        }
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // `let registry = Arc::new(ServiceRegistry::new())` could be used due to wasm's single threaded nature
    // but `Arc` works well with `tokio::spawn`
    let registry = Arc::new(ServiceRegistry::new());

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to address: {}", addr));
    println!("load balancer listening on: {}", addr);

    // loop to keep listening to new connections on the tcplistener bound address
    loop {
        match listener.accept().await {
            // rust's destructuring assignment :
            // stream: The TcpStream
            // peer_addr: The SocketAddr
            Ok((stream, peer_addr)) => {
                println!("accepted connection from: {}", peer_addr);
                let registry_clone = registry.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream, registry_clone).await {
                        println!("error handling client {}: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                eprintln!("failed to accept connection: {}", e);
            }
        }
    }
}
