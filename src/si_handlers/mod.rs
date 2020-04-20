use crate::utils::config::{Services,AnnounceServiceDetails,ServiceDetails};
use crate::utils::structs::*;
use tokio::sync::{mpsc,Mutex};
use std::sync::Arc;
use multimap::MultiMap;
use futures::channel::oneshot;
use rand::{rngs, Rng, SeedableRng};
use rand::distributions::{Alphanumeric};




pub mod swir_grpc_internal_api {
    tonic::include_proto!("swir_internal");
}
type GrpcClient = swir_grpc_internal_api::service_invocation_discovery_api_client::ServiceInvocationDiscoveryApiClient<tonic::transport::Channel>;
use swir_grpc_internal_api::InvokeRequest;


fn create_domain(svc: &ServiceDetails) -> String{
    return format!("_{}._{}._{}.local",svc.protocol,svc.service_name,svc.domain);
}

fn generate_instance_name()->String{
    rngs::SmallRng::from_entropy().sample_iter(Alphanumeric).take(16).collect()
}

fn get_service_domain_and_name(svc: &AnnounceServiceDetails) -> (String, String){    
    let domain = create_domain(&svc.service_details);
    let name = generate_instance_name();
    (name, domain)        
}


fn parse_service_name(service: &String )-> Option<(String,String)> {   
    let ind = service.find('.');
    if let Some(i) = ind{
	let service_type = String::from(&service[(i+1)..]);
	if service_type.ends_with("._swir.local"){
	    let  ending_len = "._swir.local".len();
	    let service_type_len = service_type.len();
	    let pos = service_type_len-ending_len;
	    let prot_and_service = String::from(&service_type[..pos]).replacen('_'," ",1).trim().to_string();
	    if let Some(i) = prot_and_service.find('.'){
		let service = String::from(&prot_and_service[(i+1)..]).replacen('_'," ",1).trim().to_string();
		Some((service,service_type))
	    }else{
		Some((prot_and_service,service_type))
	    }	    
	}else{
	    None
	}    	
    }else{
	None
    }
}

async fn invoke_handler(grpc_clients:Arc<Mutex<MultiMap<String, GrpcClient>>>, sender: oneshot::Sender<SIResult>, correlation_id:String, service_name:String, req: ServiceInvokeRequest){
    debug!("public_invoke_handler: correlation {} {}",correlation_id,service_name);
    let mut grpc_clients = grpc_clients.lock().await;
    if let Some(client) = grpc_clients.get_mut(&service_name){
	let internal_req = InvokeRequest{
	    correlation_id: correlation_id.clone(),
	    service_name: service_name,
	    method: req.method.to_string(),
	    request_target: req.request_target,
	    headers: req.headers,
	    payload: req.payload
	};
	
	let resp = client.invoke(internal_req).await;
	debug!("public_invoke_handler: got response on internal {:?}",resp);
	if let Ok(result) = resp{
	    let result = result.into_inner();
	    let _res = sender.send(SIResult{
		correlation_id,
		status: BackendStatusCodes::Ok("Service call ok".to_string()),
		payload: result.payload.clone()
	    });
	}else{
	    let _res = sender.send(SIResult{
		correlation_id,
		status: BackendStatusCodes::Error(resp.unwrap_err().to_string()),
		payload: vec![0]
	    });
	}			    
    }else{
	let _res = sender.send(SIResult{
	    correlation_id,
	    status: BackendStatusCodes::NoService(format!("Service {} has not been resolved",service_name)),
	    payload: vec![0]
	});			
    }
}




pub struct ServiceInvocationService{
    grpc_clients: Arc<Mutex<MultiMap<String, GrpcClient>>>,
}



impl ServiceInvocationService{

    pub fn new()->Self{
	ServiceInvocationService{
	    grpc_clients: Arc::new(Mutex::new(MultiMap::<String, GrpcClient>::new())),	    
	}
    }

       
    pub async fn start(&self, internal_port: u16, services: Services,mut receiver: mpsc::Receiver<RestToSIContext>,http_sender: mpsc::Sender<BackendToRestContext>) {

	
	let services_to_resolve = services.resolve_services.clone();
	let services_to_announce = services.announce_services.clone();

	let (sender,mut resolve_receiver) = tokio::sync::mpsc::channel(10);
	let responder = Arc::new(Mutex::new(mdns_responder::Responder::new().unwrap()));
	
	let resp = responder.clone();
	let mut tasks = vec![];
	let h1 = tokio::spawn(
	    async move{
		let services_to_announce = services_to_announce.clone();
		let responder = resp.lock().await;
		let tasks = responder.start();
		for svc in services_to_announce.iter(){
		    let (mdns_name, mdns_domain) = get_service_domain_and_name(&svc);		  		    
		    debug!("resolver: announcing service {} {}",mdns_name,mdns_domain);
		    let _svc = responder.register(
			mdns_domain.to_owned(),
			mdns_name.to_owned(),
			internal_port,
			&["path=/"],
		    ).await;		
		}
		
		for svc in services_to_resolve.iter(){
		    let domain = create_domain(&svc);
		    debug!("resolver: resolving domain {}",domain);
		    responder.resolve(domain.to_owned(),sender.clone()).await;
		}
		futures::future::join_all(tasks).await;
	    }
	);
	tasks.push(h1);

	
	let grpc_clients = self.grpc_clients.clone();
	let h2 = tokio::spawn(
	    async move {
		while let Some((service_name,socket_addr)) = resolve_receiver.recv().await{
		    if let Some((svc_name, _svc_domain)) = parse_service_name(&service_name){
			    debug!("resolver: service {} {} found at {:?} ", service_name, svc_name, socket_addr);
			    let mut grpc_clients = grpc_clients.lock().await;
			if !grpc_clients.contains_key(&svc_name){
			    let url = format!("http://{}",socket_addr);
			    match GrpcClient::connect(url.clone()).await{
				Ok(client) => {
				    grpc_clients.insert(svc_name.clone(), client);
				},
				Err(e) =>{
				    warn!("Can't connect to {} {} with {:?}",service_name,url,e);
				}
			    }
			};
		    }
		}
	    });
	
	tasks.push(h2);

	let client_endpoint_mapping = services.announce_services.clone();
	while let Some(ctx) = receiver.recv().await{
	    let grpc_clients = self.grpc_clients.clone();
	    let mut http_sender = http_sender.clone();
	    let client_endpoint_mapping = client_endpoint_mapping.clone();
	    tokio::spawn(async move{
		let client_endpoint_mapping = client_endpoint_mapping.clone();
		let ctx = ctx;
		let job = ctx.job;
		match job {
		    SIJobType::PublicInvokeGrpc{
			correlation_id,
			service_name,
			req} =>{
			invoke_handler(grpc_clients, ctx.sender, correlation_id, service_name, req).await;
		    },		
		    SIJobType::PublicInvokeHttp{
			correlation_id,
			service_name,
			req
		    } =>{
			invoke_handler(grpc_clients,ctx.sender, correlation_id, service_name, req).await;
		    },
		    SIJobType::InternalInvoke{
			correlation_id,
			service_name,
			req} =>{
			debug!("internal_invoke_handler: correlation {} {}",correlation_id,service_name);
			let (s, r) = futures::channel::oneshot::channel();


			let client_endpoint = client_endpoint_mapping.iter().filter(|s| s.service_details.service_name==service_name).map(|s| s.client_url.clone()).nth(0);
			if let Some(endpoint) = client_endpoint{			
			    let mrc = BackendToRestContext {
				correlation_id: correlation_id.clone(),
				sender: Some(s),
				request_params: RESTRequestParams{
				    payload: req.payload,
				    headers: req.headers,
				    method: req.method,
				    uri: format!("{}{}",endpoint, req.request_target)
				}
			    };
			    
			    if let Err(mpsc::error::SendError(_)) = http_sender.send(mrc).await{
				warn!("Unable to send {} {}. Channel is closed", service_name, correlation_id);
				let _res = ctx.sender.send(
				    SIResult{
					correlation_id,
					status: BackendStatusCodes::Error("Internal ereror".to_string()),
					payload: vec![]
				    });						
			    }else{
				
				if let Ok(response_from_client) = r.await{
				    let _res = ctx.sender.send(
					SIResult{
					    correlation_id,
					    status: BackendStatusCodes::Ok("Service call ok".to_string()),
					    payload: response_from_client.response_params.payload.to_owned()
					});
				}else{
				    let _res = ctx.sender.send(
					SIResult{
					    correlation_id,
					    status: BackendStatusCodes::Error("Internal ereror".to_string()),
					    payload: vec![]
					});			   		
				}
			    			  			    
			    }
			}else{
			    let msg = format!("Can't find client url for service name {}",service_name);
			    warn!("{}",msg);
			    let _res = ctx.sender.send(
				SIResult{
				    correlation_id,
				    status: BackendStatusCodes::Error(msg.to_string()),
				    payload: vec![0] 
				});
			    
			}
		    }
		}
	    }
	    );		     			
	};
	
	let resp = responder.lock().await;
	resp.shutdown().await;    
	
	futures::future::join_all(tasks).await;    
    }
}
