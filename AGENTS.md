# AGENTS.md — 實作者工作守則

> 給實作這個專案的 AI 助手。**每次開始工作前先讀完本檔**。

## 1. 開工流程（每個任務都一樣）
1. 讀 [docs/PROGRESS.md](docs/PROGRESS.md)，找到第一個未勾選的任務。
2. 讀 [docs/PLAN.md](docs/PLAN.md) 中該任務，以及它「需要閱讀」列出的章節（SPEC / RULES / INTERFACES）。
3. 只實作該任務的範圍。
4. 執行該任務的「驗收」指令，以及 `./scripts/check.sh`（T0.4 之後）。
5. 全部通過才在 PROGRESS.md 勾選，並寫一行筆記（做了什麼、遇到什麼、偏離規格之處）。
6. 回報給使用者：完成的任務編號、驗收結果、下一個任務。

## 2. 絕對規則
- **規格優先**：名稱、欄位、編碼順序照 [docs/INTERFACES.md](docs/INTERFACES.md)；規則照 [docs/RULES.md](docs/RULES.md)。不確定時**停下來問使用者**，不要自己發明。若真的必須偏離，先修改文件並在 PROGRESS 註明原因。
- **規則只寫在 `crates/uno-core`**。前端、伺服器、Python 不得重寫任何出牌合法性或計分邏輯。
- **不手寫跨語言重複型別**：TS 型別一律從 `apps/web/src/generated/`（ts-rs 產生）import。修改 Rust 型別後執行 `./scripts/gen-types.sh`。
- **引擎決定性**：`uno-core` 不得使用 `rand` crate、系統時間、`HashMap` 迭代順序（需要時用 `BTreeMap` 或 `Vec`）、浮點運算影響遊戲結果。
- **不要刪測試讓它通過**；不要用 `#[ignore]`、`.skip`、`as any`、`unwrap()` 在非測試程式碼中逃避問題（`uno-core` 中可以 `expect("不變式說明")` 用在確定不會發生的情況）。
- **不要一次改很多檔案而不跑測試**。每完成一個小步驟就 `cargo test -p <crate>` 或 `pnpm -C apps/web test`。
- **不要升級主版本或加入規格沒列出的大型依賴**（例如 UI 元件庫、圖表庫、狀態管理庫）。小工具型依賴可以，但要寫在 PROGRESS 筆記。
- 檔案不要超過約 600 行；超過就拆。
- 程式碼註解與 commit 訊息可用中文或英文，識別字用英文。

## 3. 常用指令
```bash
./scripts/setup.sh          # 安裝工具鏈與依賴
./scripts/check.sh          # 全部品質檢查
./scripts/gen-types.sh      # Rust → TS 型別
./scripts/build-wasm.sh     # 建置 WASM 到 apps/web/src/wasm/pkg
./scripts/dev.sh            # 開發模式（前端 5173 + 伺服器 8787）
./scripts/start.sh          # 正式啟動（手機掃 QR）
cargo test -p uno-core <filter>
pnpm -C apps/web test
python/.venv/bin/pytest python/tests -k <filter>
```

## 4. 常見陷阱
- `serde-wasm-bindgen` 預設把 `u64` 轉成 BigInt → 一律用 `Serializer::json_compatible()`。
- ts-rs 的 `u64`/`i64` 預設產生 `bigint` → 欄位加 `#[ts(type = "number")]`。
- Vite 中載入 wasm-pack 的 `--target web` 輸出：`import init, { WasmGame } from '@/wasm/pkg/uno_wasm'; await init();`。Worker 中也要各自 `init()`。
- 手機瀏覽器的 `100vh` 會被網址列影響 → 使用 `100dvh`。
- iOS Safari 音效需在使用者手勢後解鎖。
- `navigator.vibrate`、Wake Lock、Web Share 都不一定存在 → 一律先檢查。
- 非 HTTPS 下（區網 IP）Service Worker 不會註冊：這是預期行為，不是 bug。
- PyO3 長時間運算要 `py.allow_threads(|| ...)` 釋放 GIL。
- PyTorch 不支援系統的 Python 3.14 → Python 環境一律用 `uv venv --python 3.12`。
- Rust 測試中建立特定情境請用 `GameState::new_with_deck`（`testing` feature），不要硬改私有欄位。
