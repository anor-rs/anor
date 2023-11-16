use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use http_common::http_range::{CompleteLength, HttpRange};
use hyper::Request;
use hyper_util::rt::TokioIo;
use std::ops::Range;
use tokio::io::{self, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;

// A simple type alias so as to DRY.
type HttpClientResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn get_file(url: &str) {
    get_file_in_range(url, None)
}

pub fn get_file_in_range(url: &str, range: Option<Range<u64>>) {
    let uri = parse_url_to_uri(url);
    let async_runtime = Runtime::new().unwrap();
    async_runtime.block_on(async {
        let result = request_url("GET", uri, range).await;
        if let Err(err) = result {
            tracing::error!("Connection failed: {:?}", err)
        }
    });
}

pub fn get_file_info(url: &str) {
    let uri = parse_url_to_uri(url);
    let async_runtime = Runtime::new().unwrap();
    async_runtime.block_on(async {
        let result = request_url("HEAD", uri, None).await;
        if let Err(err) = result {
            tracing::error!("Connection failed: {:?}", err)
        }
    });
}

pub fn parse_url_to_uri(url: &str) -> hyper::Uri {
    url.parse::<hyper::Uri>().unwrap()
}

pub async fn request_url(
    method: &str,
    uri: hyper::Uri,
    range: Option<Range<u64>>,
) -> HttpClientResult<()> {
    let host = uri.host().expect("uri has no host");
    let port = uri.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            tracing::error!("Connection failed: {:?}", err);
        }
    });

    tracing::trace!(
        "File client connected to {}://{}:{}",
        uri.scheme().unwrap(),
        host,
        port
    );

    let authority = uri.authority().unwrap().clone();

    let mut req = Request::builder()
        .uri(uri)
        .method(method)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    if let Some(range_v) = range {
        let http_range = HttpRange {
            ranges: vec![range_v],
            complete_length: Some(CompleteLength::Unknown),
        };
        req.headers_mut().append(
            hyper::header::CONTENT_RANGE,
            http_range.to_header().parse().unwrap(),
        );
    }

    tracing::trace!("Request:\n{:#?}", req);

    let mut res = sender.send_request(req).await?;

    if tracing::enabled!(tracing::Level::TRACE) {
        tracing::trace!("Response status: {}", res.status());
        tracing::trace!("Response headers:\n{:#?}", res.headers());
    }

    // Stream the body, writing each chunk to stdout as we get it
    // (instead of buffering and printing at the end).
    while let Some(next) = res.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            io::stdout().write_all(chunk).await?;
        }
    }

    Ok(())
}
