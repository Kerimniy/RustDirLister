use std::ffi::OsStr;
use std::path::PathBuf;
use axum::{http::{StatusCode}, RequestExt};
use axum::{response::IntoResponse, routing::{get,post}, Router, extract::{Query, Json}};
use axum::body::Body;
use tokio::fs::File;
use askama;
use tokio_util::io::ReaderStream;
use hyper::header;

use mime_guess;
use once_cell::sync::Lazy;

use serde_json;
use serde::Serialize;

use std::time::{Duration, SystemTime};
use askama::Template;
use chrono::prelude::DateTime;
use chrono::Local;

use percent_encoding;
use axum::extract::OriginalUri;

use tower_cookies::{Cookies, Cookie, CookieManagerLayer, Key};

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;
use std::default::Default;
use std::io::Write;
use axum::response::Redirect;
use walkdir::WalkDir;


use rand::random;
use tower_cookies::cookie::SameSite;

static SECRET_KEY: Lazy<tower_cookies::cookie::Key> = Lazy::new(||Key::from(&read_secret_key(".SECRETKEY")));


#[derive(askama::Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    dirs: String,
    files: String,
    md: String
}

#[derive(Debug, Serialize)]
struct ElementInfo{
    el_type: &'static str,
    size: u64,
    modified: String
}
#[derive(Debug)]
struct MDInfo{
    time: SystemTime,
    source: String,
}

#[derive(serde::Deserialize,Default)]
struct SearchQuery {
    search: String,

}

#[derive(serde::Deserialize, Default)]
struct UserInfo {
    login: String,
    password: String,
}

#[derive(Clone)]
struct AppState {
    key: Key,
}

impl axum::extract::FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}
static LOGIN_TMPL: Lazy<String> = Lazy::new(|| {std::fs::read_to_string("templates/login.html").unwrap_or(String::new())});
static HOST: Lazy<String> = Lazy::new(|| {std::fs::read_to_string("HOST").unwrap_or(String::from("127.0.0.1:8000"))});
static SERVING_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {let strr = std::fs::read_to_string("PATH").unwrap_or(String::from("rdl_static")); std::path::PathBuf::from(strr)});
static MD_CACHE: Lazy<Arc<Mutex<HashMap<String, MDInfo>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static USER: OnceLock<UserInfo> = OnceLock::new();
static AUTH_ENABLED: OnceLock<bool> =OnceLock::new();

fn stToDateAndTime(st: SystemTime) -> String{
    let dt : DateTime<Local> = st.into();
    dt.with_timezone(&dt.timezone()).format("%Y-%m-%d %H-%M-%S").to_string()
}

async fn search(rsp: &mut HashMap<String,ElementInfo>,files: &mut HashMap<String,ElementInfo>,file_path:PathBuf, needle:&str){

    let mut count = 0;

    let start_time = SystemTime::now();

    for entry in WalkDir::new(file_path).into_iter().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().to_lowercase().contains(&needle)) {
        let now = SystemTime::now();

        if now.duration_since(start_time).unwrap_or_default().as_secs()>2{
            return;
        }

        let entry_path = entry.path();
        let metadata = match tokio::fs::metadata(&entry_path).await  {
            Ok(metadata) => metadata,
            Err(_) => { return },
        };

        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let size = metadata.len();
        if metadata.is_file() {
            let eli = ElementInfo{
                el_type: "File",
                size: size,
                modified: stToDateAndTime(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
            };

            files.insert(name.to_string(),eli);
        }
        else {
            let eli = ElementInfo{
                el_type: "Dir",
                size: 0,
                modified: String::new(),
            };

            rsp.insert(name.to_string(),eli);
        }

        count+=1;
        if count > 25{
            break;
        }
    }

}
async fn handle(OriginalUri(uri): OriginalUri, request: axum::http::Request<axum::body::Body>) -> impl IntoResponse {

    let path_raw =percent_encoding::percent_decode_str(uri.path()).decode_utf8().unwrap().to_string();
    let path = path_raw.trim_matches('/');
    if path.starts_with("rdl_static"){
        let file_path = std::path::Path::new(path);
        let metadata = match tokio::fs::metadata(&file_path).await  {
            Ok(metadata) => metadata,
            Err(_) =>{return StatusCode::NOT_FOUND.into_response()},
        };
        if metadata.is_dir() {
            return StatusCode::NOT_FOUND.into_response();
        }
        else if metadata.is_file() {

            let content_type = mime_guess::from_path(&file_path).first_or_octet_stream();

            let header_val1 = format!("filename=\"file.bin\"; filename*=UTF-8''{} ", file_path.file_name().unwrap_or(OsStr::new("file.bin")).to_string_lossy());
            let header = [
                (header::CONTENT_TYPE, content_type.essence_str().to_owned()),
                (header::CONTENT_DISPOSITION, header_val1)
            ];
            let file = File::open(file_path).await.unwrap();
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            return  (header, body).into_response()
        }

        return StatusCode::NOT_IMPLEMENTED.into_response();
    }


    if AUTH_ENABLED.get().unwrap_or(&false)==&true {
        let cookies = request.extensions().get::<Cookies>();
        let v = cookies.unwrap_or(&Cookies::default()).signed(&SECRET_KEY).get("session");

        match v {
            Some(v) => { if v.value()!="auth"{return Redirect::temporary("/login").into_response();}},
            None => {return Redirect::temporary("/login").into_response();}
        };
    }
    let cookies = request.extensions().get::<Cookies>().unwrap_or(&Cookies::default()).clone();

    let mut is_search = false;
    let q: Query<SearchQuery> = match request.extract().await{
        Ok(q) => {is_search=true; q},
        Err(_) => { Query::default()}
    };
    if q.0.search.trim() !=""{

        let mut rsp: HashMap<String,ElementInfo>= HashMap::new();
        let mut files: HashMap<String,ElementInfo>= HashMap::new();

        let path_raw =percent_encoding::percent_decode_str(uri.path()).decode_utf8().unwrap().to_string();
        let path = path_raw.trim_matches('/');
        let file_path = SERVING_PATH.join(path);

        let needle = q.0.search.to_lowercase();

        search(&mut rsp,&mut files,file_path,&needle).await;

        let dirs_json = serde_json::to_string(&rsp).unwrap_or_default();
        let files_json = serde_json::to_string(&files).unwrap_or_default();

        let header = [
            (header::CONTENT_TYPE, "text/html".to_owned()),
        ];
        let tmpl = IndexTemplate{dirs: dirs_json,md: "".to_string() , files: files_json}.render().unwrap_or(String::new());
        return  (header, tmpl).into_response()
    }
    let md_found = false;
    let mut md_source: String= "".to_string();


    let file_path = SERVING_PATH.join(path);


    let metadata = match tokio::fs::metadata(&file_path).await  {
        Ok(metadata) => metadata,
        Err(_) => { let header = [
            (header::CONTENT_TYPE, "text/html".to_owned()),
            (header::WWW_AUTHENTICATE, r#"Basic realm="Restricted""#.to_string())
        ]; let tmpl = IndexTemplate{dirs: "{}".to_string(),md: String::new() , files: "{}".to_string()}.render().unwrap_or(String::new()); return (header, tmpl).into_response()},
    };


    if metadata.is_file() {
        let header_val1 = format!("attachment; filename=\"file.bin\"; filename*=UTF-8''{} ", file_path.file_name().unwrap_or(OsStr::new("file.bin")).to_string_lossy());

        let file = File::open(file_path).await.unwrap();
        let size = match file.metadata().await{
            Ok(metadata) => metadata.len(),
            Err(_) => 0
        };
        let header = [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::CONTENT_DISPOSITION, header_val1),
            (header::CONTENT_LENGTH, size.to_string())
        ];
        let stream = ReaderStream::new(file);
        let body = Body::from_stream(stream);
        return  (header, body).into_response()
    }
    else {
        let mut dir = tokio::fs::read_dir(&file_path).await.unwrap();
        let mut rsp: HashMap<String,ElementInfo>= HashMap::new();
        let mut files: HashMap<String,ElementInfo>= HashMap::new();

        while let Some(entry) = dir.next_entry().await.unwrap() {

            let inner_path = file_path.join(entry.file_name());
            let metadata =match std::fs::metadata(&inner_path){
                Ok(m) => m,
                Err(e) =>{
                    continue
                }
            };

            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if md_found == false && name == "README.md" {
                let mut md_cache_h = MD_CACHE.lock().await;
                let c_md_info = md_cache_h.get(&inner_path.to_string_lossy().to_string());
                if let Some(c_md_info) = c_md_info {
                    let last_modified = c_md_info.time;

                    if metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH).duration_since(last_modified).unwrap_or(Duration::new(0, 0)) == Duration::new(0, 0) {
                        md_source = c_md_info.source.clone();
                    }
                    else {
                        let bytes_md = tokio::fs::read(&inner_path).await.unwrap_or_default();
                        md_source = std::str::from_utf8(&bytes_md).unwrap_or_default().to_string();


                        md_cache_h.insert(inner_path.to_string_lossy().to_string(), MDInfo { time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), source: md_source.to_string() });
                    }
                }
                else {
                    let bytes_md = tokio::fs::read(&inner_path).await.unwrap_or_default();
                    md_source = std::str::from_utf8(&bytes_md).unwrap_or_default().to_string();

                    md_cache_h.insert(inner_path.to_string_lossy().to_string(), MDInfo { time: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH), source: md_source.to_string() });
                }
            }
            let size = metadata.len();
            if metadata.is_file() {
                let eli = ElementInfo{
                    el_type: "File",
                    size: size,
                    modified: stToDateAndTime(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)),
                };

                files.insert(name.to_string(),eli);
            }
            else {
                let eli = ElementInfo{
                    el_type: "Dir",
                    size: 0,
                    modified: String::new(),
                };

                rsp.insert(name.to_string(),eli);
            }


        }
        let dirs_json = serde_json::to_string(&rsp).unwrap_or_default();
        let files_json = serde_json::to_string(&files).unwrap_or_default();

        let header = [
            (header::CONTENT_TYPE, "text/html".to_owned()),
            (header::WWW_AUTHENTICATE, r#"Basic realm="Restricted""#.to_owned()),
        ];
        let tmpl = IndexTemplate{dirs: dirs_json,md: md_source.to_string() , files: files_json}.render().unwrap_or(String::new());
        (header, tmpl).into_response()

    }

}

async fn login(request: axum::http::Request<axum::body::Body>)-> impl IntoResponse{

    if AUTH_ENABLED.get().unwrap_or(&false)==&true {
        let cookies = request.extensions().get::<Cookies>().unwrap_or(&Cookies::default()).clone();
        let payload: Json<UserInfo> = request.extract().await.unwrap();
        let usr = match USER.get() {
            Some(u) => u,
            None => return (StatusCode::BAD_REQUEST, "").into_response(),
        };

        if payload.login == usr.login && payload.password == usr.password {
            let mut cookie = Cookie::new("session", "auth");
            cookie.set_path("/");
            cookie.set_http_only(true);
            cookie.set_same_site(SameSite::Strict);
            cookie.make_permanent();

            cookies.signed(&SECRET_KEY).add(cookie);
            return (StatusCode::OK).into_response()

        }
    }

    return (StatusCode::BAD_REQUEST).into_response()
}

async fn render_login() -> impl IntoResponse{
    let header = [
        (header::CONTENT_TYPE, "text/html".to_owned()),
    ];
    let tmpl = LOGIN_TMPL.as_str();
    (header, tmpl).into_response()
}

fn create_file_if_not_exists(path: &str, default: &str){

    match std::fs::File::open(path){
        Ok(_) => return,
        Err(e) => {
            let mut p = std::fs::File::create(path).expect("Couldn't create PATH file");
            p.write_all(default.as_bytes()).expect("Couldn't write to PATH");
        }
    }



}

#[tokio::main]
async fn main() {


    create_file_if_not_exists("PATH","rdl_static");
    create_file_if_not_exists("HOST", "127.0.0.1:8000");

    let user_str = match tokio::fs::read_to_string("USER").await {
        Ok(s) => s,
        Err(e) => {"".to_string()}
    };
    let splitted = user_str.split(" ").collect::<Vec<&str>>();
    if splitted.len() == 2 {
        let usr = UserInfo {
            login:splitted[0].to_string(),
            password: splitted[1].to_string()
        };
        USER.set(usr);
        AUTH_ENABLED.set(true);
    }
    else {
        let usr = UserInfo {
            login: String::new(),
            password: String::new()
        };
        USER.set(usr);
    }
    let state = AppState {
        key: Key::generate(),
    };
    let app = Router::new().route("/login",get(render_login)).route("/-/api/login",post(login)).route("/{*path}", get(handle)).route("/", get(handle)).with_state(state).layer(CookieManagerLayer::new());


    let host = HOST.clone();

    let listener = tokio::net::TcpListener::bind(host).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn read_secret_key(path: &str) -> [u8;64]{
    let content =match std::fs::read(path){
        Ok(string_content) => string_content,
        Err(_) => {

            let key: [u8;64] = random();
            std::fs::write(".SECRETKEY", key);
            return key;
            key.to_vec()
        }
    };

    let mut key:[u8;64] = [0;64];

    if content.len() !=64{
        key = random();
        std::fs::write(".SECRETKEY", key);

    }
    else {
        for i in 0..64{
            key[i] = content[i];
        }
    }

    key
}