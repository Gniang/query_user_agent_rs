use serde::{Deserialize, Serialize};

use crate::wts_wrapper::WtsSessionInfoW;

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub enum LoginTrigger {
    Login,
    Logout,
}

/// ログイン・ログアウトのイベント
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginEvent {
    pub trigger: LoginTrigger,
    pub server_user_name: String,
    pub client_name: String,
}

/// ログイン先のサーバーのユーザー名とリモート接続クライアント名
#[derive(Debug, PartialEq, Eq, Serialize, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UserClient {
    pub server_user: String,
    pub client_name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerSessionAll {
    pub server_name: String,
    pub sessions: Vec<SessionInfoWithClient>,
}

/// Windowsのログインユーザー情報
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionUser {
    pub user_name: String,
    pub session_name: String,
    pub id: u32,
    pub state: String,
    pub idle_time: String,
    pub login_time: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfoWithClient {
    pub server_user_name: String,
    pub client_name: String,
    pub session: WtsSessionInfoW,
}
