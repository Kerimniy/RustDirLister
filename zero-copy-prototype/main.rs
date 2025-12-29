use urlencoding;
use std::os::windows::io::{AsRawSocket, IntoRawSocket};

use tokio::task;
use windows_sys::Win32::Networking::WinSock::{TransmitFile, TRANSMIT_FILE_BUFFERS, TF_USE_KERNEL_APC};

use std::os::windows::io::AsRawHandle;
use tokio::io::AsyncWriteExt;
use std::{convert::Infallible, net::SocketAddr};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::Metadata;
use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

use once_cell::sync::Lazy;

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use hyper::StatusCode;

use serde_json;
use serde::Serialize;
use chrono::prelude::{DateTime, Utc};
use chrono::{Local, TimeZone};
use tokio::time::{timeout, Duration};

use askama::Template;
use libflate;
use core2::io::{Read, Write};

use httpdate::fmt_http_date;


#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    dirs: String,
}

#[derive(Debug, Serialize)]
struct Element{
    el_type: &'static str,
    name: String,
    size: u64,
    modified: String
}
#[derive(Debug, Serialize)]
struct Elements{
    elements: Vec<Element>,
}

#[derive(Debug, Clone, Copy, Default)]
enum Method {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    UPDATE,



}

impl Method {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "GET"    => Some(Method::GET),
            "POST"   => Some(Method::POST),
            "PUT"    => Some(Method::PUT),
            "DELETE" => Some(Method::DELETE),
            "UPDATE" => Some(Method::UPDATE),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Request {
    route: String,
    method: Method,
    host: String,
    http_version: String,
}

impl Request {
    fn parse(input: &str) -> Option<Self> {
        let mut lines = input.lines();

        let request_line = lines.next()?;
        let mut parts = request_line.split_whitespace();

        let method_str = parts.next()?;

        let route = urlencoding::decode(parts.next()?).unwrap_or(Cow::from("")).to_string();
        let http_version = parts.next()?.to_string();

        let method = Method::parse(method_str)?;

        let mut host = None;

        for line in lines {
            if line.is_empty() {
                break;
            }

            if let Some(value) = line.strip_prefix("Host:") {
                host = Some(value.trim().to_string());
            }
        }

        Some(Self {
            route,
            method,
            host: host?,
            http_version,
        })
    }
}

static base_path: Lazy<PathBuf> = Lazy::new(|| {PathBuf::from("testdir")});

fn stToDateAndTime(st: SystemTime) -> String{
    let dt : DateTime<Local> = st.into();
    dt.with_timezone(&dt.timezone()).format("%Y-%m-%d %H-%M-%S").to_string()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {


    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buff = [0; 4096];
        let n = match timeout(
            Duration::from_secs(20),
            stream.read(&mut buff),
        ).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => continue,
            Err(_) => {
                continue;
            }
        };

        let mut gzip_enabled = false;

        let read = String::from_utf8_lossy(&buff[0..n]).to_string();
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);
        let res = req.parse(&buff).unwrap();
        
            match req.path {
                Some(ref path) => {
                    for el in req.headers{
                        println!("{:?}", el.name );
                        if el.name == "Accept-Encoding"{
                            if std::str::from_utf8(el.value).unwrap_or("").contains("gzip")
                            {
                                gzip_enabled = true;
                            }
                            break;
                        }
                        if el.name == "If-Modified-Since"{
                            println!("{}", String::from_utf8(el.value.to_vec()).unwrap_or(String::from("s")));
                        }
                    }
                },
                None => {
                }
            }

        if let Some(req) = Request::parse(&read) {
            if req.route.contains("../"){
                write_response(&mut stream,StatusCode::FORBIDDEN, "".to_string()).await?;
            }
            let trimmed_route = req.route.trim_matches('/');
            if trimmed_route.starts_with("rdlstatic"){


                let metadata =match std::fs::metadata(&trimmed_route){
                    Ok(m) => m,
                    Err(e) =>{
                        write_response(&mut stream,StatusCode::NOT_FOUND, "".to_string()).await?;
                        continue;
                    }
                };


                if metadata.is_dir(){
                    write_response(&mut stream,StatusCode::NOT_FOUND, "".to_string()).await?;
                    continue;
                }
                else if metadata.is_file() {
                    send_static(&mut stream,req.route.trim_matches('/').to_string(),gzip_enabled, &metadata).await;
                }


                continue
            }

            let path =base_path.join(trimmed_route);

            let metadata =match std::fs::metadata(&path){
                Ok(m) => m,
                Err(e) =>{
                    write_response(&mut stream,StatusCode::NOT_FOUND, "".to_string()).await?;
                    continue;
                }
            };
            if metadata.is_dir() {
                let mut paths = tokio::fs::read_dir(&path).await.unwrap();

                let mut rsp: Elements = Elements{elements: Vec::new()};


                while let Some(entry) = paths.next_entry().await? {

                    let mut el: Element;

                    let inner_path = path.join(entry.file_name());
                    let metadata =match std::fs::metadata(&inner_path){
                        Ok(m) => m,
                        Err(e) =>{
                           continue
                        }
                    };

                    let file_name = entry.file_name();
                    let name = file_name.to_string_lossy();

                    let size = metadata.len();
                    if metadata.is_file() {
                        el = Element{
                            el_type: "File",
                            name: name.to_string(),
                            size: size,
                            modified: stToDateAndTime(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
                        };
                    }
                    else {
                        el = Element{
                            el_type: "Dir",
                            name: name.to_string(),
                            size: 0,
                            modified: String::new(),
                        };
                    }

                    rsp.elements.push(el);
                }
                let json = serde_json::to_string(&rsp).unwrap_or(String::new());

                let tmpl = IndexTemplate{dirs: json}.render().unwrap_or_default();

                write_response(&mut stream, StatusCode::OK, tmpl).await?;
            }
            else if metadata.is_file() {
                send_file(&stream,path.to_string_lossy().to_string()).await;
            }

        } else {
            eprintln!("Invalid HTTP request");
        }
    }
}

async fn write_response(
    stream: &mut tokio::net::TcpStream,
    status: StatusCode,
    body: String,
) -> std::io::Result<()> {

    let body_bytes = body.as_bytes();

    let rsp = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Cache-Control: max-age=2592000\r\n\
         \r\n",
        status,
        body_bytes.len()
    );

    stream.write_all(rsp.as_bytes()).await?;
    stream.write_all(body_bytes).await?;
    stream.shutdown().await
}


async fn send_static(
    stream: &mut tokio::net::TcpStream,
    path: String,
    gzip_enabled: bool,
    meta: &Metadata
) {
    let p = std::path::Path::new(&path);
    let raw = tokio::fs::read(&path).await.unwrap();

    let body = if gzip_enabled {
        let mut encoder = libflate::gzip::Encoder::new(Vec::new()).unwrap();
        encoder.write_all(&raw).unwrap();
        encoder.finish().into_result().unwrap()
    } else {
        raw
    };

    let filename = p
        .file_name()
        .unwrap_or(OsStr::new("download.bin"))
        .to_string_lossy();

    let cnt_type = mime_guess::from_path(p).first_or_octet_stream();
    let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let mut header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {}; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Content-Disposition:  filename=\"{}\"; filename*=UTF-8''{}\r\n\
         Cache-Control: max-age=2592000\r\n\
         Last-Modified: {}\r\n",
        cnt_type.essence_str(),
        body.len(),
        filename,
        urlencoding::encode(&filename),
        fmt_http_date(modified)


    );

    if gzip_enabled {
        header.push_str("Content-Encoding: gzip\r\n");
        header.push_str("Vary: Accept-Encoding\r\n");
    }

    header.push_str("\r\n");

    stream.write_all(header.as_bytes()).await.unwrap();
    stream.write_all(&body).await.unwrap();
    stream.shutdown().await.unwrap();
}
#[cfg(windows)]
async fn send_file(stream: &tokio::net::TcpStream, path: String) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::sync::Arc;

    let socket = stream.as_raw_socket();

    let path = Arc::new(path);

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use std::fs::File;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Networking::WinSock::{
            TransmitFile, TRANSMIT_FILE_BUFFERS, TF_USE_KERNEL_APC,
        };

        let file = File::open(&*path)
            .context("open file failed")?;

        let size = file.metadata()?.len();
        let filename = std::path::Path::new(&*path).file_name().unwrap().to_str().unwrap();
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/octet-stream\r\n\
             {}\r\n\
             Content-Length: {}\r\n\
             \r\n",
            format!("Content-Disposition: attachment; filename=\"download.bin\"; filename*=UTF-8''{}", urlencoding::encode(filename)),
            size
        );


        let mut buffers = TRANSMIT_FILE_BUFFERS {
            Head: header.as_ptr() as *mut _,
            HeadLength: header.len() as u32,
            Tail: std::ptr::null_mut(),
            TailLength: 0,
        };

        let ok = unsafe {
            TransmitFile(
                socket as _,
                file.as_raw_handle() as _,
                0,
                0,
                std::ptr::null_mut(),
                &mut buffers,
                0,
            )
        };

        if ok == 0 {
            anyhow::bail!(
                "TransmitFile failed: {}",
                std::io::Error::last_os_error()
            );
        }

        Ok(())
    }).await??;

    Ok(())
}


#[cfg(unix)]
async fn send_file(stream: &tokio::net::TcpStream, path: String) -> anyhow::Result<()> {
    use std::os::unix::io::AsRawFd;

    let socket_fd = stream.as_raw_fd();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use std::fs::File;
        use nix::sys::sendfile::sendfile;

        let file = File::open(&path)?;
        let size = file.metadata()?.len();
        let mut offset = 0;

        while offset < size {
            let sent = sendfile(
                socket_fd,
                file.as_raw_fd(),
                Some(&mut offset),
                (size - offset) as usize,
            )?;
            if sent == 0 {
                break;
            }
        }
        Ok(())
    })
        .await??;

    Ok(())
}