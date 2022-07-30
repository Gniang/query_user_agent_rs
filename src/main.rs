use std::error::Error;

use actix_web::{get, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use windows::Win32::System::RemoteDesktop;
use windows::{core::PWSTR, Win32::Foundation::HANDLE};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !cfg!(target_os = "windows") {
        panic!("supported only windows")
    }

    let conf = read_app_config();
    HttpServer::new(|| App::new().service(api_users))
        .bind((conf.listen_address, conf.listen_port))?
        .run()
        .await
}

const WTS_CURRENT_SERVER_HANDLE: HANDLE = HANDLE(0);
#[allow(dead_code)]
const WTS_CURRENT_USER_SESSION_ID: u32 = 1;

#[tracing::instrument]
#[get("/api/users")]
async fn api_users() -> impl Responder {
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct UserClient {
        user: SessionUser,
        client_name: String,
    }

    let users = query_user().unwrap_or(vec![]);
    let user_clients: Vec<_> = users
        .into_iter()
        .map(|user| {
            let client_name = get_wts_session_info(user.id, RemoteDesktop::WTSClientName);
            UserClient { user, client_name }
        })
        .collect();

    web::Json(user_clients)
}

/// WTSQuerySessionInfoの問い合わせ結果を文字列で取得する
fn get_wts_session_info(session_id: u32, info_type: RemoteDesktop::WTS_INFO_CLASS) -> String {
    let mut result_byte_size: u32 = 0;
    let mut result_buffer: [u16; 8192] = [0; 8192];
    let result_txt = unsafe {
        let mut result_pwstr = PWSTR(result_buffer.as_mut_ptr());
        RemoteDesktop::WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            info_type,
            &mut result_pwstr,
            &mut result_byte_size,
        );
        if result_byte_size == 0 {
            return "".to_string();
        }

        String::from_utf16_lossy(std::slice::from_raw_parts(
            result_pwstr.0,
            (result_byte_size / 2 - 1) as usize,
        ))
    };
    result_txt
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

    let users: Vec<_> = lines
        // skip `active code page` message & headers
        .skip(2)
        .map(|line| {
            let items: Vec<_> = line.split("\t").collect();
            if items.len() > 4 {
                Some(SessionUser {
                    user_name: items.get(0)?.to_string(),
                    session_name: items.get(1)?.to_string(),
                    id: items.get(2)?.parse::<u32>().unwrap_or(0),
                    state: items.get(3)?.to_string(),
                    idle_time: items.get(4)?.to_string(),
                    login_time: items.get(5)?.to_string(),
                })
            } else {
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
            .parse::<u16>()
            .unwrap_or(8080),
    }
}

/// Windowsのログインユーザー情報
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionUser {
    user_name: String,
    session_name: String,
    id: u32,
    state: String,
    idle_time: String,
    login_time: String,
}

/// アプリ設定
#[derive(Debug)]
struct AppConfig {
    listen_port: u16,
    listen_address: String,
}
