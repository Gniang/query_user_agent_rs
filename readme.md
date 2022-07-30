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
  * 起動バッチ(`./query_user_agent_rs.cmd`)で設定を編集する 
    * リッスンアドレスとポートを指定する 
    * `0.0.0.0` = どこからでもアクセス可
2. タスクスケジューラに起動バッチを登録する

### リモートクライアントの確認

http://localhost:8080/api/users

（`localhost:8080`は設定や接続元に応じて適切に読み替えてください）

上記にURLアクセスすることで、サインイン中のユーザーごとのステータスを取得できます。

サンプルレスポンス

```jsonc
[
    // ユーザーごと
    {
        // query user の結果相当
        "user": {
            "userName": "test user",
            // console: ローカル利用状態, RDP...: リモート接続状態 
            "sessionName": "console",
            // セッションID
            "id": 1,
            // active: 利用中、 listen:
            "state": "Active",
            // 最後に操作してからの時間
            "idleTime": "58",
            "loginTime": "2022/07/25 22:50"
        },
        // 上記セッションIDのクライアント接続名。
        // リモート接続のときのみ値が設定される。
        "clientName": ""
    }
]
```

## 開発用

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

## その他

リモートデスクトップ用なのでWindowsでしか動きません。
