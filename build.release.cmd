chcp 65001

cd /d %~dp0

cargo build --release

del /Q .\query_user_agent.zip
powershell -NoProfile -ExecutionPolicy Unrestricted ".\archive.ps1"
