chcp 65001

cd /d %~dp0

set APP=.\query_user_agent_rs.exe
set LISTEN_ADDRESS=0.0.0.0
set LISTEN_PORT=9284

@REM run app
%APP%