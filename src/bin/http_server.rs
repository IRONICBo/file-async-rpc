use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use tokio::runtime::Runtime;

async fn echo_service(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await?;
    Ok(Response::new(Body::from(whole_body)))
}

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let make_svc = make_service_fn(|_conn| async {
            Ok::<_, hyper::Error>(service_fn(echo_service))
        });

        let addr = ([127, 0, 0, 1], 8990).into();
        let server = Server::bind(&addr).serve(make_svc);
        println!("Listening on http://{}", addr);

        if let Err(e) = server.await {
            eprintln!("server error: {}", e);
        }
    });
}
