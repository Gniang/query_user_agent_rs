use std::{env, time::Duration};

// アプリ設定
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// 待ち受けポート番号
    pub listen_port: u16,
    /// 待ち受けアドレス "0.0.0.0"で全許可
    pub listen_address: String,
    /// ログ出力先
    pub log_dir: String,
    /// Debug,Info,Errorなど
    pub log_level: String,
    /// ログイン状態監視間隔（秒）
    pub observable_interval: Duration,
    /// ログイン状態変更通知先URL
    pub chat_webhook_url: String,
    /// ログイン状態変更通知先チャネル（mattermost専用）
    pub chat_webhook_channel: String,
    /// 代替サーバー名
    pub server_name: String,
}

impl AppConfig {
    /// 環境変数からアプリ設定を読込
    pub fn read_env() -> AppConfig {
        // .env < 環境変数 の優先順

        // .env ファイルからの環境変数読込
        let addr_dot = dotenv::var("LISTEN_ADDRESS").ok();
        let port_dot = dotenv::var("LISTEN_PORT").ok();
        let log_dir_dot = dotenv::var("LOG_DIR").ok();
        let log_level_dot = dotenv::var("LOG_LEVEL").ok();
        let interval_dot = dotenv::var("OBSERVABLE_INTERVAL").ok();
        let webhook_url_dot = dotenv::var("CHAT_WEBHOOK_URL").ok();
        let webhook_channel_dot = dotenv::var("CHAT_WEBHOOK_CHANNEL").ok();
        let server_name_dot = dotenv::var("SERVER_NAME").ok();

        let webhook_url = env::var("CHAT_WEBHOOK_URL")
            .unwrap_or(webhook_url_dot.unwrap_or("".to_string()))
            .to_string();
        let channel = env::var("CHAT_WEBHOOK_CHANNEL")
            .unwrap_or(webhook_channel_dot.unwrap_or("".to_string()))
            .to_string();

        // サーバー名が設定されていればそれを。設定されていなければWindowsの環境変数であるコンピューター名を取得する。
        let computer_name = env::var("COMPUTERNAME").unwrap_or("".to_string());
        let server_name = env::var("SERVER_NAME")
            .unwrap_or(server_name_dot.unwrap_or(computer_name))
            .to_string();

        // webhookのURLが設定されているときだけ監視は有効とする
        let interval: u64 = if webhook_url == "" {
            0 // no work
        } else {
            env::var("OBSERVABLE_INTERVAL")
                .unwrap_or(interval_dot.unwrap_or("120".to_string()))
                .parse::<_>()
                .expect("OBSERVABLE_INTERVAL cannot parse interval seconds.")
        };     

        AppConfig {
            listen_address: env::var("LISTEN_ADDRESS")
                .unwrap_or(addr_dot.unwrap_or("0.0.0.0".to_string()))
                .to_string(),
            listen_port: env::var("LISTEN_PORT")
                .unwrap_or(port_dot.unwrap_or("9284".to_string()))
                .parse::<_>()
                .expect("LISTEN_PORT cannot parse port number"),
            log_dir: env::var("LOG_DIR")
                .unwrap_or(log_dir_dot.unwrap_or("./log".to_string()))
                .to_string(),
            log_level: env::var("LOG_LEVEL")
                .unwrap_or(log_level_dot.unwrap_or("Debug".to_string()))
                .to_string(),
            observable_interval: Duration::from_secs(interval),
            chat_webhook_url: webhook_url,
            chat_webhook_channel: channel,
            server_name: server_name
        }
    }
}
