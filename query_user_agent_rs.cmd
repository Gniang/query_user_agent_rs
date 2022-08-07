chcp 65001

cd /d %~dp0

set APP=.\query_user_agent_rs.exe
set LISTEN_ADDRESS=0.0.0.0
set LISTEN_PORT=9284
@REM for slack
@REM   doc https://api.slack.com/apps/A03SNNMR5NF/incoming-webhooks?
@REM   eg. https://hooks.slack.com/services/XXX/YYY/ZZZ
@REM for mattermost
@REM   doc https://docs.mattermost.com/developer/webhooks-incoming.html
@REM   eg. http://{your-mattermost-site}/hooks/XXX
set SLACK_WEBHOOK_URL=""

@REM run app
%APP%