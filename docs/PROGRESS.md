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
- [ ] T6.3 帳號 API
- [ ] T6.4 前端帳號頁

## P7 多人連線
- [ ] T7.1 uno-protocol
- [ ] T7.2 房間 actor
- [ ] T7.3 WebSocket 端點
- [ ] T7.4 RemoteGameClient 與大廳
- [ ] T7.5 聊天、表情、觀戰、重連

## P8 戰績、重播、每日
- [ ] T8.1 戰績 API 與頁面
- [ ] T8.2 單機戰績上傳
- [ ] T8.3 重播檢視器 (2026-11-13 單機版先行完成：實作 `/replay/:id` 本機互動重播檢視器、`ReplayTimeline` WASM 逐步重建、播放/暫停/上一步/下一步/時間軸拖曳、0.5x–4x 倍速、上帝視角切換與 `ResultsPage` 近期重播清單連結；線上 `/api/replays/:id` 串接待 P8 實作)
- [ ] T8.4 每日挑戰

## P9 Python RL 環境
- [ ] T9.1 uno-py
- [ ] T9.2 Gymnasium
- [ ] T9.3 PettingZoo
- [ ] T9.4 SB3 VecEnv 與效能（記錄 steps/s）
- [ ] T9.5 RLCard 相容層

## P10 訓練與模型上線
- [ ] T10.1 MaskablePPO
- [ ] T10.2 自我對戰 PPO
- [ ] T10.3 DQN + 評估
- [ ] T10.4 ONNX 匯出
- [ ] T10.5 模型上線
- [ ] T10.6 AI 實驗室

## P11 打磨與發佈
- [x] T11.1 PWA / GitHub Pages (2026-11-13: 完成 GitHub Pages 純單機版部署與 PWA：支援動態 base path、BrowserRouter basename、Howler 音效路徑、.nojekyll 與 404.html SPA fallback、.github/workflows/deploy.yml 自動建置 WASM 與發佈工作流、scripts/build-pages.sh；VitePWA precache 136 entries；check.sh 全過)
- [x] T11.2 無障礙 (2026-11-13: 修正 ColorPicker 數字徽章對比 `bg-black/75`、TargetPicker `z-40`、禁用手牌不透視基底防重疊透字、橫向抽牌堆防裁切、桌面方向環放大、`CardBack` 三款主題、鍵盤方向鍵焦點指示與點擊桌面取消選牌；Vitest 17 檔 44 項與 Playwright 含 `@axe-core/playwright` 於 `/solo`/`/settings`/`/rules`/`/game`/`/results`/`/replay/local` 0 嚴重違規通過)
- [x] T11.3 i18n (2026-11-13: 移除 `OpponentStrip`、`OpponentAvatar`、`ResultsPage` 殘留硬編碼字串，補齊 Flip 質疑變體、團隊/淘汰狀態與重播檢視器全部 `zh-TW`/`en` 鍵值，`i18n.test.ts` 同鍵與參數檢查及 `check.sh` 全過)
- [x] T11.4 規則說明與教學 (2026-11-13: 實作 TutorialOverlay 首次進入 /game 顯示 4 步可跳過教學並記憶於 localStorage，搭配 RulesPage 模式圖文說明；TutorialOverlay.test.tsx、RulesPage.test.tsx 與 Playwright E2E 全過)
- [ ] T11.5 一鍵啟動 / Docker
- [ ] T11.6 最終驗收
