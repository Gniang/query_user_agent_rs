use serde::{self, Deserialize, Serialize};
use windows::Win32::System::RemoteDesktop;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WtsSessionInfoW {
    pub session_id: u32,
    pub state: i32,
    pub win_station_name: String,
}

impl From<&RemoteDesktop::WTS_SESSION_INFOW> for WtsSessionInfoW {
    fn from(w: &RemoteDesktop::WTS_SESSION_INFOW) -> WtsSessionInfoW {
        let name = unsafe { w.pWinStationName.to_string().unwrap_or("".to_string()) };
        WtsSessionInfoW {
            session_id: w.SessionId,
            state: w.State.0,
            win_station_name: name,
        }
    }
}
