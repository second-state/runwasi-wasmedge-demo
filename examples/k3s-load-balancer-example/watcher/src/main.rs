use futures::StreamExt;
use k8s_openapi::api::core::v1::Service; // kubernetes service type
use kube::{api::ListParams, runtime::watcher, Api, Client, ResourceExt};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::lookup_host;
use tokio::time::{interval, Duration};

#[derive(Serialize, Debug)]
struct RegisterPayload {
    name: String, 
    weight: u32,  
    ip: String,   
    port: u16,   
}

#[derive(Deserialize, Debug, Clone)]
struct RegisteredService {
    name: String,
    weight: u32,
    ip: String,
    port: u16,
}

async fn register_service(
    svc: &Service,
    http: &HttpClient,
    context: &str, // ,ie. startup, reconciliation, or event
) -> anyhow::Result<()> {
    let name = svc.name_any();
    let namespace = svc.namespace().unwrap_or("default".to_string());
    println!("processing {} service: {}/{}", context, namespace, name);

    // get annotations
    let annotations = svc.metadata.annotations.clone().unwrap_or_default();
    if context == "event" {
        println!("service annotations: {:?}", annotations);
    }

    // get weight from annotation
    let weight = annotations
        .get("llamaedge/weight")
        .and_then(|w| w.parse::<u32>().ok())
        .unwrap_or(1);

    if annotations.contains_key("llamaedge/weight") {
        println!("weight found in annotations: {}", weight);
    } else {
        println!("no weight annotation found, using default: {}", weight);
    }

    // get service port
    let mut service_port = 8080u16; // default port
    if let Some(spec) = &svc.spec {
        if let Some(ports) = &spec.ports {
            if context == "event" {
                println!(
                    "service ports: {:?}",
                    ports
                        .iter()
                        .map(|p| format!(
                            "{}:{}",
                            p.name.as_ref().unwrap_or(&"unnamed".to_string()),
                            p.port
                        ))
                        .collect::<Vec<_>>()
                );
            }

            // use the first port if available
            if let Some(first_port) = ports.first() {
                service_port = first_port.port as u16;
                println!("using port {} for DNS resolution", service_port);
            }
        }
        if let Some(cluster_ip) = &spec.cluster_ip {
            if context == "event" {
                println!("service cluster ip: {}", cluster_ip);
            }
        }
    }

    // perform DNS resolution
    let hostname = format!("{}.{}.svc.cluster.local:{}", name, namespace, service_port);
    println!("performing DNS lookup for {}: {}", context, hostname);

    let lookup_result = lookup_host(hostname.clone()).await;
    match lookup_result {
        Ok(mut addrs) => {
            if let Some(first_addr) = addrs.next() {
                let ip = first_addr.ip().to_string();
                let port = first_addr.port();
                println!("DNS resolution successful: {}:{}", ip, port);

                // create payload for registration
                let payload = RegisterPayload {
                    name: name.clone(),
                    weight,
                    ip,
                    port,
                };
                println!("preparing {} payload: {:?}", context, payload);
                
                if context == "event" {
                    println!("payload being sent: {:?}", serde_json::to_string(&payload)?);
                }

                // send POST request
                let lb_url = "http://load-balancer-service.default.svc.cluster.local:8080/api/register";
                println!("sending {} registration request to: {}", context, lb_url);

                let res = http.post(lb_url).json(&payload).send().await;

                match res {
                    Ok(resp) => {
                        let status = resp.status();
                        println!(
                            "{} registration successful for {}/{}: http {}",
                            context, namespace, name, status
                        );

                        // log response body if available (only for events to reduce noise)
                        if context == "event" {
                            if let Ok(body) = resp.text().await {
                                if !body.is_empty() {
                                    println!("response body: {}", body);
                                }
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "{} registration failed for {}/{}: {}",
                            context, namespace, name, err
                        );
                        if context == "event" {
                            eprintln!("check if lb is running at: {}", lb_url);
                        }
                    }
                }
            } else {
                eprintln!("DNS resolution returned no addresses for {}: {}", context, hostname);
            }
        }
        Err(err) => {
            eprintln!("DNS resolution failed for {} {}: {}", context, hostname, err);
            if context == "event" {
                eprintln!("check if the service exists and is accessible");
            }
        }
    }

    Ok(())
}

// get current services with the target label
async fn get_services(
    services: &Api<Service>,
    lp: &ListParams,
) -> anyhow::Result<Vec<Service>> {
    match services.list(lp).await {
        Ok(service_list) => {
            println!("found {} services with label llamaedge/target=true", 
                    service_list.items.len());
            Ok(service_list.items)
        }
        Err(err) => {
            eprintln!("failed to list services: {}", err);
            Ok(Vec::new())
        }
    }
}

// extract service info from service
async fn extract_service_info(svc: &Service) -> Option<(String, u32, String, u16)> {
    let name = svc.name_any();
    let namespace = svc.namespace().unwrap_or("default".to_string());
    
    // get weight from annotation
    let annotations = svc.metadata.annotations.clone().unwrap_or_default();
    let weight = annotations
        .get("llamaedge/weight")
        .and_then(|w| w.parse::<u32>().ok())
        .unwrap_or(1);

    // get service port
    let mut service_port = 8080u16;
    if let Some(spec) = &svc.spec {
        if let Some(ports) = &spec.ports {
            if let Some(first_port) = ports.first() {
                service_port = first_port.port as u16;
            }
        }
    }

    // perform DNS resolution to get IP
    let hostname = format!("{}.{}.svc.cluster.local:{}", name, namespace, service_port);
    match lookup_host(hostname).await {
        Ok(mut addrs) => {
            if let Some(first_addr) = addrs.next() {
                let ip = first_addr.ip().to_string();
                let port = first_addr.port();
                Some((name, weight, ip, port))
            } else {
                eprintln!("DNS resolution returned no addresses for: {}", name);
                None
            }
        }
        Err(err) => {
            eprintln!("DNS resolution failed for {}: {}", name, err);
            None
        }
    }
}

// register a service using payload
async fn register_service_payload(payload: &RegisterPayload, http: &HttpClient) -> anyhow::Result<()> {
    let lb_url = "http://load-balancer-service.default.svc.cluster.local:8080/api/register";
    
    let res = http.post(lb_url).json(payload).send().await?;
    
    if res.status().is_success() {
        println!("successfully registered/updated service: {}", payload.name);
    } else {
        eprintln!("failed to register service {}: http {}", payload.name, res.status());
    }
    
    Ok(())
}

// name-based service sync with lb
async fn sync_services_with_load_balancer(
    services: &Api<Service>,
    lp: &ListParams,
    http: &HttpClient,
    context: &str,
) -> anyhow::Result<()> {
    println!("starting service synchronization with lb ({})", context);
    
    // get current state from both sources
    let k8s_services = get_services(services, lp).await?;
    let lb_services = get_registered_services(http).await?;
    
    // convert to maps for easier comparison
    let mut k8s_service_map: HashMap<String, (u32, String, u16)> = HashMap::new();
    
    // extract info from services
    for svc in &k8s_services {
        if let Some((name, weight, ip, port)) = extract_service_info(svc).await {
            k8s_service_map.insert(name, (weight, ip, port));
        }
    }
    
    let mut lb_service_map: HashMap<String, RegisteredService> = HashMap::new();
    for svc in lb_services {
        lb_service_map.insert(svc.name.clone(), svc);
    }
    
    println!("comparison: {} K8s services vs {} LB services", 
            k8s_service_map.len(), lb_service_map.len());
    
    // 1. handle services that exist in K8s but not in LB (need to register)
    for (k8s_name, (weight, ip, port)) in &k8s_service_map {
        if !lb_service_map.contains_key(k8s_name) {
            println!("service {} exists in K8s but not in LB - registering", k8s_name);
            
            let payload = RegisterPayload {
                name: k8s_name.clone(),
                weight: *weight,
                ip: ip.clone(),
                port: *port,
            };
            
            if let Err(err) = register_service_payload(&payload, http).await {
                eprintln!("failed to register missing service {}: {}", k8s_name, err);
            }
        }
    }
    
    // 2. handle services that exist in LB but not in K8s (stale, need to remove)
    for (lb_name, _) in &lb_service_map {
        if !k8s_service_map.contains_key(lb_name) {
            println!("service {} exists in LB but not in K8s - removing stale registration", lb_name);
            
            let unregister_url = format!(
                "http://load-balancer-service.default.svc.cluster.local:8080/api/unregister/{}",
                lb_name
            );
            
            match http.delete(&unregister_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        println!("successfully removed stale service: {}", lb_name);
                    } else {
                        eprintln!("failed to remove stale service {}: http {}", lb_name, resp.status());
                    }
                }
                Err(err) => {
                    eprintln!("error removing stale service {}: {}", lb_name, err);
                }
            }
        }
    }
    
    // 3. handle services that exist in both but might have different details (need to update)
    for (k8s_name, (k8s_weight, k8s_ip, k8s_port)) in &k8s_service_map {
        if let Some(lb_service) = lb_service_map.get(k8s_name) {
            // compare details to see if update is needed
            let needs_update = lb_service.weight != *k8s_weight 
                            || lb_service.ip != *k8s_ip 
                            || lb_service.port != *k8s_port;
                            
            if needs_update {
                println!("service {} details changed - updating registration", k8s_name);
                println!("old: weight={}, ip={}, port={}", 
                        lb_service.weight, lb_service.ip, lb_service.port);
                println!("new: weight={}, ip={}, port={}", 
                        k8s_weight, k8s_ip, k8s_port);
                
                let payload = RegisterPayload {
                    name: k8s_name.clone(),
                    weight: *k8s_weight,
                    ip: k8s_ip.clone(),
                    port: *k8s_port,
                };
                
                if let Err(err) = register_service_payload(&payload, http).await {
                    eprintln!("failed to update service {}: {}", k8s_name, err);
                }
            }
        }
    }
    
    println!("service sunc completed");
    Ok(())
}

// get currently registered services from lb
async fn get_registered_services(http: &HttpClient) -> anyhow::Result<Vec<RegisteredService>> {
    let lb_url = "http://load-balancer-service.default.svc.cluster.local:8080/api/services";
    println!("fetching currently registered services from: {}", lb_url);
    
    let res = http.get(lb_url).send().await?;
    
    if res.status().is_success() {
        let services: Vec<RegisteredService> = res.json().await?;
        println!("lb has {} registered services", services.len());
        Ok(services)
    } else {
        let status = res.status();
        eprintln!("failed to fetch registered services: http {}", status);
        Ok(Vec::new()) // return empty vec on error to continue op
    }
}

// reconciliation function to sync all services
async fn reconcile_services(
    services: &Api<Service>,
    lp: &ListParams,
    http: &HttpClient,
) -> anyhow::Result<()> {
    println!("starting periodic reconciliation of services...");
    
    match services.list(lp).await {
        Ok(service_list) => {
            println!("reconciliation found {} services with label llamaedge/target=true", 
                    service_list.items.len());
            
            if service_list.items.is_empty() {
                println!("no services found during reconciliation");
                return Ok(());
            }
            
            for svc in service_list.items {
                if let Err(err) = register_service(&svc, http, "reconciliation").await {
                    eprintln!("reconciliation failed for service: {}", err);
                }
            }
            
            println!("reconciliation completed successfully");
        }
        Err(err) => {
            eprintln!("reconciliation failed to list services: {}", err);
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("starting service watcher for llamaedge lb");

    // create k8s client
    println!("connecting to cluster...");
    let k8s_client = Client::try_default().await?;
    println!("successfully connected to cluster");

    // API interface for Services in all namespaces
    let services: Api<Service> = Api::all(k8s_client);
    println!("configured to watch services across all namespaces");

    // create HTTP client
    let http = HttpClient::new();
    println!("HTTP client initialized for lb communication");

    // only watch Services with label "llamaedge/target=true"
    let lp = ListParams::default().labels("llamaedge/target=true");
    println!("label selector configured: llamaedge/target=true");

    // discover and register existing services
    println!("discovering existing services with matching labels...");

    match services.list(&lp).await {
        Ok(service_list) => {
            println!(
                "found {} existing services with label llamaedge/target=true",
                service_list.items.len()
            );

            if service_list.items.is_empty() {
                println!("no existing services found to register");
            } else {
                for svc in service_list.items {
                    if let Err(err) = register_service(&svc, &http, "startup").await {
                        eprintln!("startup registration failed: {}", err);
                    }
                }
            }

            println!("finished processing existing services");
        }
        Err(err) => {
            eprintln!("failed to discover existing services: {}", err);
            eprintln!("continuing with watcher anyway...");
        }
    }

    // configure the watcher with label selector
    let watcher_config = watcher::Config::default().labels(&lp.label_selector.clone().unwrap_or_default());

    // start watching Services with config
    let mut watcher_stream = watcher(services.clone(), watcher_config).boxed();
    println!("starting to watch services with label llamaedge/target=true");

    // set up periodic reconciliation and service sync
    let mut reconcile_timer = interval(Duration::from_secs(300)); // every 5 minutes
    let mut sync_timer = interval(Duration::from_secs(60)); // every 60 seconds
    println!("periodic reconciliation configured: every 5 minutes");
    println!("service sync configured: every 60 seconds");
    println!("waiting for service events...");

    loop {
        tokio::select! {
            // handle reconciliation timer
            _ = reconcile_timer.tick() => {
                if let Err(err) = reconcile_services(&services, &lp, &http).await {
                    eprintln!("reconciliation error: {}", err);
                }
                
                // sync after reconciliation
                if let Err(err) = sync_services_with_load_balancer(&services, &lp, &http, "post-reconciliation").await {
                    eprintln!("post-reconciliation sync error: {}", err);
                }
            }
            
            // handle service sync timer
            _ = sync_timer.tick() => {
                if let Err(err) = sync_services_with_load_balancer(&services, &lp, &http, "periodic").await {
                    eprintln!("periodic sync error: {}", err);
                }
            }
            
            // handle watcher events
            event = watcher_stream.next() => {
                match event {
                    Some(Ok(kube::runtime::watcher::Event::Applied(svc))) => {
                        if let Err(err) = register_service(&svc, &http, "event").await {
                            eprintln!("event registration failed: {}", err);
                        } else {
                            // sync services after successful registration
                            if let Err(err) = sync_services_with_load_balancer(&services, &lp, &http, "post-registration").await {
                                eprintln!("post-registration sync failed: {}", err);
                            }
                        }
                    }

                    Some(Ok(kube::runtime::watcher::Event::Deleted(svc))) => {
                        // get service name and namespace
                        let name = svc.name_any();
                        let namespace = svc.namespace().unwrap_or("default".to_string());
                        println!("service event: deleted - {}/{}", namespace, name);

                        // send DELETE request to lb - using same hostname as registration
                        let url = format!(
                            "http://load-balancer-service.default.svc.cluster.local:8080/api/unregister/{}",
                            name
                        );
                        println!("sending deregistration request to: {}", url);

                        // enhanced logging for deregistration
                        let res = http.delete(&url).send().await;
                        match res {
                            Ok(resp) => {
                                let status = resp.status();
                                println!(
                                    "deregistration successful for {}/{}: http {}",
                                    namespace, name, status
                                );

                                // log response body if available
                                if let Ok(body) = resp.text().await {
                                    if !body.is_empty() {
                                        println!("response body: {}", body);
                                    }
                                }
                                
                                // sync services after deregistration
                                if let Err(err) = sync_services_with_load_balancer(&services, &lp, &http, "post-deregistration").await {
                                    eprintln!("post-deregistration sync failed: {}", err);
                                }
                            }
                            Err(err) => {
                                eprintln!(
                                    "deregistration failed for {}/{}: {}",
                                    namespace, name, err
                                );
                                eprintln!("check if lb is running at: {}", url);
                            }
                        }
                    }

                    Some(Ok(event)) => {
                        println!(
                            "received unhandled event type: {:?}",
                            std::mem::discriminant(&event)
                        );
                    }

                    Some(Err(err)) => {
                        eprintln!("watcher error occurred: {}", err);
                        eprintln!("continuing to watch for service events...");
                    }

                    None => {
                        println!("watcher stream ended");
                        break;
                    }
                }
            }
        }
    }

    println!("watcher stopped");
    Ok(())
}