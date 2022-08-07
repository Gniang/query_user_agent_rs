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
    pub slack_webhook_url: String,
}

impl AppConfig {
    /// 環境変数からアプリ設定を読込
    pub fn read_env() -> AppConfig {
        // デバッグ用の環境変数読込
        let log_level_dot = dotenv::var("LOG_LEVEL").ok();
        let interval_dot = dotenv::var("OBSERVABLE_INTERVAL").ok();
        let webhook_url_dot = dotenv::var("SLACK_WEBHOOK_URL").ok();
        let port = dotenv::var("LISTEN_PORT").ok();

        let webhook_url = env::var("SLACK_WEBHOOK_URL")
            .unwrap_or(webhook_url_dot.unwrap_or("".to_string()))
            .to_string();

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
                .unwrap_or("127.0.0.1".to_string())
                .to_string(),
            listen_port: env::var("LISTEN_PORT")
                .unwrap_or(port.unwrap_or("9284".to_string()))
                .parse::<_>()
                .expect("LISTEN_PORT cannot parse port number"),
            log_dir: env::var("LOG_DIR")
                .unwrap_or("./log".to_string())
                .to_string(),
            log_level: env::var("LOG_LEVEL")
                .unwrap_or(log_level_dot.unwrap_or("Debug".to_string()))
                .to_string(),
            observable_interval: Duration::from_secs(interval),
            slack_webhook_url: webhook_url,
        }
    }
}
