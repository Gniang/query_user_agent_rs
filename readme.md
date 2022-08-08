# Responding Windows Remote Client Agent

Windowsのリモート接続しているクライアント端末情報を返すエージェント（単機能のRestAPIサーバ）

複数人でリモート操作したいとき、誰がサーバーにログインしているのかをわかるようにする。

セキュリティは考慮しておらず、LAN内での利用を想定。
（リモートデスクトップの情報を扱う物なので）

## 使い方

### 起動方法

サーバー側の端末で以下セットアップを行う

1. releaseから`query_user_agent.zip`をダウンロードする
  * 適当なフォルダに解凍する
2. タスクスケジューラに起動バッチを登録する

### リモートクライアントの確認

http://localhost:9284/api/sessions

（`localhost:9284`は設定や接続元に応じて適切に読み替えてください）

上記にURLアクセスすることで、ログインセッションごとのステータスを取得できます。

また`CHAT_WEBHOOK_URL`と`OBSERVABLE_INTERVAL`を設定していると、
指定のSlack互換チャットにログイン/ログアウトを通知可能

## 開発用


### 前提

OpenSSLのパッケージが必要

```powershell
# 適当なディレクトリを作成し、そこに移動する
cd "c:\dev"
# vspkg（C++用パッケージ管理ツール)をインストール(別途実施済であれば不要)
git clone https://github.com/Microsoft/vcpkg 
cd vcpkg
.\bootstrap-vcpkg.bat
# === システムの詳細設定から`c:\dev\vcpkg`をPATHに追加する ===
# === ターミナル再起動 ===

# OpenSSL関連のパッケージをインストール
vcpkg install openssl-windows:x64-windows
vcpkg install openssl:x64-windows-static
vcpkg integrate install

# === システムの詳細設定から環境変数を設定する ===
# * `VCPKGRS_DYNAMIC`を追加し、`1`を設定する
# * `RUSTFLAGS`を追加し、`-Ctarget-feature=+crt-static`を設定する
#   * OpenSSLを静的リンクするよう設定する
#   * 動的リンクだとインストール先PCにopenSSLのDLLが必要になる

# ルート証明書の設定
# 配置先ディレクトリを用意する
cd "c:\openssl"
# * https://curl.se/docs/caextract.html から最新のCA証明書をDLする
# * 上記フォルダに`cacert.pem`の名称で配置する

# === システムの詳細設定から環境変数を設定する ===
# * `SSL_CERT_FILE`を追加し、`c:\openssl\cacert.pem`(上記ファイル）を設定する
# === ターミナル再起動 ===
```

参考資料  
https://stackoverflow.com/questions/55912871/how-to-work-with-openssl-for-rust-within-a-windows-development-environment

https://github.com/sfackler/rust-openssl/tree/5948898e54882c0bedd12d87569eb4dbee5bbca7#acquiring-root-certificates

Windowsはめんどう(´・ω・`)

### ビルド

```powershell
# デバッグビルド
cargo build

# リリースビルド
cargo build --release

# ホットリロードモード
cargo watch -x 'run --bin query_user_agent_rs'

# 配布データ作成(リリースビルド後)
Compress-Archive -Path `
    '.\target\release\query_user_agent_rs.exe', '.\query_user_agent_rs.cmd' `
    -DestinationPath '.\query_user_agent.zip'
```

### 環境変数

実行場所と同じ位置の.envファイルもしくは環境変数で設定を上書き可能

詳細は`./doc/.env`を参照

## その他

リモートデスクトップ用なのでWindowsでしか動きません。
