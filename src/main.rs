pub mod wts_array;
pub mod wts_wrapper;

use std::error::Error;

use actix_web::{get, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use windows::Win32::System::RemoteDesktop;
use windows::{core::PWSTR, Win32::Foundation::HANDLE};
use wts_array::WtsArray;
use wts_wrapper::WtsSessionInfoW;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !cfg!(target_os = "windows") {
        panic!("supported only windows")
    }
    let conf = read_app_config();
    let _wg = init_logs(&conf);

    info!("Application init.");
    info!("{:?}", &conf);

    // test
    {
        loop {
            let a = query_sessions();
            for b in &a {
                let client_name = get_wts_session_info(b.session_id, RemoteDesktop::WTSClientName);
                println!("{:?}", client_name)
            }
        }
    }

    // interval check
    {
        if (&conf).observable_interval > 0 {
            let c = conf.clone();
            actix_web::rt::spawn(observe_session_status(c));
        }
    }

    // rest api
    {
        HttpServer::new(|| App::new().service(api_users).service(api_sessions))
            .bind((conf.listen_address, conf.listen_port))?
            .run()
            .await
    }
}

fn init_logs(conf: &AppConfig) -> Vec<WorkerGuard> {
    let file_appender = tracing_appender::rolling::daily(&conf.log_dir, "app.log");
    let (nb_stdout, gd_stdout) = tracing_appender::non_blocking(std::io::stdout());
    let (nb_file, gd_file) = tracing_appender::non_blocking(file_appender);

    let all_files = nb_file.and(nb_stdout);
    tracing_subscriber::fmt()
        .with_writer(all_files)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    return vec![gd_stdout, gd_file];
}

async fn observe_session_status(conf: AppConfig) {
    use actix_web::rt::time;

    struct UserDiff {
        client_name: String,
        user_name: String,
        is_status_changed: bool,
    }

    let mut interval = time::interval(std::time::Duration::from_secs(conf.observable_interval));
    let mut last_sutatus = None;
    loop {
        interval.tick().await;
        info!("query users interval");
        let users = query_user().unwrap_or_else(|e| {
            error!(" query user error{:?}", e);
            vec![]
        });
        let mut user_clients: Vec<_> = users
            .into_iter()
            .map(|user| {
                let client_name = get_wts_session_info(user.id, RemoteDesktop::WTSClientName);
                UserClient {
                    user: user,
                    client_name: client_name,
                }
            })
            .collect();

        if last_sutatus.is_none() {
            last_sutatus =
                Some(user_clients.sort_by(|a, b| a.user.user_name.cmp(&b.user.user_name)));
            continue;
        }

        // last_sutatus.sort()
    }
}

#[tracing::instrument(level = "info")]
#[get("/api/users")]
async fn api_users() -> impl Responder {
    info!("query users web api");
    let users = query_user().unwrap_or_else(|e| {
        error!(" query user error{:?}", e);
        vec![]
    });
    let user_clients: Vec<_> = users
        .into_iter()
        .map(|user| {
            let client_name = get_wts_session_info(user.id, RemoteDesktop::WTSClientName);
            UserClient { user, client_name }
        })
        .collect();

    web::Json(user_clients)
}

#[tracing::instrument(level = "info")]
#[get("/api/sessions")]
async fn api_sessions() -> impl Responder {
    info!("session info web api");
    let sessions = query_sessions();
    web::Json(sessions)
}

const WTS_CURRENT_SERVER_HANDLE: HANDLE = HANDLE(0);
/// WTSQuerySessionInfoの問い合わせ結果を文字列で取得する
fn get_wts_session_info(session_id: u32, info_type: RemoteDesktop::WTS_INFO_CLASS) -> String {
    #[allow(dead_code)]
    const WTS_CURRENT_USER_SESSION_ID: u32 = 1;

    let result_txt = unsafe {
        let mut len: u32 = 0;
        let pwstr = std::ptr::null_mut();
        RemoteDesktop::WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            info_type,
            pwstr,
            &mut len,
        );
        let result = if len == 0 {
            "".to_string()
        } else {
            (*pwstr).to_string().unwrap_or("".to_string())
        };
        RemoteDesktop::WTSFreeMemory(pwstr as _);
        result
        // String::from_utf16_lossy(std::slice::from_raw_parts(
        //     result_pwstr.0,
        //     // u8 -> u16 and last cstr \0 remove
        //     (result_byte_size / 2 - 1) as usize,
        // ))
    };
    result_txt
}

fn query_sessions() -> Vec<WtsSessionInfoW> {
    unsafe {
        let mut data_ptr = std::ptr::null_mut();
        let mut len = 0u32;
        RemoteDesktop::WTSEnumerateSessionsW(
            WTS_CURRENT_SERVER_HANDLE,
            0,
            1,
            &mut data_ptr,
            &mut len,
        );

        let sessions = WtsArray::from_raw(data_ptr, len);

        sessions
            .as_slice()
            .iter()
            .map(|s| WtsSessionInfoW::from(s))
            .collect()
    }
}

/// Windowsのログインユーザー情報一覧を取得
fn query_user() -> Result<Vec<SessionUser>, Box<dyn Error>> {
    use regex::Regex;
    use std::process::Command;

    let out = Command::new("cmd")
        .args(vec!["/c", "chcp 65001 && query user"])
        .output()?;
    let text = String::from_utf8(out.stdout)?;

    let lf_text = text.replace("\r", "");
    // convert two more white space to tab
    let re = Regex::new(r"  +").expect("regex format err");
    let tsv_text = re.replace_all(&lf_text, "\t");
    let lines = tsv_text.split("\n");

    debug!("{}", tsv_text);
    let users: Vec<_> = lines
        // skip `active code page` message & headers
        .skip(2)
        .map(|line| {
            let items: Vec<_> = line.split("\t").collect();
            if items.len() > 4 {
                let user = SessionUser {
                    user_name: items.get(0)?.to_string(),
                    session_name: items.get(1)?.to_string(),
                    id: items.get(2)?.parse::<u32>().unwrap_or(0),
                    state: items.get(3)?.to_string(),
                    idle_time: items.get(4)?.to_string(),
                    login_time: items.get(5)?.to_string(),
                };
                debug!("{:?}", &user);
                Some(user)
            } else {
                debug!("{}", &line);
                None
            }
        })
        .flatten()
        .collect();

    Ok(users)
}

/// 環境変数からアプリ設定を読込
fn read_app_config() -> AppConfig {
    AppConfig {
        listen_address: option_env!("LISTEN_ADDRESS")
            .unwrap_or("127.0.0.1")
            .to_string(),
        listen_port: option_env!("LISTEN_PORT")
            .unwrap_or("8080")
            .parse::<_>()
            .expect("LISTEN_PORT cannot parse port number"),
        log_dir: option_env!("LOG_DIR").unwrap_or("./log").to_string(),
        observable_interval: option_env!("OBSERVABLE_INTERVAL")
            .unwrap_or("0")
            .parse::<_>()
            .expect("OBSERVABLE_INTERVAL cannot parse interval seconds."),
    }
}

/// Windowsのログインユーザー情報
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SessionUser {
    user_name: String,
    session_name: String,
    id: u32,
    state: String,
    idle_time: String,
    login_time: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct UserClient {
    user: SessionUser,
    client_name: String,
}

/// アプリ設定
#[derive(Debug, Clone)]
struct AppConfig {
    listen_port: u16,
    listen_address: String,
    log_dir: String,
    observable_interval: u64,
}
