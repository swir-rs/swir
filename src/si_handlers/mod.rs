use crate::utils::config::{Services};
use crate::utils::structs::*;
use tokio::sync::{mpsc,Mutex};
use std::sync::Arc;
use multimap::MultiMap;
use futures::channel::oneshot;
use crate::swir_common;
use crate::swir_grpc_internal_api;
use crate::service_discovery::ServiceDiscovery;



type GrpcClient = swir_grpc_internal_api::service_invocation_discovery_api_client::ServiceInvocationDiscoveryApiClient<tonic::transport::Channel>;



fn map_client_to_backend_status_calls(ccsc:ClientCallStatusCodes)->(BackendStatusCodes,swir_common::InvokeResult){
    match ccsc{
	ClientCallStatusCodes::Ok(msg)=>(BackendStatusCodes::Ok(msg.clone()),swir_common::InvokeResult{status: swir_common::InvokeStatus::Ok as i32, msg}),
	ClientCallStatusCodes::Error(msg)=>(BackendStatusCodes::Error(msg.clone()),swir_common::InvokeResult{status: swir_common::InvokeStatus::Error as i32, msg})
    }
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

async fn invoke_handler(grpc_clients:Arc<Mutex<MultiMap<String, GrpcClient>>>, sender: oneshot::Sender<SIResult>, req: swir_common::InvokeRequest){
    let correlation_id =req.correlation_id.clone();
    let service_name = req.service_name.clone();
	
    debug!("public_invoke_handler: correlation {} {}",&correlation_id,&service_name);
    let mut grpc_clients = grpc_clients.lock().await;
    if let Some(client) = grpc_clients.get_mut(&req.service_name){	
	let resp = client.invoke(req).await;
	trace!("public_invoke_handler: got response on internal {:?}",resp);
	if let Ok(result) = resp{
	    let result = result.into_inner();
	    let _res = sender.send(SIResult{
		correlation_id,
		status: BackendStatusCodes::Ok("Service call ok".to_string()),
		response: Some(result)
	    });
	}else{
	    let _res = sender.send(SIResult{
		correlation_id,
		status: BackendStatusCodes::Error(resp.unwrap_err().to_string()),
		response: None
	    });
	}			    
    }else{
	let _res = sender.send(SIResult{
	    correlation_id,
	    status: BackendStatusCodes::NoService(format!("Service {} has not been resolved",service_name)),
	    response: None
	});			
    }
}




pub struct ServiceInvocationService{
    grpc_clients: Arc<Mutex<MultiMap<String, GrpcClient>>>    
}



impl ServiceInvocationService{
    pub fn new()->Self{
	ServiceInvocationService{
	    grpc_clients: Arc::new(Mutex::new(MultiMap::<String, GrpcClient>::new())),	    
	}
    }
       
    pub async fn start<T: ServiceDiscovery>(&self, services: Services,resolver: &T, mut receiver: mpsc::Receiver<RestToSIContext>,http_sender: mpsc::Sender<BackendToRestContext>) {	
	let services_to_resolve = services.resolve_services.clone();
	let services_to_announce = services.announce_services.clone();

	let (sender,mut resolve_receiver) = tokio::sync::mpsc::channel(10);
	
		
	let services_to_announce = services_to_announce.clone();			
	for svc in services_to_announce.iter(){	    
	    let _svc = resolver.announce(&svc).await;		
	}
	
	for svc in services_to_resolve.iter(){
	    resolver.resolve(svc,sender.clone()).await;
	}
    

	let grpc_clients = self.grpc_clients.clone();
	let mut tasks = vec![];
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
		warn!("Channel closed");
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
			req} =>{
			invoke_handler(grpc_clients, ctx.sender, req).await;
		    },		
		    SIJobType::PublicInvokeHttp{
			req
		    } =>{
			invoke_handler(grpc_clients,ctx.sender, req).await;
		    },
		    SIJobType::InternalInvoke{
			req} =>{
			debug!("internal_invoke_handler: {}",req);
			let (s, r) = futures::channel::oneshot::channel();
			let correlation_id = req.correlation_id;
			let service_name = req.service_name.clone();
			let client_endpoint = client_endpoint_mapping.iter().filter(|s| s.service_details.service_name==service_name).map(|s| s.client_url.clone()).nth(0);
			if let Some(endpoint) = client_endpoint{
			    let method = swir_common::HttpMethod::from_i32(req.method).unwrap();
			    let mrc = BackendToRestContext {
				correlation_id: correlation_id.clone(),
				sender: Some(s),
				request_params: RESTRequestParams{
				    payload: req.payload,
				    headers: req.headers,
				    method: method.to_string(),
				    uri: format!("{}{}",endpoint, req.request_target)
				}
			    };
			    
			    if let Err(mpsc::error::SendError(_)) = http_sender.send(mrc).await{
				warn!("Unable to send {} {}. Channel is closed", service_name, correlation_id);
				let _res = ctx.sender.send(
				    SIResult{
					correlation_id,
					status: BackendStatusCodes::Error("Internal ereror".to_string()),
					response: None
				    });						
			    }else{
				
				if let Ok(response_from_client) = r.await{
				    let (status, result) = map_client_to_backend_status_calls(response_from_client.status);
				    let _res = ctx.sender.send(					
					SIResult{
					    correlation_id:correlation_id.clone(),
					    status,
					    response : Some(swir_common::InvokeResponse{
						correlation_id,
						service_name,
						result: Some(result),
						payload: response_from_client.response_params.payload.to_owned(),
						..Default::default()						    
					    })
					});
				}else{
				    let _res = ctx.sender.send(
					SIResult{
					    correlation_id,
					    status: BackendStatusCodes::Error("Internal ereror".to_string()),
					    response: None
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
				    response : None
				});
			    
			}
		    }
		}
	    }
	    );		     			
	};
	

	futures::future::join_all(tasks).await;    
    }
}
