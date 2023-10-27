use std::io::SeekFrom;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use anor_common::utils::config::Config;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncSeekExt;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, Result, StatusCode};
use hyper_util::rt::TokioIo;

use bytes::Bytes;
use http_body_util::Full;

use log;

use http_common::http_range::{self, HttpRange};

// A simple type alias so as to DRY.
type FileServerResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn start_file_server(
    config: Arc<Config>,
    flag_ready: Arc<AtomicBool>,
    flag_shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    assert!(config.http.is_some());
    let file_server_config = config.http.as_ref().unwrap();
    assert!(!file_server_config.listen_on.is_empty());
    let listen_on = file_server_config.listen_on[0];

    log::info!("Starting HTTP service...");

    std::thread::spawn(move || {
        let async_runtime = Runtime::new().unwrap();
        async_runtime.block_on(async {
            if let Err(err) = start(listen_on, flag_ready, flag_shutdown).await {
                log::error!("HTTP service failed: {:?}", err);
            }
        });
    })
}

async fn start(
    listen_on: SocketAddr,
    flag_ready: Arc<AtomicBool>,
    flag_shutdown: Arc<AtomicBool>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(listen_on).await?;

    flag_ready.store(true, Ordering::SeqCst);

    log::info!("HTTP service running on http://{}", listen_on);

    while !flag_shutdown.load(Ordering::SeqCst) {
        let (stream, _) = listener.accept().await?;
        tokio::task::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(file_service))
                .await
            {
                log::error!("Failed to serve connection: {:?}", err);
            }
        });
    }
    Ok(())
}

async fn file_service(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    if log::log_enabled!(log::Level::Trace) {
        log::trace!("recevied request:{:#?}", req);
    }

    match *req.method() {
        Method::HEAD => file_info(&req).await,
        Method::GET => file_send(&req).await,
        _ => Ok(send_error_404()),
    }
}

/// HTTP status code 403
fn send_error_403() -> Response<Full<Bytes>> {
    blank_response(StatusCode::FORBIDDEN)
}

/// HTTP status code 404
fn send_error_404() -> Response<Full<Bytes>> {
    blank_response(StatusCode::NOT_FOUND)
}

/// HTTP status code 500
fn send_error_500() -> Response<Full<Bytes>> {
    blank_response(StatusCode::INTERNAL_SERVER_ERROR)
}

/// A blank response with status code
fn blank_response(status_code: StatusCode) -> Response<Full<Bytes>> {
    let mut response = Response::<Full<Bytes>>::new(Full::new(Bytes::new()));
    *response.status_mut() = status_code;
    response
}

async fn file_info(req: &Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    let path = req.uri().path().replace('/', "");
    let file_path = Path::new(&path);
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("file path:{:?}", file_path);
    }

    if file_path.file_name().is_none() {
        log::error!("filename is empty");
        return Ok(send_error_403());
    }

    match get_file_len(file_path).await {
        Ok(file_len) => {
            if let Ok(response) = Response::builder()
                .status(StatusCode::OK)
                .header(hyper::header::ACCEPT_RANGES, http_range::RANGE_UNIT)
                .header(hyper::header::CONTENT_LENGTH, file_len)
                .body(Full::new(Bytes::new()))
            {
                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("response:{:#?}", response);
                }
                Ok(response)
            } else {
                log::error!("unable to build response");
                Ok(send_error_500())
            }
        }
        Err(_err) => {
            log::error!("file not found: {:?}", file_path);
            Ok(send_error_404())
        }
    }
}

async fn get_file_len(filename: &Path) -> FileServerResult<u64> {
    let file = tokio::fs::File::open(filename).await?;
    let metadata = file.metadata().await?;
    if metadata.is_file() {
        let file_len = metadata.len();
        if log::log_enabled!(log::Level::Trace) {
            log::trace!(
                "The length of the file {:?} is {} bytes",
                filename,
                file_len
            )
        }
        return Ok(file_len);
    }
    let err_msg = format!("Not a file: {:?}", filename);
    log::error!("{err_msg}");
    Err(err_msg.into())
}

async fn file_send(req: &Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    let content_type: &str = "text/html; charset=utf-8";

    let path = req.uri().path().replace('/', "");
    let file_path = Path::new(&path);
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("file path: {:?}", file_path);
    }

    if file_path.file_name().is_none() {
        log::error!("filename is empty");
        return Ok(send_error_403());
    }

    let content_length: u64;
    if let Ok(file_len) = get_file_len(file_path).await {
        content_length = file_len;
    } else {
        log::error!("file not found: {:?}", file_path);
        return Ok(send_error_404());
    }

    let headers = req.headers();
    let http_range_option = if headers.contains_key(hyper::header::CONTENT_RANGE) {
        let content_range = headers.get(hyper::header::CONTENT_RANGE).unwrap();
        HttpRange::from_header(content_range.to_str().unwrap(), content_length)
    } else {
        None
    };

    match http_range_option {
        // send a response in ranges
        Some(http_range) => {
            send_file_range(file_path, content_type, content_length, &http_range).await
        }

        // send a response with full content
        None => send_file_full(file_path, content_type).await,
    }
}

async fn send_file_full(filename: &Path, content_type: &str) -> Result<Response<Full<Bytes>>> {
    if let Ok(contents) = tokio::fs::read(&filename).await {
        let body = contents.into();
        if let Ok(response) = Response::builder()
            .status(StatusCode::OK)
            .header(hyper::header::ACCEPT_RANGES, http_range::RANGE_UNIT)
            .header(hyper::header::CONTENT_TYPE, content_type)
            .body(Full::new(body))
        {
            return Ok(response);
        } else {
            log::error!("unable to build response");
            return Ok(send_error_500());
        }
    }
    Ok(send_error_404())
}

async fn send_file_range(
    filename: &Path,
    content_type: &str,
    content_length: u64,
    http_range: &HttpRange,
) -> Result<Response<Full<Bytes>>> {
    if http_range.none_satisfiable(content_length) {
        if let Ok(response) = Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(hyper::header::ACCEPT_RANGES, http_range::RANGE_UNIT)
            .header(
                hyper::header::CONTENT_RANGE,
                format!("{} */{}", http_range::RANGE_UNIT, content_length),
            )
            .body(Full::new(Bytes::new()))
        {
            if log::log_enabled!(log::Level::Debug) {
                log::debug!("Range Not Satisfiable (416). Requested range is out of existing content, {:?} > {}", http_range, content_length);
            }
            return Ok(response);
        } else {
            log::error!("unable to build response");
            return Ok(send_error_500());
        }
    }

    let ranges = &http_range.ranges;
    for range in ranges {
        let capacity = (range.end - range.start + 1) as usize;
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("preparing the range to send {:?}", range);
            log::trace!("capacity {}", capacity);
        }

        if HttpRange::range_satisfiable(range, content_length) {
            let mut buffer = vec![0; capacity];
            if let Ok(mut file) = tokio::fs::File::open(&filename).await {
                if let Ok(_seek) = file.seek(SeekFrom::Start(range.start)).await {
                    if log::log_enabled!(log::Level::Trace) {
                        log::trace!("seek result {}", _seek);
                    }
                    if let Ok(read_count) = file.read_exact(&mut buffer).await {
                        if log::log_enabled!(log::Level::Trace) {
                            log::trace!("read_count {}", read_count);
                        }
                        let body: Bytes = buffer.into();
                        if let Ok(response) = Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header(hyper::header::ACCEPT_RANGES, http_range::RANGE_UNIT)
                            .header(
                                hyper::header::CONTENT_RANGE,
                                format!(
                                    "{} {}-{}/{}",
                                    http_range::RANGE_UNIT,
                                    range.start,
                                    range.end,
                                    content_length
                                ),
                            )
                            .header(hyper::header::CONTENT_TYPE, content_type)
                            .body(Full::new(body))
                        {
                            return Ok(response);
                        } else {
                            log::error!("unable to build response");
                            return Ok(send_error_500());
                        }
                    } else {
                        log::error!("could not read bytes from file");
                    }
                } else {
                    log::error!("could not seek file position: {}", range.start);
                }
            } else {
                log::error!("could not open file: {:?}", filename);
            }
        }
    }

    Ok(send_error_404())
}
