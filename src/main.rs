use actix_web::{get, web, App, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !cfg!(target_os = "windows") {
        panic!("supported only windows")
    }

    let conf = read_app_config();

    HttpServer::new(|| App::new().service(greet))
        .bind((conf.listen_address, conf.listen_port))?
        .run()
        .await
}

#[tracing::instrument]
#[get("/api/users")]
async fn greet() -> impl Responder {
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct UserAndClient {
        users: Vec<SessionUser>,
        client_name: String,
    }

    let users = query_user();
    let client_name = option_env!("CLIENTNAME")
        .unwrap_or("client name undefined")
        .to_string();
    web::Json(UserAndClient { users, client_name })
}

/// Windowsのログインユーザー情報一覧を取得
fn query_user() -> Vec<SessionUser> {
    use regex::Regex;
    use std::process::Command;

    let out = Command::new("cmd")
        .args(vec!["/c", "chcp 65001 && query user"])
        .output()
        .expect("command error");
    let text = String::from_utf8(out.stdout).expect("not utf8 string");
    dbg!(&text);

    let lf_text = text.replace("\r", "");
    // convert two more white space to tab
    let re = Regex::new(r"  +").expect("regex format err");
    let tsv_text = re.replace_all(&lf_text, "\t");
    let lines = tsv_text.split("\n");

    let users = lines
        // skip `active code page` message & headers
        .skip(2)
        .map(|line| {
            let items: Vec<_> = line.split("\t").collect();
            dbg!(line);
            if items.len() > 4 {
                Some(SessionUser {
                    user_name: items.get(0).unwrap().to_string(),
                    session_name: items.get(1).unwrap().to_string(),
                    id: items.get(2).unwrap().to_string(),
                    state: items.get(3).unwrap().to_string(),
                    idle_time: items.get(4).unwrap().to_string(),
                    login_time: items.get(5).unwrap().to_string(),
                })
            } else {
                None
            }
        })
        .flatten()
        .collect();

    return users;
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
    id: String,
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
