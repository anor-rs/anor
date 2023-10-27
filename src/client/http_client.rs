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
    let url = url.parse::<hyper::Uri>().unwrap();

    let async_runtime = Runtime::new().unwrap();
    async_runtime.block_on(async {
        let result = request_url("GET", url, range).await;
        if let Err(err) = result {
            log::error!("Connection failed: {:?}", err)
        }
    });
}

pub fn get_file_info(url: &str) {
    let url = url.parse::<hyper::Uri>().unwrap();

    let async_runtime = Runtime::new().unwrap();
    async_runtime.block_on(async {
        let result = request_url("HEAD", url, None).await;
        if let Err(err) = result {
            log::error!("Connection failed: {:?}", err)
        }
    });
}

async fn request_url(
    method: &str,
    url: hyper::Uri,
    range: Option<Range<u64>>,
) -> HttpClientResult<()> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            log::error!("Connection failed: {:?}", err);
        }
    });

    if log::log_enabled!(log::Level::Trace) {
        log::trace!(
            "File client connected to {}://{}:{}",
            url.scheme().unwrap(),
            host,
            port
        );
    }

    let authority = url.authority().unwrap().clone();

    let mut req = Request::builder()
        .uri(url)
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

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("Request:\n{:#?}", req);
    }

    let mut res = sender.send_request(req).await?;

    if log::log_enabled!(log::Level::Trace) {
        log::trace!("Response status: {}", res.status());
        log::trace!("Response headers:\n{:#?}", res.headers());
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
