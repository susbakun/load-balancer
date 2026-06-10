use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1 as client_http1;
use hyper::server::conn::http1;
use hyper::{Request, Response, body::Incoming};
use hyper_util::rt::TokioIo;

use super::*;

type BoxBody = http_body_util::combinators::UnsyncBoxBody<Bytes, hyper::Error>;

pub async fn setup_listener(pool: Arc<Mutex<Pool>>, listen_address: String) -> Result<()> {
    let listener = TcpListener::bind(listen_address).await?;
    while let Ok((client_stream, _)) = listener.accept().await {
        let pool_cloned = Arc::clone(&pool);
        tokio::spawn(async move {
            if let Err(err) = handle_connection(pool_cloned, client_stream).await {
                eprintln!("connection error: {err}");
            }
        });
    }
    Ok(())
}

async fn handle_connection(pool: Arc<Mutex<Pool>>, client_stream: TcpStream) -> Result<()> {
    let backend_address = {
        let mut pool = pool.lock().await;
        pool.next_server()
            .map(|server| server.address.clone())
            .ok_or_else(|| anyhow!("couldn't find a healthy server"))?
    };

    let io = TokioIo::new(client_stream);

    http1::Builder::new()
        .serve_connection(
            io,
            hyper::service::service_fn(move |req| {
                let backend = backend_address.clone();
                async move { serve_request(&backend, req).await }
            }),
        )
        .await?;

    Ok(())
}

async fn serve_request(
    backend: &str,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, Infallible> {
    match proxy_request(backend, req).await {
        Ok(response) => Ok(response),
        Err(err) => {
            eprintln!("proxy error: {err}");
            Ok(bad_gateway())
        }
    }
}

fn bad_gateway() -> Response<BoxBody> {
    Response::builder()
        .status(502)
        .body(
            Full::new(Bytes::from_static(b"Bad Gateway"))
                .map_err(|_| unreachable!())
                .boxed_unsync(),
        )
        .expect("valid 502 response")
}

async fn proxy_request(
    backend_address: &str,
    request: Request<Incoming>,
) -> Result<Response<BoxBody>> {
    let stream = TcpStream::connect(backend_address).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = client_http1::Builder::new().handshake(io).await?;

    tokio::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("backend connection error: {err}");
        }
    });

    let backend_response = sender.send_request(request).await?;
    let (parts, body) = backend_response.into_parts();
    let body = body
        .collect()
        .await
        .map_err(|err| anyhow!("failed to read backend body: {err}"))?
        .to_bytes();

    Ok(Response::from_parts(
        parts,
        Full::new(body).map_err(|_| unreachable!()).boxed_unsync(),
    ))
}
