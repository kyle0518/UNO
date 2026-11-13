# UNO 規則規格（引擎唯一依據）

> 本檔是 `uno-core` 的**唯一**規則來源。若與網路資料不同，**以本檔為準**。
> 標記「〔本專案定義〕」的條目是官方規則模糊或我們刻意簡化之處，請照寫，不要自行改。
> 每一條規則都有編號（例如 `R2.4.1`），測試名稱請包含編號，例如 `fn r2_4_1_reverse_acts_as_skip_with_two_players()`。

---

## 1. 名詞

| 名詞 | 意義 |
|------|------|
| 座位 seat | 0..n-1，固定不變（換手牌不換座位） |
| 方向 direction | `+1` 順時針（座位遞增）、`-1` 逆時針 |
| 下一位 next(p) | 從 p 依方向找下一個**仍在遊戲中**（未淘汰）的座位 |
| 目前顏色 active color | 棄牌堆頂的有效顏色（萬用牌為被指定的顏色） |
| 累積抽牌 pending draw | 疊加規則下尚未被抽的張數 |
| 回合 turn | 一位玩家從開始行動到輪到下一位為止 |
| 輪 round | 從發牌到有人出完牌（或被判定結束）為止 |
| 局 game | 由一或多輪組成，直到達成勝利條件 |

---

## 2. 經典規則（Classic）

### R2.1 牌組（108 張）
- R2.1.1 每種顏色（紅、黃、綠、藍）：數字 `0` ×1、數字 `1`–`9` 各 ×2、Skip ×2、Reverse ×2、Draw Two ×2 → 每色 25 張，共 100 張。
- R2.1.2 黑色：Wild ×4、Wild Draw Four ×4。
- R2.1.3 每張牌有唯一 `CardId`（u16，建牌時依上述順序從 0 開始編號：紅→黃→綠→藍→黑，每色內依 0,1,1,2,2,…,9,9,Skip,Skip,Reverse,Reverse,DrawTwo,DrawTwo）。
- R2.1.4 `deck_count`（房規）> 1 時，複製整副牌，CardId 接續編號。

### R2.2 開局
- R2.2.1 第 1 輪莊家 `dealer = 0`；之後每輪莊家 = 上一輪莊家的下一位（依座位遞增，跳過已淘汰者）。
- R2.2.2 用 `Rng` 洗牌（Fisher–Yates，見 INTERFACES §1.4）。
- R2.2.3 從莊家的下一位開始，依座位遞增輪流發牌，每人 `hand_size` 張（預設 7）。
- R2.2.4 翻開牌堆頂一張作為棄牌堆第一張（起始牌）。
- R2.2.5 方向初始為 `+1`，第一位行動者 `first = next(dealer)`。
- R2.2.6 起始牌效果：
  - 數字牌：無效果，`first` 開始。
  - Skip：`first` 被跳過，由 `next(first)` 開始。
  - Reverse：方向變 `-1`，由**莊家**開始〔官方〕；2 人時等同 Skip（由莊家開始也就是跳過對手，結果相同）。
  - Draw Two：`first` 抽 2 張並被跳過，由 `next(first)` 開始。（疊加規則開啟時也一樣，不可疊。）
  - Wild：由 `first` 選色（進入 `ChooseColor` 階段），選完後由 `first` 出牌。
  - Wild Draw Four：放回牌堆、重新洗牌、重新翻，直到不是 Wild Draw Four。

### R2.3 出牌合法性
- R2.3.1 一張有色牌可以出，若「顏色 = 目前顏色」或「與頂牌同數字（數字牌）」或「與頂牌同種類（功能牌，如 Skip 對 Skip）」。
- R2.3.2 Wild 任何時候都可以出（`pending_draw > 0` 時除外，見 R3.1）。
- R2.3.3 Wild Draw Four 依 `wild4_rule`：
  - `Challenge`（預設，官方）：任何時候都可以出（可以虛張），下一位可質疑（R2.6）。
  - `Strict`：只有手上**沒有**與目前顏色相同的牌時才可以出；不能質疑。
  - `Free`：任何時候都可以出；不能質疑。
- R2.3.4 玩家即使有可出的牌，也可以選擇抽牌〔官方〕。

### R2.4 功能牌效果
- R2.4.1 Skip：下一位被跳過。2 人時 = 出牌者再出一次。
- R2.4.2 Reverse：方向反轉；2 人時 = Skip。
- R2.4.3 Draw Two：下一位抽 2 張並被跳過（疊加規則見 R3.1）。
- R2.4.4 Wild：出牌者選色（`ChooseColor` 階段），之後輪到下一位。
- R2.4.5 Wild Draw Four：出牌者選色 → 若 `wild4_rule = Challenge` 且未開疊加，下一位進入 `Challenge` 階段（R2.6）；否則下一位抽 4 張並被跳過。

### R2.5 抽牌
- R2.5.1 玩家選擇 `Draw`：抽 1 張。
- R2.5.2 若抽到的牌可以出（依 R2.3），進入 `DrawnCardDecision` 階段：玩家可以「出這張」(`Play` 該張) 或 `Pass`（保留）。若 `force_play = true`，則只能出。
- R2.5.3 若抽到的牌不能出，回合直接結束（不需 Pass）。
- R2.5.4 `draw_to_match = true` 時：一直抽到抽出可出的牌為止，再進入 R2.5.2。若牌堆與棄牌堆都抽乾仍沒有可出的牌，回合結束。
- R2.5.5 牌堆空時：保留棄牌堆頂牌，其餘洗牌成新牌堆（萬用牌恢復成無色）。若仍無牌可抽，則抽 0 張。
- R2.5.6 若玩家沒有可出的牌且牌堆與棄牌堆都無牌可抽，合法動作只有 `Pass`。
- R2.5.7 「被罰抽 k 張」（Draw Two、+4、UNO 懲罰…）一律逐張抽，中途牌堆空依 R2.5.5 處理，抽不到的部分忽略。罰抽不會觸發 `DrawnCardDecision`。

### R2.6 Wild Draw Four 質疑
- R2.6.1 `Challenge` 階段由被 +4 的玩家（受害者）決定：`Challenge` 或 `Accept`。
- R2.6.2 `Accept`：受害者抽 4 張並被跳過。
- R2.6.3 `Challenge`：檢查出 +4 者**出牌當下**（出 +4 之前）手上是否有「顏色 = 出 +4 前的目前顏色」的牌（萬用牌不算）。
  - 有（虛張成立）：出牌者抽 4 張；受害者**不**抽、**不**被跳過，輪到受害者正常出牌。
  - 沒有（質疑失敗）：受害者抽 6 張並被跳過。
- R2.6.4 無論結果，出牌者選的顏色都維持。
- R2.6.5 引擎在 +4 出牌時記錄 `wild4_was_legal: bool` 於狀態中（不在 `PlayerView` 中公開）。

### R2.7 UNO 宣告
- R2.7.1 `uno_rule = Manual`（預設）：
  - 輪到自己、手上剩 2 張、且有可出的牌時，可以先做 `CallUno`（不結束回合），再出牌。也可以在 `DrawnCardDecision` 階段且手上剩 2 張時 `CallUno`。
  - 出牌後手上剩 1 張且**沒有**先宣告 → 開啟「UNO 視窗」：`uno_window = Some(seat)`。
  - 視窗開啟期間：其他任何玩家可做 `CatchUno { target: seat }` → 該玩家罰抽 `uno_penalty` 張（預設 2），視窗關閉；該玩家本人可做 `CallUno` → 視窗關閉、不罰。
  - 視窗在「**目標玩家以外**的玩家做出會改變輪次的動作」被套用時自動關閉（也就是之後的玩家做了 `Play`/`Draw`/`Pass`/`Challenge`/`Accept`/`ChooseColor`/`ChooseTarget`）。目標玩家自己出萬用牌後的 `ChooseColor`、出 7 後的 `ChooseTarget` **不會**關閉視窗。`CatchUno` 與 `CallUno` 本身不改變輪次。若目標玩家因 Skip/2 人 Reverse 等原因再次行動，視窗在他做出下一個 `Play`/`Draw` 時關閉。
  - 〔本專案定義〕回合制宿主（Python AEC 環境）中，只有「目前輪到的玩家」能做 `CatchUno`；即時宿主（伺服器、單機網頁）中任何非目標玩家都能做。
- R2.7.2 `uno_rule = Auto`：出到剩 1 張時自動視為已宣告；`CallUno`、`CatchUno` 永遠不合法。（RL 預設可用此設定簡化動作空間。）
- R2.7.3 玩家手上剩的牌數因被罰抽而 ≥ 2 時，其 UNO 已宣告狀態清除。

### R2.8 出完牌與一輪結束
- R2.8.1 玩家出完最後一張牌即贏得此輪。最後一張若是功能牌，效果仍然生效（Draw Two / +4 的抽牌會發生，影響計分）；+4 最後一張時不進入質疑階段，直接抽 4 張。
- R2.8.2 最後一張若是萬用牌，仍需選色（為了事件完整性），選完後才結束此輪。〔本專案定義〕
- R2.8.3 一輪結束時產生 `RoundEnded` 事件，含每人手牌點數與本輪得分。

### R2.9 計分
- R2.9.1 牌點：數字牌 = 面值；Skip、Reverse、Draw Two = 20；Wild、Wild Draw Four = 50。
- R2.9.2 `scoring = Points`（預設 500）：本輪贏家得到其他所有人手牌點數總和；累計 ≥ `target_score` 者贏得整局。未達則開始新一輪（R2.2，莊家輪替）。
- R2.9.3 `scoring = SingleRound`：第一個出完牌的人贏得整局；名次依剩餘手牌點數由小到大，同分比張數少，再同分比座位號小。
- R2.9.4 最終名次（`Points` 模式）：依累計分數由大到小，同分比最後一輪手牌點數少者。

### R2.10 逾時與系統動作（由宿主送出）
- R2.10.1 `Timeout`（針對目前需要決策的玩家）：
  - `Play` 階段：等同 `Draw`；若之後進入 `DrawnCardDecision`，自動 `Pass`（`force_play` 時自動出牌）。
  - `ChooseColor`：選該玩家手上張數最多的顏色（同數依 紅<黃<綠<藍 的順序取第一個）；沒有有色牌則選紅。
  - `ChooseTarget`：選下一位。
  - `Challenge`：`Accept`。
- R2.10.2 `EndByTime`：立刻結束此輪，名次依 R2.9.3 的方式（手牌點數少者勝）。用於閃電模式。

---

## 3. 房規（RuleConfig 選項）

| 欄位 | 型別 | 預設 | 說明 |
|------|------|------|------|
| `hand_size` | u8 (1–15) | 7 | 起始手牌 |
| `deck_count` | u8 (1–3) | 1 | 牌組副數。〔本專案定義〕建房時若玩家數 > 6 且 deck_count = 1，UI 建議改為 2（不強制） |
| `stack_draw` | `None`/`Same`/`Progressive` | `None` | R3.1 |
| `seven_zero` | bool | false | R3.2 |
| `jump_in` | bool | false | R3.3 |
| `draw_to_match` | bool | false | R2.5.4 |
| `force_play` | bool | false | R2.5.2 |
| `wild4_rule` | `Challenge`/`Strict`/`Free` | `Challenge` | R2.3.3 |
| `uno_rule` | `Manual`/`Auto` | `Manual` | R2.7 |
| `uno_penalty` | u8 (0–6) | 2 | R2.7.1 |
| `scoring` | `Points{target}`/`SingleRound` | `Points{500}` | R2.9 |
| `turn_seconds` | u16 (0=無限) | 30 | 宿主計時用，引擎不使用 |
| `game_seconds` | u16 (0=無限) | 0 | 閃電模式總時長，宿主用 |

### R3.1 疊加 stack_draw
- R3.1.1 `pending_draw > 0` 時（有人出了抽牌卡且下一位尚未處理），目前玩家只能：(a) 出一張「可疊加」的牌，或 (b) `Draw`：抽 `pending_draw` 張並被跳過，`pending_draw` 歸零。此時不能出其他牌、不能出 Wild。
- R3.1.2 `Same`：Draw Two 只能疊 Draw Two；+4 只能疊 +4。
- R3.1.3 `Progressive`：Draw Two 可疊 Draw Two；+4 可疊 Draw Two 或 +4；Draw Two **不能**疊 +4。
- R3.1.4 疊上的牌把張數加進 `pending_draw`，換下一位面對。
- R3.1.5 Draw Two 疊加時不受顏色限制（任何顏色的 Draw Two 都可疊）。〔本專案定義〕
- R3.1.6 開啟疊加時 +4 不能質疑（`wild4_rule = Challenge` 視為 `Free`）。〔本專案定義〕
- R3.1.7 疊加時出的 +4 仍要選色。

### R3.2 七零規則 seven_zero
- R3.2.1 出 7 後（若 7 不是最後一張），出牌者進入 `ChooseTarget` 階段選一位其他玩家，交換整副手牌。
- R3.2.2 出 0 後（若 0 不是最後一張），所有仍在遊戲中的玩家把手牌整副交給依目前方向的下一位。
- R3.2.3 換手牌後所有人的 UNO 宣告狀態清除；若有人換完剩 1 張，不開 UNO 視窗。〔本專案定義〕

### R3.3 搶出 jump_in
- R3.3.1 任何**非**目前玩家，若手上有一張與棄牌堆頂牌「完全相同」（同顏色且同數字/同種類）的有色牌，可在 `Play` 階段隨時 `JumpIn { card }`。
- R3.3.2 `pending_draw > 0` 時不能搶出。萬用牌不能搶出。
- R3.3.3 搶出後，視為該玩家的回合出了這張牌（效果照常），之後從搶出者的下一位繼續。
- R3.3.4 只在即時宿主中可用；PettingZoo / Gymnasium 環境若 `jump_in = true` 要丟出 `ValueError`。

---

## 4. 模式

每個模式 = 一組 `RuleConfig` 預設值 + `ModeKind`（決定牌組與特殊規則）。

| 模式 ID | ModeKind | 預設設定 | 人數 |
|---------|----------|----------|------|
| `classic` | Classic | 全部預設 | 2–10 |
| `party` | Classic | `stack_draw=Progressive, seven_zero=true, jump_in=true` | 2–10 |
| `flip` | Flip | §4.1 | 2–10 |
| `no_mercy` | NoMercy | §4.2 | 2–6 |
| `teams` | Classic + `teams=Some(2)` | §4.3 | 4 或 6 |
| `elimination` | Classic + `elimination=true` | §4.4 | 3–10 |
| `blitz` | Classic | `turn_seconds=5, game_seconds=180, scoring=SingleRound, uno_rule=Auto` | 2–10 |
| `daily` | Classic | §4.5 | 固定 4 |
| `custom` | 任意 | 使用者設定 | — |

### 4.1 Flip（UNO Flip）
- R4.1.1 每張實體牌有「亮面 Light」與「暗面 Dark」兩面。
- R4.1.2 亮面 112 張：每色（紅、黃、綠、藍）數字 `1`–`9` 各 ×2、Draw One ×2、Reverse ×2、Skip ×2、Flip ×2（每色 26 張，共 104）；Wild ×4、Wild Draw Two ×4。**沒有 0**。
- R4.1.3 暗面 112 張：每色（粉、青、橘、紫）數字 `1`–`9` 各 ×2、Draw Five ×2、Reverse ×2、Skip Everyone ×2、Flip ×2（共 104）；Wild ×4、Wild Draw Color ×4。
- R4.1.4 配對：依序建立亮面列表與暗面列表，**用遊戲 Rng 洗暗面列表一次**，再依索引配對。CardId 依亮面順序編號。（建牌在洗牌前完成，所以同 seed 配對固定。）
- R4.1.5 開局亮面朝上；所有規則只看目前朝上的面（`side`）。
- R4.1.6 **隱藏資訊**：對手手牌的「背面」是牌的另一面，所以每位玩家都能看到其他人每張手牌的「非目前面」。牌堆頂牌的背面也可見。`PlayerView` 必須包含這些資訊。
- R4.1.7 Flip 牌效果：所有牌（手牌、牌堆、棄牌堆）翻到另一面，牌堆順序反轉〔官方：整疊翻過來〕，棄牌堆頂牌變成其另一面。新的頂牌若是萬用牌，由出 Flip 的玩家選色；若是有色牌，目前顏色 = 該牌顏色。然後輪到下一位（頂牌的新面功能**不**觸發）。
- R4.1.8 亮面功能：Draw One（下一位抽 1 並跳過）、Reverse、Skip、Wild、Wild Draw Two（選色，下一位抽 2 並跳過，可依 wild4_rule 質疑，判定方式同 R2.6，但罰抽數為 2，質疑失敗 +2 共 4）。
- R4.1.9 暗面功能：Draw Five（下一位抽 5 並跳過）、Reverse、Skip Everyone（其他所有人都被跳過，出牌者再出一次）、Wild、Wild Draw Color（選色，下一位一直抽到抽出該顏色的牌為止並被跳過；可質疑：虛張成立則出牌者改為照同樣方式抽；質疑失敗則受害者額外再抽 2 張〔本專案定義〕）。
- R4.1.10 計分（看玩家手牌在**一輪結束時朝上的面**）：亮面：數字 = 面值、Draw One = 10、Reverse/Skip/Flip = 20、Wild = 40、Wild Draw Two = 50。暗面：數字 = 面值、Draw Five/Reverse/Skip Everyone/Flip = 20、Wild = 40、Wild Draw Color = 60。
- R4.1.11 疊加規則在 Flip 模式中不可開啟（UI 隱藏該選項；引擎若收到則視為 `None`）。七零規則不可開啟（沒有 0）。
- R4.1.12 起始牌若是 Wild Draw Two 依 R2.2.6 Wild Draw Four 的方式重翻；Flip 起始牌：直接翻面（等同出了 Flip，由莊家選色若需要），然後 `first` 開始。〔本專案定義〕

### 4.2 No Mercy（無情）
- R4.2.1 牌組 168 張：每色（紅黃綠藍）數字 `0`–`9` 各 ×2（80）、Draw Two ×3（12）、Draw Four（有色）×2（8）、Skip ×3（12）、Skip Everyone ×2（8）、Reverse ×3（12）、Discard All ×3（12）；黑色：Wild Reverse Draw Four ×8、Wild Draw Six ×4、Wild Draw Ten ×4、Wild Color Roulette ×8。
- R4.2.2 強制設定：`stack_draw = NoMercy`（特殊規則見 R4.2.3）、`seven_zero = true`、`draw_to_match = true`、`wild4_rule = Free`、`scoring = SingleRound`。
- R4.2.3 疊加：抽牌卡價值 Draw Two=2、Draw Four=4、Wild Reverse Draw Four=4、Wild Draw Six=6、Wild Draw Ten=10。只能疊「價值 ≥ 頂端抽牌卡」的抽牌卡。有色抽牌卡疊加時不受顏色限制。無法或不想疊的人抽走全部並被跳過。
- R4.2.4 Wild Reverse Draw Four：選色、方向反轉，「反轉後的下一位」承受 +4（可再疊）。
- R4.2.5 Discard All：出這張時，同時把手上所有與此牌同色的牌一起丟到棄牌堆（頂牌仍是 Discard All）。
- R4.2.6 Wild Color Roulette：出牌者選色〔本專案定義：由出牌者選，簡化流程〕，下一位一直翻牌堆直到翻出該顏色，翻出的牌全部加入手牌，並被跳過。
- R4.2.7 Skip Everyone：同 R4.1.9。
- R4.2.8 Mercy Rule：任何玩家手牌 ≥ 25 張立即淘汰，手牌洗回牌堆底部（洗牌後放到牌堆底）。若只剩 1 位玩家，他獲勝。
- R4.2.9 名次：出完牌者第 1；其餘依手牌點數（數字=面值、有色功能牌=20、黑色=50），被淘汰者依淘汰順序排最後（越早淘汰越後面）。

### 4.3 團隊 Teams
- R4.3.1 4 人（2v2）或 6 人（3v3 → `teams=Some(2)`，每隊 3 人）。座位 `i` 屬於隊伍 `i % 2`，因此隊友交錯而坐。
- R4.3.2 任一隊員出完牌，該隊贏得此輪；得分 = 對手隊伍所有成員手牌點數總和；`Points` 模式下隊伍分數累積到 `target_score`。
- R4.3.3 隊友之間**看不到**彼此手牌；不能 `CatchUno` 隊友。
- R4.3.4 RL：獎勵以隊伍為單位。

### 4.4 淘汰賽 Elimination
- R4.4.1 每輪結束（有人出完）時，剩餘手牌點數最高者淘汰（同分：手牌張數多者；再同分：座位號大者）。
- R4.4.2 被淘汰的玩家不再發牌、不在輪次中。
- R4.4.3 剩 1 人時他贏得整局。名次 = 淘汰順序反過來。
- R4.4.4 本模式忽略 `scoring` 設定：每輪只決定誰被淘汰，不累積分數；`RoundEnded.scores` 全填 0，`hand_points` 照實填寫。

### 4.5 每日挑戰 Daily
- R4.5.1 經典規則、4 人：真人在座位 0，座位 1–3 為「困難」AI。
- R4.5.2 seed = `fnv1a64("uno-daily-" + YYYY-MM-DD)`（伺服器本地日期）。AI 的亂數 seed = 遊戲 seed + 座位號。
- R4.5.3 `scoring = SingleRound`。分數：贏 = 其他人手牌點數總和；輸 = −（自己手牌點數）。
- R4.5.4 每位使用者每天只有第一次的分數進排行榜；之後可以重玩但標記為練習。

---

## 5. 測試必備案例（實作者至少要寫這些）
1. 建牌張數：Classic 108、Flip 112（兩面各自統計正確）、No Mercy 168。
2. 牌數守恆：任何時刻 手牌 + 牌堆 + 棄牌堆 = 牌組總數（property test，隨機合法動作 10,000 局）。
3. R2.2.6 每種起始牌各一個測試（用可控的「預先排好的牌組」建構函式 `GameState::new_with_deck`，只在 `cfg(test)` 或 `testing` feature 中提供）。
4. R2.4.1 / R2.4.2 兩人 Skip / Reverse。
5. R2.6.3 質疑成功 / 失敗。
6. R2.7 UNO 視窗開啟、被抓、自救、自動關閉。
7. R3.1 三種疊加規則的合法動作。
8. R3.2 7 換牌、0 傳牌。
9. R4.1.7 Flip 翻面後手牌、牌堆順序、頂牌、顏色正確。
10. R4.2.8 25 張淘汰。
11. 決定性：同 seed、同動作 → 最終狀態 hash 相同。
12. `PlayerView` 不包含他人手牌（Flip 只包含背面）。
