//! 房間計時排程輔助：以 `tokio::spawn` + `sleep` 在到期後送回內部命令。
//! 測試使用 `tokio::time::pause()` 時，`sleep` 會隨虛擬時間加速。

use std::time::Duration;

use tokio::sync::mpsc;

use super::RoomCommand;

/// 在 `delay` 之後把 `cmd` 送回房間 actor；通道已關閉時靜默丟棄。
pub fn schedule_after(tx: mpsc::Sender<RoomCommand>, delay: Duration, cmd: RoomCommand) {
    if delay.is_zero() {
        let _ = tx.try_send(cmd);
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        let _ = tx.send(cmd).await;
    });
}

/// 目前的伺服器 epoch 毫秒（供 TurnTimer deadline 與 ChatMsg 時間）。
pub fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
