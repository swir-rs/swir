use futures::channel::oneshot;

use tokio::sync::mpsc;

use crate::utils::structs::*;


use crate::swir_grpc_internal_api;
use crate::swir_common;



#[derive(Debug)]
pub struct SwirServiceInvocationDiscoveryApi {
    pub from_client_to_si_sender: mpsc::Sender<RestToSIContext>,
}

impl SwirServiceInvocationDiscoveryApi {
    
    pub fn new(from_client_to_si_sender: mpsc::Sender<RestToSIContext>) -> Self {
        SwirServiceInvocationDiscoveryApi {
            from_client_to_si_sender
        }
    }
}



#[tonic::async_trait]
impl swir_grpc_internal_api::service_invocation_discovery_api_server::ServiceInvocationDiscoveryApi for SwirServiceInvocationDiscoveryApi {    
    async fn invoke(&self, request: tonic::Request<swir_common::InvokeRequest>) -> Result<tonic::Response<swir_common::InvokeResponse>, tonic::Status>{
	
	let req = request.into_inner();
	debug!("invoke internal : {}",req);

	if !validate_method(req.method){
	    return Err(tonic::Status::invalid_argument("Unsupported method"));
	}
	
	let job = SIJobType::InternalInvoke{
	    req
	};

	let (local_sender, local_rx): (oneshot::Sender<SIResult>, oneshot::Receiver<SIResult>) = oneshot::channel();

	let ctx =RestToSIContext{
	    job,
	    sender:local_sender	    
	};
	let mut sender = self.from_client_to_si_sender.clone();
	let res = sender.try_send(ctx);
	if let Err(e) = res{
            warn!("invoke internal : Channel is dead {:?}", e);
	    Err(tonic::Status::internal("Internal error"))
	}else{    
	    let response_from_service: Result<SIResult, oneshot::Canceled> = local_rx.await;
	    
	    if let Ok(res) = response_from_service {
		debug!("invoke internal : Got result from client {}", res);
		if let Some(si_response) = res.response{
		    Ok(tonic::Response::new(si_response))		    
		}else{
		    Ok(tonic::Response::new(swir_common::InvokeResponse{
			correlation_id: res.correlation_id,
			result: Some(swir_common::InvokeResult{
			    status: swir_common::InvokeStatus::Error as i32,
			    msg: res.status.to_string()
			}),
			..Default::default()		    
		    }))
		}
	    } else {
		Err(tonic::Status::internal("Internal error : canceled"))
	    }
	}
	
    }
}

