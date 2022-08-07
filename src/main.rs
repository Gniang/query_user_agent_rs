mod app_config;
mod app_data;
mod wts_array;
mod wts_wrapper;

use std::collections::HashMap;
use std::env;
use std::error::Error;

use actix_web::{cookie::time::Duration, get, web, App, HttpServer, Responder};
use app_config::AppConfig;
use app_data::{LoginEvent, ServerSessionAll, SessionInfoWithClient, SessionUser};
use itertools::Itertools;
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use windows::core::PWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::RemoteDesktop;
use wts_array::WtsArray;
use wts_wrapper::WtsSessionInfoW;

use crate::app_data::{LoginTrigger, UserClient};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if !cfg!(target_os = "windows") {
        panic!("supported only windows")
    }
    let conf = AppConfig::read_env();
    let _wg = init_logs(&conf);

    info!("Application init.");
    info!("{:?}", &conf);

    // // test
    // {
    //     loop {
    //         let a = query_sessions();
    //         for b in &a {
    //             let client_name = get_wts_session_info(b.session_id, RemoteDesktop::WTSClientName);
    //             println!("{:?}", client_name)
    //         }
    //     }
    // }

    // interval check
    {
        if (&conf).observable_interval > Duration::ZERO {
            let c = conf.clone();
            info!("observe start. interval:{:?}", &c.observable_interval);
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
    let client = awc::Client::default();
    let url = conf.slack_webhook_url;
    let mut interval = time::interval(conf.observable_interval);
    let mut last_clients = None;
    loop {
        debug!("observe session status checking...");

        let all = get_server_session_all();
        let clients: Vec<_> = all
            .sessions
            .into_iter()
            .filter(|s| s.client_name != "")
            .map(|s| UserClient {
                client_name: s.client_name.to_string(),
                server_user: s.server_user_name.to_string(),
            })
            .sorted()
            .collect();

        if let Some(last_clients) = last_clients {
            let login_events = create_login_events(&last_clients, &clients);
            if !login_events.is_empty() {
                info!(
                    "login evented. server:{} events:{:?}",
                    &all.server_name, login_events
                );
                let json = create_webhook_msg(&login_events, &all.server_name);
                let res = client.post(&url).send_body(json.to_string()).await;
                if res.is_err() {
                    error!("{:?}", res);
                }
            } else {
                debug!("no login events.");
            }
        }
        last_clients = Some(clients);
        debug!("observe session status check ended.");
        interval.tick().await;
    }
}

fn create_webhook_msg(login_events: &Vec<LoginEvent>, server_name: &str) -> serde_json::Value {
    let events_msg = login_events
        .iter()
        .map(|x| {
            let event = match x.trigger {
                LoginTrigger::Login => "Logined",
                LoginTrigger::Logout => "Logouted",
            };
            format!(
                "- **{}**  client:`{}` server_user:`{}`",
                event, x.client_name, x.server_user_name
            )
        })
        .join("\n");

    let text = format!(
        r#"
            server: {}
            
            {}
        "#,
        server_name, &events_msg,
    );
    serde_json::json!({ "text": text })

    // slack msg format reference
    //
    // {
    //     "text": "main text",
    //     "blocks": [
    //         {
    //             "type": "section",
    //             "text": {
    //                 "type": "mrkdwn",
    //                 "text": "Danny Torrence left the following review for your property:"
    //             }
    //         },
    //         {
    //             "type": "section",
    //             "block_id": "section567",
    //             "text": {
    //                 "type": "mrkdwn",
    //                 "text": "<https://example.com|Overlook Hotel> \n :star: \n Doors had too many axe holes, guest in room 237 was far too rowdy, whole place felt stuck in the 1920s."
    //             },
    //             "accessory": {
    //                 "type": "image",
    //                 "image_url": "https://is5-ssl.mzstatic.com/image/thumb/Purple3/v4/d3/72/5c/d3725c8f-c642-5d69-1904-aa36e4297885/source/256x256bb.jpg",
    //                 "alt_text": "Haunted hotel image"
    //             }
    //         },
    //         {
    //             "type": "section",
    //             "block_id": "section789",
    //             "fields": [
    //                 {
    //                     "type": "mrkdwn",
    //                     "text": "*Average Rating*\n1.0"
    //                 }
    //             ]
    //         }
    //     ]
    // }
}

///
fn create_login_events(old: &Vec<UserClient>, new: &Vec<UserClient>) -> Vec<LoginEvent> {
    debug!("old: {:?}", old);
    debug!("new: {:?}", new);
    if old == new {
        return vec![];
    }

    let old_g = old.iter().into_group_map_by(|&x| x);
    let new_g = new.iter().into_group_map_by(|&x| x);
    //
    let logouted = detect_chnged_core(&old_g, &new_g);
    let logined = detect_chnged_core(&new_g, &old_g);

    let login_events = logined.into_iter().map(|x| LoginEvent {
        trigger: LoginTrigger::Login,
        client_name: x.client_name.to_string(),
        server_user_name: x.server_user.to_string(),
    });

    let logout_events = logouted.into_iter().map(|x| LoginEvent {
        trigger: LoginTrigger::Logout,
        client_name: x.client_name.to_string(),
        server_user_name: x.server_user.to_string(),
    });

    login_events.chain(logout_events).collect()
}

fn detect_chnged_core<'a>(
    base: &'a HashMap<&UserClient, Vec<&UserClient>>,
    other: &HashMap<&UserClient, Vec<&UserClient>>,
) -> Vec<&'a UserClient> {
    base.iter()
        .filter(|&(&key, items)| {
            let same_other = other.get(key);
            let is_state_changed = match same_other {
                Some(other_items) => other_items.len() == items.len(),
                _ => true,
            };
            is_state_changed
        })
        .map(|(&key, _)| key)
        .collect()
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
            UserClient {
                server_user: user.user_name,
                client_name,
            }
        })
        .collect();

    web::Json(user_clients)
}

#[tracing::instrument(level = "info")]
#[get("/api/sessions")]
async fn api_sessions() -> impl Responder {
    info!("session info web api");
    let server_session_all = get_server_session_all();
    web::Json(server_session_all)
}

fn get_server_session_all() -> ServerSessionAll {
    let sessions = query_sessions();
    let session_with_clients: Vec<_> = sessions
        .into_iter()
        .map(|s| {
            let client = get_wts_session_info(s.session_id, RemoteDesktop::WTSClientName);
            let user_name = get_wts_session_info(s.session_id, RemoteDesktop::WTSUserName);
            SessionInfoWithClient {
                server_user_name: user_name,
                client_name: client,
                session: s,
            }
        })
        .collect();
    let server_name = env::var("COMPUTERNAME").unwrap_or("".to_string());
    ServerSessionAll {
        sessions: session_with_clients,
        server_name: server_name,
    }
}

const WTS_CURRENT_SERVER_HANDLE: HANDLE = HANDLE(0);
/// WTSQuerySessionInfoの問い合わせ結果を文字列で取得する
fn get_wts_session_info(session_id: u32, info_type: RemoteDesktop::WTS_INFO_CLASS) -> String {
    #[allow(dead_code)]
    const WTS_CURRENT_USER_SESSION_ID: u32 = 1;

    let result_txt = unsafe {
        let mut len: u32 = 0;
        let mut bytes = [0u16; 1024];
        // let pwstr = std::ptr::null_mut();
        let mut pwstr = PWSTR(bytes.as_mut_ptr());
        RemoteDesktop::WTSQuerySessionInformationW(
            WTS_CURRENT_SERVER_HANDLE,
            session_id,
            info_type,
            &mut pwstr,
            &mut len,
        );
        let result = if len == 0 {
            "".to_string()
        } else {
            String::from_utf16_lossy(std::slice::from_raw_parts(
                pwstr.0,
                // u8 -> u16 and last cstr \0 remove
                (len / 2 - 1) as usize,
            ))
        };
        RemoteDesktop::WTSFreeMemory(pwstr.as_ptr() as _);
        result
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
