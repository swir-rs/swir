use crate::utils::config::{Services};
use crate::utils::structs::*;
use tokio::sync::{mpsc,Mutex};
use std::sync::Arc;
use multimap::MultiMap;
use futures::channel::oneshot;




pub mod swir_grpc_internal_api {
    tonic::include_proto!("swir_internal");
}
type GrpcClient = swir_grpc_internal_api::service_invocation_discovery_api_client::ServiceInvocationDiscoveryApiClient<tonic::transport::Channel>;
use swir_grpc_internal_api::InvokeRequest;



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

async fn invoke_handler(clients:Arc<Mutex<MultiMap<String, GrpcClient>>>, sender: oneshot::Sender<SIResult>, correlation_id:String, service_name:String, req: ServiceInvokeRequest){
    debug!("public_invoke_handler: correlation {} {}",correlation_id,service_name);
    let mut clients = clients.lock().await;
    if let Some(client) = clients.get_mut(&service_name){
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
    clients: Arc<Mutex<MultiMap<String, GrpcClient>>>
}



impl ServiceInvocationService{

    pub fn new()->Self{
	ServiceInvocationService{
	    clients: Arc::new(Mutex::new(MultiMap::<String, GrpcClient>::new()))
	}
    }

    
    
    pub async fn start(&self, services: Services,mut receiver: mpsc::Receiver<RestToSIContext>,http_sender: mpsc::Sender<BackendToRestContext>) {

	let domains_to_resolve = services.resolve_domains.clone();
	let services_to_announce = services.announce_services.clone();


	let (sender,mut resolve_receiver) = tokio::sync::mpsc::channel(10);
	let responder = Arc::new(Mutex::new(mdns_responder::Responder::new().unwrap()));
	
	let resp = responder.clone();
	let mut tasks = vec![];
	let h1 = tokio::spawn(
	    async move{	    
		let responder = resp.lock().await;
		let tasks = responder.start();
		for svc in services_to_announce.iter(){
		    let service_id = &svc.service_id;
		    debug!("resolver: announcing service {:?}",service_id);
		    let _svc = responder.register(
			svc.domain.to_owned(),
			svc.name.to_owned(),        
			50012,
			&["path=/"],
		    ).await;		
		}
		
		for domain in domains_to_resolve.iter(){
		    debug!("resolver: resolving domain {}",domain);
		    responder.resolve(domain.to_owned(),sender.clone()).await;
		}
		futures::future::join_all(tasks).await;
	    }
	);
	tasks.push(h1);

	
	let clients = self.clients.clone();
	let h2 = tokio::spawn(
	    async move {
		while let Some((service_name,socket_addr)) = resolve_receiver.recv().await{
		    if let Some((svc_name, _svc_domain)) = parse_service_name(&service_name){
			    debug!("resolver: service {} {} found at {:?} ", service_name, svc_name, socket_addr);
			    let mut clients = clients.lock().await;
			if !clients.contains_key(&svc_name){
			    let url = format!("http://{}",socket_addr);
			    match GrpcClient::connect(url).await{
				Ok(client) => {
				    clients.insert(svc_name.clone(), client);
				},
				Err(e) =>{
				    warn!("Can't connect to {} with {:?}",service_name, e);
				}
			    }
			};
		    }
		}
	    });
	
	tasks.push(h2);

	while let Some(ctx) = receiver.recv().await{
	    let clients = self.clients.clone();
	    let mut http_sender = http_sender.clone();
	    tokio::spawn(async move{		
		let ctx = ctx;
		let job = ctx.job;
		match job {
		    SIJobType::PublicInvokeGrpc{
			correlation_id,
			service_name,
			req} =>{
			invoke_handler(clients, ctx.sender, correlation_id, service_name, req).await;
		    },		
		    SIJobType::PublicInvokeHttp{
			correlation_id,
			service_name,
			req
		    } =>{
			invoke_handler(clients,ctx.sender, correlation_id, service_name, req).await;
		    },
		    SIJobType::InternalInvoke{
			correlation_id,
			service_name,
			req} =>{
			debug!("internal_invoke_handler: correlation {} {}",correlation_id,service_name);
			let (s, r) = futures::channel::oneshot::channel();			
			let mrc = BackendToRestContext {
			    correlation_id: correlation_id.clone(),
			    sender: Some(s),
			    request_params: RESTRequestParams{
				payload: req.payload,
				headers: req.headers,
				method: req.method,
				uri: format!("http://127.0.0.1:8090{}",req.request_target)
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
			    
			// send to the client channel but for time being this uses MessagingToRestcontext
			// so we just reflect the payload

			    
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
