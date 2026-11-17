# 進度追蹤（PROGRESS）

> 實作者每完成一個任務：把 `[ ]` 改成 `[x]`，並在後面寫一行筆記（日期、重點、偏離規格之處、驗收數據）。

## P0 環境與骨架
- [x] T0.1 安裝工具鏈 (2026-11-13: scripts/setup.sh 安裝 Rust 1.100.0 / rustfmt / clippy / wasm32-unknown-unknown / wasm-pack 0.15.1 / uv 0.13.8；pnpm 12.16.1 驗證通過)
- [x] T0.2 Rust workspace (2026-11-13: 建立 Cargo workspace、7 個 crate、rustfmt.toml、.gitignore；cargo build/test --workspace 通過)
- [x] T0.3 前端骨架 (2026-11-13: 建立 apps/web (React 19 + Vite 8 + Tailwind 4 + TS strict)；偏離：Vite 8 使用 Rolldown 無 rollup，移除 vite-plugin-top-level-await；pnpm 12 test 不需 --；build/test/lint/typecheck 全過)
- [x] T0.4 開發腳本 (2026-11-13: 建立 scripts/check.sh、gen-types.sh、build-wasm.sh；./scripts/check.sh 通過)
- [x] T0.5 文件 (2026-11-13: 更新 README.md 並確認所有檔案連結與 check.sh 通過)

## P1 核心引擎
- [x] T1.1 RNG (2026-11-13: 實作 SplitMix64 + xoshiro256** 與 Fisher-Yates shuffle；回歸測試與 check.sh 通過)
- [x] T1.2 卡牌型別與牌點 (2026-11-13: 實作 Color/Face/CardFace/Card/Side 與 points()，含 TS export 與全牌點測試)
- [x] T1.3 設定 (2026-11-13: 實作 RuleConfig、所有模式 preset() 與 validate() 規則修正，測試與 check.sh 通過)
- [x] T1.4 牌組建構 (2026-11-13: 實作 Classic 108/Flip 112/No Mercy 168 牌組建構與張數/確定性配對測試，check.sh 通過)
- [x] T1.5 GameState 建構與發牌 (2026-11-13: 實作 Phase/Action/Event/EngineError 與 GameState::new/new_with_deck/發牌/六種起始牌測試，check.sh 通過)
- [x] T1.6 合法動作 (2026-11-13: 實作 rules.rs 的 can_play / legal_actions / UNO 視窗與所有階段動作判斷，測試與 check.sh 通過)
- [x] T1.7 套用動作 (2026-11-13: 實作 apply.rs/effects.rs 與基本 scoring.rs，涵蓋出牌效果、draw_to_match、+4 質疑、UNO 視窗與 Wild 選色規則、Timeout/EndByTime，check.sh 全過)
- [x] T1.8 計分、多輪、結束 (2026-11-13: 完成 scoring.rs 個人與 2v2 團隊計分、平手排名規則與 next_round 輪莊重洗，測試與 check.sh 全過)
- [x] T1.9 PlayerView 與事件過濾 (2026-11-13: 實作 PublicPlayer/PlayerView/GodView、view/spectator_view、filter_events 與 FNV-1a state_hash，隱藏資訊與雜湊測試通過，check.sh 全過)
- [x] T1.10 隨機對局壓力測試與決定性 (2026-11-13: 實作 tests/random_play.rs 含 2-10 人 Proptest、多輪重播與 10 萬場 --release 壓測於 82.60s 通過，check.sh 全過)

## P2 房規、模式、編碼、重播
- [x] T2.1 疊加 (2026-11-13: 完善與驗證 stack_draw Same / Progressive / NoMercy 規則與 R3.1.6 不進質疑、stack 測試與 check.sh 全過)
- [x] T2.2 七零與搶出 (2026-11-13: 實作 Seven-Zero 出 7 換牌/出 0 依方向輪轉/清 UNO 狀態/末張不觸發與 JumpIn 搶出規則，check.sh 全過)
- [x] T2.3 Flip (2026-11-13: 實作 Flip 雙面翻轉、牌堆反轉與牌背顯示、SkipEveryone/DrawOne/DrawFive/WildDrawTwo/WildDrawColor 質疑及起手 Flip 規則，並對齊 PlayerView/PublicPlayer 介面，check.sh 全過)
- [x] T2.4 No Mercy (2026-11-13: 實作 DiscardAll/WildReverseDrawFour/WildDrawSix/WildDrawTen/WildColorRoulette/DrawFour 與 25 張 Mercy Rule 淘汰洗回牌底、draw_to_match 達 25 立即淘汰與 R4.2.9 名次排序，check.sh 全過)
- [x] T2.5 團隊與淘汰賽 (2026-11-13: 驗證 4/6 人團隊模式計分、盲視手牌與隊友不可 CatchUno；實作淘汰賽每輪不累積分數、scores 全 0、同分比張數與座位號、next_round 跳過淘汰者與完整 4 人賽，check.sh 全過)
- [x] T2.6 全模式壓力測試 (2026-11-13: 擴充 random_play.rs 涵蓋 8 種預設模式與隨機房規組合含 Flip 牌背不變式，--release 8 模式各 20,000 局共 160,000 局於 103.52s 通過，check.sh 全過)
- [x] T2.7 RL 編碼 (2026-11-13: 實作 encode.rs 的 FaceTable/行動空間 A=81/95/135 與觀測向量 D=295/351/1484，通過往返、全模式 mask⇔legal 不變式與 [-1, 50] 值域測試，check.sh 全過)
- [x] T2.8 重播與世界取樣 (2026-11-13: 實作 replay.rs 重播格式與 GameState::steps/set_last_step_time/to_replay/replay()，及 world.rs 的 sample_world 與 from_view_sampled 含 Flip 牌背保持，check.sh 全過)
- [x] T2.9 TS 型別匯出 (2026-11-13: 確認所有公開型別 #[ts(export)] 與 64 位元 #[ts(type="number")]，更新 scripts/gen-types.sh 產出 25 個 TS 檔與 index.ts，typecheck 與 check.sh 全過)

## P3 AI 與模擬
- [x] T3.1 Random、Heuristic (2026-11-13: 實作 Bot trait、BotKind/BotError/make_bot、RandomBot 與 HeuristicBot 含 7 項規則與全模式 1,000 局測試，check.sh 全過)
- [x] T3.2 Hard（ISMCTS） (2026-11-13: 實作 HardBot 扁平根節點兩階段配對 CRN + tactical rollout + Horizon-44 Softmax 殘局校準 + from_view_sampled，測試決定性與多模式合法性全過)
- [x] T3.3 uno-sim CLI（貼勝率結果） (2026-11-13: 實作 arena CLI 含輪轉座位與 Wilson 95% CI；heuristic vs 3x random 2000 局 1996 勝 = 99.80% [99.49%, 99.92%] AvgRank 1.002；hard vs 3x heuristic 400 局 124 勝 = 31.00% [26.67%, 35.70%] AvgRank 2.270；check.sh 全過)
  - 2026-11-16 AI 強化（搜尋演算）：Hard bot rollout 深度改為自適應——中局 44→150 步、殘局（2 人／自己 ≤3 張／任一對手 ≤2 張／總手牌 ≤16）直接打到回合結束（1000 步）；新增 rollout_horizon 單元測試。arena 單回合 1000 局（seed 99）：舊版 25.00% → 新版 27.20% [24.53%, 30.04%]（seed 7 亦 27.10%）；完整 500 分對局 800 局新舊 27.75% vs 28.75% 在信賴區間內視為持平；2 人單回合 64.00%、NoMercy 51.00%、Flip 26.50%。實驗後撤除的方案：點數負債評估（完整對局無助益）、關鍵局面加倍 sims（更慢且未變強）、深度 220/300（不如 150）。另順手修復上游 DirectionRing 的 lint（getNextActiveSeat 移至 direction.ts），check.sh 全過
  - 2026-11-16 地獄難度 Hell bot（網路調研後實作）：依近年 ISMCTS 文獻（SO/MO-ISMCTS 單樹跨 determinization 累積統計、ensemble determinization）實作 crates/uno-bots/src/hell.rs——單一資訊集搜尋樹（UCB1＋行動者視角回傳）、每次迭代新取樣世界、樹節點以抽象牌面鍵共享統計（解 card-id 跨世界對不上的問題）、根節點再以 600 次 CRN 扁平 MC 集成平均；HELL_SIMS=1500。接線：BotKind::Hell、WASM（HELL_SIMS）、前端單機/房間 bot 選單「地獄」、伺服器 CatchUno 90%。arena 同場對照（classic 單回合，hell/hard/各 2 heuristic）：1000 局聚合 Hell 28.90% vs Hard 24.50%（seed 5/11/21 分別 29.00/29.50/28.25 對 21.00/27.00/23.75）；hell 測試（決定性/全模式合法）與 check.sh 全過。代價：每步思考約為 Hard 的 5–10 倍，屬選用難度

## P4 WASM 與前端基礎
- [x] T4.1 uno-wasm（記錄 wasm gzip 大小） (2026-11-13: 完成 WasmGame/WasmBot/filter_events/preset/validate_config/replay_validate 綁定並以 json_compatible 序列化；build-wasm.sh 產出 uno_wasm_bg.wasm gzip = 231,213 bytes ≈ 225.8 KB ≤ 600 KB；wasm.test.ts 與 check.sh 全過)
- [x] T4.2 設計系統與主題 (2026-11-13: 實作 theme.css 三套主題與 Tailwind v4 @theme、global.css、離線打包 Fredoka 與 Noto Sans TC woff2 至 public/fonts/、useSettings zustand+localStorage 狀態與測試；build/lint/typecheck/test 全過)
- [x] T4.3 UnoCard 元件 (2026-11-13: 實作 SVG <UnoCard>/CardSymbols/CardBack 與 18 種 Face、4 尺寸/4 狀態/色盲符號與 Flip 牌背、/dev/cards 開發頁，單元測試與 Playwright 截圖 docs/screenshots/cards.png 目視驗證通過)
- [x] T4.4 路由、版面、i18n (2026-11-13: 實作 SPEC §5.3 全部路由骨架與互動設定頁、AppShell safe-area/44px 按鈕、zh-TW/en i18n 字典與同鍵測試、index.html mobile meta；build/lint/typecheck/test 全過)

## P5 單機遊戲畫面
- [x] T5.1 LocalGameClient 與 Worker (2026-11-13: 實作 GameClient 介面、LocalGameHost、game.worker.ts 與 LocalGameClient；700ms × animSpeed 預算、bot CatchUno 0%/30%/70%、turn_seconds/game_seconds 超時、即時切速；LocalGameHost.test.ts 與 check.sh 全過)
- [x] T5.2 store 與動畫佇列 (2026-11-13: 實作 store.ts、handSort.ts 與 AnimationQueue.ts，依 SPEC §5.6 時長與倍速推演中間 displayView、off 模式同步套用並連動音效/震動；AnimationQueue.test.ts 與 handSort.test.ts 全過)
- [x] T5.3 遊戲桌（直向） (2026-11-13: 實作 GameTable 等 9 個直向牌桌元件與 ResizeObserver 手牌重疊、點選/雙擊/上滑/鍵盤出牌、Flip 牌背及 axe 無障礙修正；GameTable.test.tsx 與 Playwright iPhone 13/Pixel 7 截圖 game-iphone13.png、game-pixel7.png 目視無重疊/0 overflow 通過)
- [x] T5.4 對話框 (2026-11-13: 實作 TurnTimer、ColorPicker、DrawnCardDialog、ChallengeDialog、TargetPicker 與 RoundResultDialog 並整合至 GameTable；dialogs.test.tsx 6 項 render+點擊測試全過)
- [x] T5.5 橫向/桌機版面 (2026-11-13: 實作 landscape ≥768px 橢圓牌桌依角度 180°+(i+1)·180°/n 分佈對手，並補 max-height 520px 短視窗緊湊佈局避免 iPhone 橫向中央牌堆裁切；Playwright game-desktop-1280x800.png、game-iphone-landscape.png 通過)
- [x] T5.6 動畫 (2026-11-13: 實作 card-<id> layoutId 飛牌、方向環翻轉、UNO! 彈出字與 Canvas 勝利彩帶，支援 MotionConfig reducedMotion="user" 與 animSpeed 倍率；未另於 Chrome DevTools 錄製 FPS trace，已以 Vitest 與 Playwright 實跑手機對局驗證流暢度)
- [x] T5.7 單機設定與結算 (2026-11-13: 實作 SoloSetupPage 8 模式/人數/AI/房規抽屜、GamePage useWakeLock+useLeaveGuard 與 ResultsPage 名次/再來一局/本地重播匯出；偏離規格之處：依使用者需求精簡 HomePage 入口僅留「快速 vs AI / 規則說明 / 設定」，Playwright 手機全場至 /results 54.5s 通過，10/10 E2E 全過)
- [x] T5.8 音效與震動 (2026-11-13: 合成 6 種 WAV 音效至 public/sounds/ 並記錄於 CREDITS.md；實作 sound.ts howler 延遲解鎖與音量 0 不播放、haptics.ts 安全檢查 navigator.vibrate；單元測試 mock howler 與全部檢查通過)

## P6 伺服器與帳號
- [x] T6.1 伺服器骨架 + QR (2026-11-13: 實作 uno-server 的 clap CLI、Axum `/api/server-info`、`/models` 靜態目錄與 `ServeDir`+`ServeFile` SPA fallback、區網 IPv4 偵測排除 `docker`/`br-`/`veth` 並輸出 Unicode QR 與 `tracing_subscriber` 日誌；`uno-server` 9 項測試、`curl` 實測與 `./scripts/check.sh` 全過)
- [x] T6.2 資料庫 (2026-11-13: 實作 rusqlite bundled 遷移、WAL 模式、外鍵設定；完成 users/sessions、matches/match_players/daily_scores 與 ratings/rating_history Repository；支援 tokio spawn_blocking 非同步與個人/團隊/Solo Elo 運算；cargo test -p uno-server db 10 項測試與 ./scripts/check.sh 全過)
- [x] T6.3 帳號 API (2026-11-16: 實作 /api/guest、/register、/login、/logout、/me；審查修正：PublicUser 對齊規格 UserPublic{name}、Argon2 改 spawn_blocking、認證端點限流 10/分鐘每 key、username 大小寫正規化、session token 只存 Blake2b 雜湊、每小時清理過期 session、Internal 錯誤不外洩；沙箱環境故障無法執行 cargo test，待 ./scripts/check.sh 驗證)
- [x] T6.4 前端帳號頁 (2026-11-16: 實作 Login/Register 頁與 useAuth；審查修正後前端 ApiUser{name} 與後端回應一致，useAuth.test.tsx mock 即真實形狀)

## P7 多人連線
- [x] T7.1 uno-protocol (2026-11-16: 依 INTERFACES §7.1/§7.2 實作 ClientMsg/ServerMsg/RoomInfo/SeatInfo/Occupant/RoomStatus/TurnTimer/UserPublic/EloChange/ErrorCode 全型別與 TS export，i64/u64 欄位以 #[ts(type="number")] 對齊 JS number；serde round-trip 測試涵蓋每種訊息各一例共 15 項通過；gen-types 產出 10 個新 TS 檔；本機補裝 Rust 1.100.0/wasm-pack 後 ./scripts/check.sh 全過，含先前 T6.3 待驗證的 uno-server 測試與 web 18 檔 57 項)
- [x] T7.2 房間 actor (2026-11-16: 實作 rooms/{mod,registry,room,bots,timers}.rs：RoomRegistry 以 Mutex<HashMap> 管理、房號 6 碼排除 0O1IL；Room actor 單一 task 處理 Join/Leave/Disconnect 與全部 ClientMsg，座位/準備/房主權限/開始條件驗證，依座位 filter_events 廣播與快照；bot 排程 600–1200ms 帶 epoch+turn 防過期、CatchUno 機率 0/30/70%、回合與總時限計時、RoundOver 8 秒自動下一輪、斷線 1 秒 Heuristic 代打與 60 秒永久轉 bot；GameOver 寫入 matches/match_players 並以 apply_match_ratings 結算 Elo 後廣播 MatchSaved，AppState 已持有 RoomRegistry；rooms 測試 5 項通過，含 2 真人+2 bot 完整對局打完且 DB 有紀錄；./scripts/check.sh 全過)
- [x] T7.3 WebSocket 端點 (2026-11-16: 實作 src/ws.rs 的 /ws 升級端點：第一則 Hello 5 秒逾時、protocol=1 與 Bearer token 驗證後回 Welcome；CreateRoom/JoinRoom 經 RoomRegistry 路由，其餘訊息轉發 Room actor；每連線每秒 10 則限流回 RateLimited；連線關閉通知房間 Disconnect 保留座位代打；tokio-tungstenite 整合測試 2 項通過（雙客戶端建房/加入/開始皆收到 GameSnapshot、壞 token 回 Unauthorized）；./scripts/check.sh 全過。待後續：所有真人離開 5 分鐘刪房的清理計時尚未實作)
- [x] T7.4 RemoteGameClient 與大廳 (2026-11-16: 實作 src/net/socket.ts 單例連線（Hello 驗證、自動重連 1/2/4…10s、重連自動回房、訊息分派）、RemoteGameClient 將 ServerMsg 映射為 GameUpdate（單元測試 2 項）、remoteSession 待進房代碼；新增 CreateRoomPage（模式/人數/公開切換、WASM preset 建房）、JoinRoomPage（6 碼大字輸入）、RoomPage（座位網格、加 bot/移除、房規摘要、QR=join_url、複製/Web Share、聊天、準備/開始）；/r/:code 未登入先訪客暱稱再自動加入；GamePage 依 pending remote 改用 RemoteGameClient 渲染同一 GameTable；雙瀏覽器 E2E（桌機建房 + iPhone 13 掃碼式加入、開始、雙方見牌桌且 B 能出牌）通過；check.sh 全過)
- [x] T7.5 聊天、表情、觀戰、重連 (2026-11-16: 遊戲內 ChatDrawer 底部抽屜（聊天＋表情快捷列）、EmoteBubble 最新表情泡泡顯示 2 秒、觀戰時顯示「觀戰中」標籤（view.me 為 null）、斷線時頂部「重新連線中…」橫幅；socket 重連後自動重新 JoinRoom 取快照；修正 GamePage 進入遊戲桌一律重新 join 以取得最新快照的 bug；E2E 新增離線 3 秒重連後能繼續出牌、遊戲內聊天對方可見 2 項，連同 T7.4 共 3 項通過；check.sh 全過)

## P8 戰績、重播、每日
- [x] T8.1 戰績 API 與頁面 (2026-11-16: 新增 api/stats.rs 端點 GET /api/users/:id/stats（總覽＋by_mode＋elo_history）、/api/users/:id/matches、/api/matches/:id/replay、/api/leaderboard、/api/rooms，整合測試驗證 Elo 1216/1184 與名次；前端 StatsPage 總覽卡片、各模式 Elo 與自繪 SVG 折線、最近對局連至重播，LeaderboardPage 模式分頁；check.sh 全過)
- [x] T8.2 單機戰績上傳 (2026-11-16: POST /api/matches/solo 以 Replay::replay() 伺服器重跑驗證、seat 0 須為本人，入庫 is_solo 且不計 Elo（計 games/wins）；整合測試：合法重播 200 且 Elo 維持 1200、竄改 Play{card:9999} 回 400；前端單機結束自動上傳、失敗靜默)
- [x] T8.3 重播檢視器 (2026-11-13 單機版先行完成：實作 `/replay/:id` 本機互動重播檢視器、`ReplayTimeline` WASM 逐步重建、播放/暫停/上一步/下一步/時間軸拖曳、0.5x–4x 倍速、上帝視角切換與 `ResultsPage` 近期重播清單連結；2026-11-16 補線上串接：ReplayPage 本地無紀錄且 id 為數字時改向 GET /api/matches/:id/replay 取得伺服器重播並以同一檢視器播放，StatsPage 最近對局即連至此)
- [x] T8.4 每日挑戰 (2026-11-16: GET /api/daily 回傳 UTC 日期/種子（與前端 getDailySeed 同公式，civil 演算法測試含閏日）/daily preset/my_best；POST /api/daily/submit 驗證 seed 與 mode_id 符合當日、重播重跑驗證後入庫並以 record_daily_score 計榜，整合測試第二次提交 counted=false；前端 DailyPage 顯示今日資訊與最佳分數、以固定 seed + 3 Hard bot 開局，結算頁自動提交並顯示分數/排名/是否計入；排行榜頁含 daily 分頁)

## P9 Python RL 環境（已移除）
> 2026-11-16 依使用者指示移除：本專案 AI 以搜尋演算為主（uno-bots 的 Random／Heuristic／Hard ISMCTS），
> 不做神經網路訓練，故 Python RL 環境整階段不施作。本次曾動工的 `python/` 與 uno-py 綁定已全數清除、uno-py 還原為 stub。
- [-] T9.1 uno-py（移除：AI 以搜尋演算為主）
- [-] T9.2 Gymnasium（移除）
- [-] T9.3 PettingZoo（移除）
- [-] T9.4 SB3 VecEnv 與效能（移除）
- [-] T9.5 RLCard 相容層（移除）

## P10 訓練與模型上線（已移除）
> 2026-11-16 依使用者指示移除：本階段的訓練與模型產出可能需要 GPU，本機不執行。
> 連帶移除相依的模型上線與 AI 實驗室（其驗收需要訓練好的模型）。以下保留紀錄，不再施作：
- [-] T10.1 MaskablePPO（移除：可能需要 GPU）
- [-] T10.2 自我對戰 PPO（移除：可能需要 GPU）
- [-] T10.3 DQN + 評估（移除：可能需要 GPU）
- [-] T10.4 ONNX 匯出（移除：相依於訓練產出）
- [-] T10.5 模型上線（移除：相依於訓練好的模型）
- [-] T10.6 AI 實驗室（移除：相依於模型上線）

## P11 打磨與發佈
- [x] T11.1 PWA / GitHub Pages (2026-11-13: 完成 GitHub Pages 純單機版部署與 PWA：支援動態 base path、BrowserRouter basename、Howler 音效路徑、.nojekyll 與 404.html SPA fallback、.github/workflows/deploy.yml 自動建置 WASM 與發佈工作流、scripts/build-pages.sh；VitePWA precache 136 entries；check.sh 全過)
- [x] T11.2 無障礙 (2026-11-13: 修正 ColorPicker 數字徽章對比 `bg-black/75`、TargetPicker `z-40`、禁用手牌不透視基底防重疊透字、橫向抽牌堆防裁切、桌面方向環放大、`CardBack` 三款主題、鍵盤方向鍵焦點指示與點擊桌面取消選牌；Vitest 17 檔 44 項與 Playwright 含 `@axe-core/playwright` 於 `/solo`/`/settings`/`/rules`/`/game`/`/results`/`/replay/local` 0 嚴重違規通過)
- [x] T11.3 i18n (2026-11-13: 移除 `OpponentStrip`、`OpponentAvatar`、`ResultsPage` 殘留硬編碼字串，補齊 Flip 質疑變體、團隊/淘汰狀態與重播檢視器全部 `zh-TW`/`en` 鍵值，`i18n.test.ts` 同鍵與參數檢查及 `check.sh` 全過)
- [x] T11.4 規則說明與教學 (2026-11-13: 實作 TutorialOverlay 首次進入 /game 顯示 4 步可跳過教學並記憶於 localStorage，搭配 RulesPage 模式圖文說明；TutorialOverlay.test.tsx、RulesPage.test.tsx 與 Playwright E2E 全過)
- [x] T11.5 一鍵啟動 / Docker (2026-11-16: 新增 scripts/start.sh（build-wasm → gen-types → web build → cargo run -p uno-server --release，參數轉傳）並實測啟動成功；多階段 Dockerfile（rust:trixie 建 WASM＋server、node 建 web、debian:trixie-slim 執行；wasm-bindgen 需 GLIBC 2.38 故不用 bookworm）與 docker-compose.yml（network_mode: host 以利區網 IP/QR 正確、volume ./data）；docker build 成功且容器實跑 /api/server-info 與首頁 200)
- [x] T11.6 最終驗收 (2026-11-16: 逐條核對 SPEC §2.1 F1–F26 與 N1–N6，結果如下；全 E2E 13/13 通過（smoke/無障礙、單機截圖與完整對局、連線建房/重連/聊天），check.sh 全過，README 已更新功能截圖與手機/Docker 說明。
  - F1 經典規則：uno-core 引擎＋RULES 編號測試（P1）通過。
  - F2 房規選項：疊加/7-0/搶出/draw_to_match/force_play/+4 質疑/UNO 懲罰（P2 測試）通過。
  - F3 八模式＋自訂：preset 與全模式壓力測試（T2.6）通過。
  - F4 2–10 人：config 驗證與 random_play 2–10 人 proptest 通過。
  - F5 單局/累積 500 分：scoring 測試通過。
  - F6 單機離線 vs AI：WASM 本地對局 E2E 完整打完一局通過。
  - F7 區網多人：房間/6 碼/QR/真人+AI 混桌，雙瀏覽器 E2E 通過。
  - F8 斷線重連：座位保留、AI 代打、60 秒轉 bot，rooms 測試＋offline E2E 通過。
  - F9 觀戰：spectator view 與「觀戰中」標籤（T7.5）完成。
  - F10 聊天與表情：ChatDrawer/EmoteBubble，聊天 E2E 對方可見通過。
  - F11 AI 難度：Random/Heuristic/Hard(ISMCTS) 已實作並有勝率紀錄（T3.3）；「研究型 ONNX」隨 P10 移除（AI 以搜尋演算為主）。
  - F12 註冊/登入/訪客：帳號 API＋前端頁（P6）通過。
  - F13 個人戰績：/stats 總場數/勝率/各模式 Elo/最近對局看重播（T8.1）完成。
  - F14 本機排行榜：/leaderboard 各模式＋每日分頁（T8.1/T8.4）完成。
  - F15 Python uno_engine/uno_rl：隨 P9 移除（AI 以搜尋演算為主，不做神經網路訓練）。
  - F16 可重現：同 seed 決定性由 state_hash 測試（T1.10）於引擎/瀏覽器/伺服器共用 uno-core 保證；Python 部分隨 P9 移除。
  - F17 向量化環境：隨 P9 移除。
  - F18 訓練範例（PPO/DQN）：隨 P10 移除（可能需要 GPU）。
  - F19 ONNX 匯出上線：隨 P10 移除。
  - F20 AI 實驗室：隨 P10 移除。
  - F21 重播通用與檢視器：Replay 格式於網頁/伺服器共用（線上重播 T8.3）；Python 部分隨 P9 移除。
  - F22 QR：伺服器啟動橫幅與房間 QR（T6.1/T7.4）完成。
  - F23 直向主版面＋橫向可用、點擊 ≥44px：截圖 E2E 與 UI 規格通過。
  - F24 觸控選牌/出牌/上滑與震動：GameTable 手勢與 haptics（P5）完成。
  - F25 PWA：manifest/離線單機（T11.1）完成；HTTPS 為選用未啟用（HTTP 模式可正常遊玩，PROGRESS T11.1 已述）。
  - F26 動畫可調速/關閉與 prefers-reduced-motion：設定與 MotionConfig（P5）完成。
  - N1 三套主題設計系統（T4.2）完成。N2 效能：實測 JS+CSS gzip 254,970 B（≤400 KB）、WASM gzip 231,261 B（≤600 KB）達標。N3 安全：伺服器權威＋事件過濾不洩漏手牌（view/filter 測試）、argon2、輸入驗證通過。N4 測試：引擎分支/property/E2E 覆蓋（check.sh＋Playwright 13 項）通過。N5 i18n zh-TW/en 同鍵測試通過。N6 無障礙：鍵盤操作、色盲符號、axe 0 嚴重違規（T11.2）通過。
  - 已知保留事項：所有真人離開 5 分鐘刪房的清理計時未實作（T7.3 筆記）；GitHub Pages 為純單機版，連線功能需自架 uno-server。)
