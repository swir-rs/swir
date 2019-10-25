#![deny(warnings)]
extern crate futures;
extern crate hyper;
extern crate kafka;

use kafka::producer::{Producer,  Record, RequiredAcks};
use futures::future;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender,Receiver};
use std::{env,sync::Arc,time::Duration};
use std::sync::mpsc;
use std::thread;




type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;


fn echo(req: Request<Body>, counter: &AtomicUsize , s:&Sender<usize>) -> BoxFut {
    let mut response = Response::new(Body::empty());
    counter.fetch_add(1, Ordering::Relaxed);
    println!("Counter {}", counter.load(Ordering::Relaxed));
    let headers= req.headers();
    println!("Headers {:?}", headers);
    let (parts,body) = req.into_parts();
    println!("Parts {:?}", parts);
    println!("Body {:?}", body);

    match (parts.method, parts.uri.path()) {
        // Serve some instructions at /
        (Method::GET, "/") => {
            *response.body_mut() = Body::from("Try POSTing data to /echo");
        }

        // Simply echo the body back to the client.
        (Method::POST, "/echo") => {
//            let res = body.concat2().wait().unwrap();
//            println!("{:?}", res);
//            producer.send(&Record::from_value("Request", body.as_bytes())).unwrap();
            println!("Sending {}", counter.load(Ordering::Relaxed));
            s.send(counter.load(Ordering::Relaxed)).unwrap();
            *response.body_mut() = body;
        }

        // Convert to uppercase before sending back to client.
        (Method::POST, "/echo/uppercase") => {
            let mapping = body.map(|chunk| {
                chunk
                    .iter()
                    .map(|byte| byte.to_ascii_uppercase())
                    .collect::<Vec<u8>>()
            });

            *response.body_mut() = Body::wrap_stream(mapping);
        }

        // Reverse the entire body before sending back to the client.
        //
        // Since we don't know the end yet, we can't simply stream
        // the chunks as they arrive. So, this returns a different
        // future, waiting on concatenating the full body, so that
        // it can be reversed. Only then can we return a `Response`.
        (Method::POST, "/echo/reversed") => {
            let reversed = body.concat2().map(move |chunk| {
                let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
                *response.body_mut() = Body::from(body);
                response
            });

            return Box::new(reversed);
        }

        // The 404 Not Found route...
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let external_address = &args[1].parse().expect("Unable to parse socket address");;
    let request_counter = Arc::new(AtomicUsize::new(0));
    let broker_address = String::from(&args[2]);
    println!("Using kafka broker on {}", broker_address);
    println!("Listening on http://{}", external_address);


    let (tx, rx):(Sender<usize>, Receiver<usize>) = mpsc::channel();


    let mut producer =
        Producer::from_hosts(vec!(String::from(broker_address)))
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();

    let server = Server::bind(&external_address)
        .serve(move || {
            let inner_rc = Arc::clone(&request_counter);
            let inner_txx = mpsc::Sender::clone(&tx);
            service_fn(move |req| echo(req, &inner_rc,&mpsc::Sender::clone(&inner_txx)))
        }
        ).map_err(|e| eprintln!("server error: {}", e));




    let child = thread::spawn(move ||{
        loop {
            let i = rx.recv().unwrap();
            println!("Got {}", i);
            producer.send(&Record::from_value("Request",i.to_string())).unwrap();
        }
    });


    hyper::rt::run(server);
    
    child.join().unwrap()

}

