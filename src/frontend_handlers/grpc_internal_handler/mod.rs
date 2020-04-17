use std::fmt;

use std::str::FromStr;
use futures::channel::oneshot;

use tokio::sync::mpsc;

use crate::utils::structs::*;


pub mod swir_grpc_internal_api {
    tonic::include_proto!("swir_internal");
}



impl fmt::Display for swir_grpc_internal_api::InvokeRequest{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InvokeRequest {{ correlation_id:{}, service_name: {} }}", &self.correlation_id, &self.service_name)
    }
}

impl fmt::Display for swir_grpc_internal_api::InvokeResponse{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InvokeResponse {{ correlation_id:{}, service_name: {} }}", &self.correlation_id, &self.service_name)
    }
}


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
    async fn invoke(&self, request: tonic::Request<swir_grpc_internal_api::InvokeRequest>) -> Result<tonic::Response<swir_grpc_internal_api::InvokeResponse>, tonic::Status>{
	
	let req = request.into_inner();
	debug!("invoke internal : {}",req);
	let correlation_id = req.correlation_id.clone();
	let service_name = req.service_name.clone();
	
	let method = if let Ok(method)  = HttpMethod::from_str(&req.method){
	    method
	}else{
	    return Err(tonic::Status::invalid_argument("Invalid method"))
	};


	let job = SIJobType::InternalInvoke{
	    correlation_id: req.correlation_id.to_owned(),
	    service_name: req.service_name.to_owned(),
	    req: ServiceInvokeRequest{
		method,
		request_target: req.request_target.to_owned(),
		headers: req.headers.to_owned(),
		payload: req.payload.to_owned()
	    }
	};

	let (local_sender, local_rx): (oneshot::Sender<SIResult>, oneshot::Receiver<SIResult>) = oneshot::channel();

	let ctx =RestToSIContext{
	    job,
	    sender:local_sender	    
	};
	let mut sender = self.from_client_to_si_sender.clone();
	let res = sender.try_send(ctx);
	if let Err(e) = res{
            warn!("Channel is dead {:?}", e);
	    Err(tonic::Status::internal("Internal error"))
	}else{    
	    debug!("Waiting for response");
	    let response_from_service: Result<SIResult, oneshot::Canceled> = local_rx.await;
	    debug!("Got result {:?}", response_from_service);
	    if let Ok(res) = response_from_service {
		Ok(tonic::Response::new(swir_grpc_internal_api::InvokeResponse{
		    correlation_id,
		    service_name,
		    payload: res.payload.to_owned(),
		    ..Default::default()		   		    
		})
		)
	    } else {
		Err(tonic::Status::internal("Internal error : canceled"))
	    }
	}	
    }
}

