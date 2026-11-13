# 介面規格（INTERFACES）

> 跨模組的所有型別、函式簽名、編碼方式、協議、資料庫結構都定義在這裡。
> 實作時**名稱、欄位、順序必須一致**。要修改請先改本檔，再改程式。

---

## 1. `uno-core`（Rust）

### 1.1 基本型別

```rust
pub type Seat = u8;          // 0..=9
pub type CardId = u16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Color { Red, Yellow, Green, Blue, Pink, Teal, Orange, Purple }
// 亮面/經典只用前 4 個；Flip 暗面用後 4 個。
// 「顏色槽」color_slot(): Red/Pink=0, Yellow/Teal=1, Green/Orange=2, Blue/Purple=3。

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "k", content = "v", rename_all = "snake_case")]
pub enum Face {
    Number(u8),              // 0..=9
    Skip, Reverse, DrawTwo,
    Wild, WildDrawFour,
    // Flip
    DrawOne, Flip, WildDrawTwo, DrawFive, SkipEveryone, WildDrawColor,
    // No Mercy
    DrawFour,                // 有色 +4
    DiscardAll, WildReverseDrawFour, WildDrawSix, WildDrawTen, WildColorRoulette,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct CardFace { pub color: Option<Color>, pub face: Face }   // 黑牌 color = None

#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
pub struct Card { pub id: CardId, pub light: CardFace, pub dark: Option<CardFace> }
// 非 Flip 模式 dark = None。

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Side { Light, Dark }
```

### 1.2 設定

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind { Classic, Flip, NoMercy }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StackDraw { None, Same, Progressive, NoMercy }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Wild4Rule { Challenge, Strict, Free }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UnoRule { Manual, Auto }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scoring { Points { target: u32 }, SingleRound }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct RuleConfig {
    pub mode_id: String,          // "classic" | "party" | "flip" | ...（RULES §4）
    pub mode: ModeKind,
    pub num_players: u8,          // 2..=10
    pub hand_size: u8,
    pub deck_count: u8,
    pub stack_draw: StackDraw,
    pub seven_zero: bool,
    pub jump_in: bool,
    pub draw_to_match: bool,
    pub force_play: bool,
    pub wild4_rule: Wild4Rule,
    pub uno_rule: UnoRule,
    pub uno_penalty: u8,
    pub scoring: Scoring,
    pub teams: Option<u8>,        // Some(2) = 兩隊
    pub elimination: bool,
    pub turn_seconds: u16,        // 宿主用
    pub game_seconds: u16,        // 宿主用
}

impl RuleConfig {
    /// 依 RULES §4 回傳模式預設值。未知 mode_id 回傳 Err。
    pub fn preset(mode_id: &str, num_players: u8) -> Result<Self, EngineError>;
    /// 依 RULES 檢查並修正（例如 Flip 強制 stack_draw=None）。回傳錯誤若人數不合法。
    pub fn validate(&mut self) -> Result<(), EngineError>;
}
```

### 1.3 狀態、動作、事件

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Play,               // 目前玩家要出牌/抽牌
    ChooseColor,        // 目前玩家要選色
    ChooseTarget,       // 7 換牌選對象
    Challenge,          // +4 受害者決定質疑/接受
    DrawnCardDecision,  // 抽到可出的牌：出或保留
    RoundOver,          // 宿主呼叫 next_round()
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Play { card: CardId },
    Draw,
    Pass,
    ChooseColor { color: Color },
    ChooseTarget { target: Seat },
    CallUno,
    CatchUno { target: Seat },
    Challenge,
    Accept,
    JumpIn { card: CardId },
    // 系統動作（只有宿主可送，seat 參數填目前決策者）
    Timeout,
    EndByTime,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    RoundStarted { round: u32, dealer: Seat, hand_sizes: Vec<u8> },
    StartCardFlipped { card: Card },
    TurnStarted { seat: Seat },
    CardPlayed { seat: Seat, card: Card, jump_in: bool },
    CardsDiscarded { seat: Seat, cards: Vec<Card> },         // Discard All 一起丟的牌
    CardsDrawn { seat: Seat, count: u8, cards: Option<Vec<Card>>, reason: DrawReason },
    // cards 只在「給該玩家本人的事件」或公開翻牌（Roulette）時為 Some，其餘為 None（由 filter_events 處理）
    DeckReshuffled { size: u16 },
    ColorChosen { seat: Seat, color: Color },
    PlayerSkipped { seat: Seat },
    DirectionChanged { direction: i8 },
    PendingDrawChanged { amount: u16 },
    ChallengeResolved { challenger: Seat, offender: Seat, success: bool },
    UnoCalled { seat: Seat },
    UnoCaught { catcher: Seat, target: Seat, penalty: u8 },
    HandsSwapped { a: Seat, b: Seat },
    HandsRotated { direction: i8 },
    SideFlipped { side: Side },
    PlayerEliminated { seat: Seat, reason: EliminationReason },
    RoundEnded { winner: Option<Seat>, scores: Vec<i32>, hand_points: Vec<u32>, totals: Vec<i32> },
    GameEnded { rankings: Vec<Seat> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DrawReason { Voluntary, Penalty, Stack, Challenge, Uno, Roulette, DrawColor }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum EliminationReason { Mercy, Elimination }

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("not your turn")] NotYourTurn,
    #[error("illegal action")] IllegalAction,
    #[error("game over")] GameOver,
    #[error("invalid config: {0}")] InvalidConfig(String),
}

pub struct GameState { /* 欄位私有；至少包含：
    config, rng, cards: Vec<Card>（依 CardId 索引）, side,
    hands: Vec<Vec<CardId>>, draw_pile: Vec<CardId>（末端為頂）, discard: Vec<CardId>（末端為頂）,
    current: Seat, direction: i8, active_color: Option<Color>, pending_draw: u16,
    phase: Phase, uno_called: Vec<bool>, uno_window: Option<Seat>,
    drawn_card: Option<CardId>, wild4_was_legal: bool, challenge_offender: Option<Seat>,
    eliminated: Vec<bool>, elimination_order: Vec<Seat>, scores: Vec<i32>,
    round: u32, dealer: Seat, turn: u32, rankings: Option<Vec<Seat>>
*/ }

impl GameState {
    pub fn new(config: RuleConfig, seed: u64) -> Result<Self, EngineError>; // 會建牌、洗牌、發牌，可能產生起始事件 → 用 initial_events() 取得
    pub fn initial_events(&self) -> &[Event];
    pub fn phase(&self) -> Phase;
    /// 目前「必須」做決策的玩家；RoundOver/GameOver 時為 None。
    pub fn decision_player(&self) -> Option<Seat>;
    /// 給 seat 的所有合法動作（含非輪到時的 CatchUno / CallUno / JumpIn）。不含系統動作。
    pub fn legal_actions(&self, seat: Seat) -> Vec<Action>;
    pub fn apply(&mut self, seat: Seat, action: Action) -> Result<Vec<Event>, EngineError>;
    pub fn next_round(&mut self) -> Result<Vec<Event>, EngineError>; // 只在 RoundOver 可呼叫
    pub fn view(&self, seat: Seat) -> PlayerView;
    pub fn spectator_view(&self) -> PlayerView;   // 不含任何手牌
    pub fn god_view(&self) -> GodView;            // 重播上帝視角、測試用
    pub fn is_over(&self) -> bool;
    pub fn rankings(&self) -> Option<&[Seat]>;
    pub fn state_hash(&self) -> u64;              // fnv1a64(serde_json::to_vec(god_view))
    pub fn sample_world(&self, seat: Seat, rng: &mut Rng) -> GameState; // ISMCTS 用，見 §3.3
}

/// 把完整事件轉成 seat 能看到的版本（隱藏他人抽到的牌）。seat=None 表示觀戰者。
pub fn filter_events(events: &[Event], seat: Option<Seat>) -> Vec<Event>;
```

### 1.4 亂數（決定性的關鍵）
- 演算法：**xoshiro256\*\***，以 **SplitMix64** 從 `u64` seed 產生 4 個 u64 狀態。自己實作（約 40 行），**不要**用 `rand` crate（版本間輸出可能改變）。
- `fn next_u64(&mut self) -> u64`
- `fn below(&mut self, n: u64) -> u64 { self.next_u64() % n }`（接受微小偏差，換取簡單與跨平台一致）。
- 洗牌：`for i in (1..len).rev() { let j = rng.below(i as u64 + 1) as usize; v.swap(i, j); }`
- `Rng` 實作 `Clone + Serialize + Deserialize`。
- 測試：seed=0 的前 3 個 `next_u64` 寫成固定值的回歸測試（第一次執行時記下來）。

### 1.5 `PlayerView`（前端與 Bot 唯一能看到的資料）

```rust
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct PlayerView {
    pub me: Option<Seat>,                 // 觀戰者 None
    pub config: RuleConfig,
    pub phase: Phase,
    pub decision_player: Option<Seat>,
    pub current: Seat,
    pub direction: i8,
    pub side: Side,
    pub top_card: Card,
    pub active_color: Option<Color>,
    pub pending_draw: u16,
    pub draw_pile_size: u16,
    pub draw_pile_top_back: Option<CardFace>,   // 只有 Flip
    pub discard: Vec<Card>,                     // 自上次洗牌後的棄牌堆（底→頂），記牌用
    pub my_hand: Vec<Card>,                     // 觀戰者為空
    pub drawn_card: Option<CardId>,             // DrawnCardDecision 時給本人
    pub players: Vec<PublicPlayer>,             // 長度 = num_players，依座位
    pub uno_window: Option<Seat>,
    pub round: u32,
    pub turn: u32,
    pub legal: Vec<Action>,                     // 方便前端直接用
    pub rankings: Option<Vec<Seat>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct PublicPlayer {
    pub seat: Seat,
    pub hand_size: u8,
    pub hand_backs: Option<Vec<CardFace>>,      // Flip：其他人手牌的背面；非 Flip 為 None；自己為 None
    pub uno_called: bool,
    pub eliminated: bool,
    pub team: Option<u8>,
    pub score: i32,
}
```

### 1.6 模組檔案（uno-core/src）
`lib.rs`（re-export）、`rng.rs`、`card.rs`（型別 + 牌點）、`deck.rs`（三種牌組建構）、`config.rs`、`state.rs`（GameState 結構與建構、發牌）、`rules.rs`（合法性判斷）、`apply.rs`（動作套用）、`effects.rs`（功能牌效果）、`scoring.rs`、`view.rs`、`events.rs`、`encode.rs`（§2 RL 編碼）、`replay.rs`（§4）、`world.rs`（sample_world）。單檔超過 600 行要拆。

---

## 2. RL 編碼（`uno-core/src/encode.rs`，觀測版本 `OBS_VERSION = 1`）

### 2.1 牌面類型表 FaceTable
每個 ModeKind 有一張固定順序的「牌面類型」表，長度 `K`：
- **Classic（K=54）**：對每色 紅→黃→綠→藍：`Number(0..=9)`、`Skip`、`Reverse`、`DrawTwo`（13 個）→ 52；再加 `Wild`、`WildDrawFour`。
- **NoMercy（K=68）**：每色：`Number(0..=9)`、`DrawTwo`、`DrawFour`、`Skip`、`SkipEveryone`、`Reverse`、`DiscardAll`（16 個）→ 64；再加 `WildReverseDrawFour`、`WildDrawSix`、`WildDrawTen`、`WildColorRoulette`。
- **Flip（K=108）**：亮面：每色 紅→黃→綠→藍：`Number(1..=9)`、`DrawOne`、`Reverse`、`Skip`、`Flip`（13）→ 52，加 `Wild`、`WildDrawTwo` → 54；暗面：每色 粉→青→橘→紫：`Number(1..=9)`、`DrawFive`、`Reverse`、`SkipEveryone`、`Flip` → 52，加 `Wild`、`WildDrawColor` → 54。亮面索引 0..54、暗面 54..108。

`fn face_index(mode, &CardFace) -> usize`、`fn face_from_index(mode, usize) -> CardFace`。萬用牌在表中 `color = None`。

### 2.2 行動空間（大小 `A = K + 27`）

| 索引 | 動作 |
|------|------|
| `0 .. K` | `Play`：出手上「朝上面 = 該牌面類型」的牌。若有多張，出 CardId 最小的那張；`DrawnCardDecision` 階段則只能是抽到的那張 |
| `K` | `Draw` |
| `K+1` | `Pass` |
| `K+2 .. K+6` | `ChooseColor`：顏色槽 0..3（依目前 side 轉成實際顏色） |
| `K+6 .. K+15` | `ChooseTarget`：相對座位 r = 1..9，`target = (me + r) % n` |
| `K+15` | `CallUno` |
| `K+16` | `Challenge` |
| `K+17` | `Accept` |
| `K+18 .. K+27` | `CatchUno`：相對座位 r = 1..9 |

Classic A=81、NoMercy A=95、Flip A=135。`JumpIn` 不在 RL 行動空間。

```rust
pub fn action_size(mode: ModeKind) -> usize;
pub fn action_mask(state: &GameState, seat: Seat) -> Vec<bool>;     // 長度 A
pub fn index_to_action(state: &GameState, seat: Seat, idx: usize) -> Result<Action, EngineError>;
pub fn action_to_index(state: &GameState, seat: Seat, a: &Action) -> Option<usize>;
```
不變式（必須有測試）：`mask[i] == true` ⇔ `index_to_action(i)` 成功且結果在 `legal_actions` 中；`legal_actions` 中每個非 JumpIn 動作都對應某個 true 索引。

### 2.3 觀測向量（`f32`，長度 `D`）
依序串接：

| # | 區段 | 長度 | 內容 |
|---|------|------|------|
| 1 | 手牌 | K | 每種牌面類型的張數（朝上面） |
| 2 | 頂牌 | K | one-hot（朝上面） |
| 3 | 目前顏色 | 4 | 顏色槽 one-hot（無則全 0） |
| 4 | 階段 | 5 | Play, ChooseColor, ChooseTarget, Challenge, DrawnCardDecision |
| 5 | 抽到的牌 | K | DrawnCardDecision 時 one-hot，否則 0 |
| 6 | 方向 | 1 | +1 / −1 |
| 7 | 累積抽牌 | 1 | pending_draw / 10 |
| 8 | 對手 | 9×6 | 相對座位 r=1..9 各：`存在(r<n)`、`hand_size/25`、`uno_called`、`eliminated`、`是隊友`、`score/target`（SingleRound 時 0）|
| 9 | 自己 | 3 | `hand_size/25`、`uno_called`、`score/target` |
| 10 | 已見棄牌 | K | 棄牌堆中每種牌面類型張數 / deck_count |
| 11 | 牌堆 | 1 | draw_pile_size / 總張數 |
| 12 | UNO 視窗 | 10 | 相對座位 r=0..9 的 one-hot（無視窗則全 0） |
| 13 | Flip 專用 | 1 + 9×K | `side`（Light=0, Dark=1）＋每位對手手牌背面的類型張數（只有 Flip 模式有這段） |

Classic D = 54+54+4+5+54+1+1+54+3+54+1+10 = **295**。`pub fn obs_size(mode) -> usize`、`pub fn observe(state, seat) -> Vec<f32>`。觀測只能使用 `PlayerView` 可得資訊（實作方式：`observe` 內部先呼叫 `view(seat)` 再編碼，確保不洩漏）。

---

## 3. `uno-bots`

```rust
pub trait Bot: Send {
    fn name(&self) -> &'static str;
    fn act(&mut self, state_view: &PlayerView) -> Action;   // 只能從 view.legal 中挑
}
pub enum BotKind { Random, Heuristic, Hard, Onnx(String /* model path */) }
pub fn make_bot(kind: &BotKind, seed: u64) -> Result<Box<dyn Bot>, BotError>;
```

### 3.1 Random
從 `view.legal` 均勻挑（使用自己的 `Rng`）。若 `CallUno` 合法，一定先做（避免被罰太蠢）。

### 3.2 Heuristic（依序判斷，第一個符合就做）
1. `CallUno` 合法 → 做。
2. `CatchUno` 合法 → 做。
3. `Challenge` 階段：若出 +4 者上一次之前的目前顏色的牌在棄牌堆中出現很少（已見 < 該色總數 30%），且出牌者手牌 ≥ 4 張 → `Challenge`，否則 `Accept`。
4. `ChooseColor`：手上張數最多的顏色槽。
5. `ChooseTarget`：手牌最少的對手。
6. `DrawnCardDecision`：能出就出。
7. `Play` 階段計分選擇：每個合法 `Play` 動作計分：
   - 下一位手牌 ≤ 2：功能牌（Skip/Reverse/Draw*）+50。
   - 數字牌：+面值（先丟大的）。
   - 能維持出「自己最多顏色」+10。
   - Wild/+4 −30（留到後面），但手牌 ≤ 2 時 +40。
   - 選最高分；都沒有 → `Draw`。

### 3.3 Hard（ISMCTS）
- 每次決策：`sims = 400`（手機上單機模式 `200`）、每次模擬：`world = state.sample_world(me, rng)` → 用 Heuristic 對所有人 rollout 到這輪結束 → 依「勝 1、敗 0；多人時 1 − rank/(n−1)」回傳值；用 UCB1（c=0.7）在根節點的動作上做統計（只做「根節點平行化」的扁平 MC 即可，不必建完整樹）。
- `sample_world`：保留自己手牌、所有公開資訊；把「未知牌」（其他人手牌 + 牌堆）收集起來洗牌後依原本張數重新分配。Flip 模式下只在「背面相同」的牌之間互換。
- 由於 Bot 只拿到 `PlayerView`，Hard bot 需要一個 `GameState::from_view_sampled(view, rng)` 版本：用 view 重建一個一致的 GameState（未知牌隨機分配）。這個函式放在 `uno-core/src/world.rs`。
- 時間上限：每步最多 300ms（wasm 中以模擬次數控制，不讀時間）。

### 3.4 Onnx
- 用 `tract-onnx` 載入；輸入 `obs: f32[1, D]`、`mask: bool→f32[1, A]`（1/0）；輸出 `logits: f32[1, A]`。取 mask 後的 argmax（`temperature` > 0 時抽樣）。
- 模型旁邊要有同名 `.json`（§5.6），載入時檢查 `mode`、`obs_version`、`obs_size`、`action_size` 一致，否則回傳錯誤。
- 瀏覽器端不使用 tract，而用 `onnxruntime-web`（因為檔案較小較快），邏輯相同；觀測與 mask 由 WASM 的 `observe`/`action_mask` 產生。

---

## 4. 重播格式（JSON，`replay_version = 1`）

```json
{
  "replay_version": 1,
  "engine_version": "0.1.0",
  "config": { "...RuleConfig..." },
  "seed": 123456789,
  "players": [
    { "seat": 0, "name": "Alice", "user_id": 12, "kind": "human" },
    { "seat": 1, "name": "Bot (困難)", "user_id": null, "kind": "bot", "bot": "hard" }
  ],
  "steps": [
    { "seat": 0, "action": { "type": "play", "card": 37 }, "t": 1532 },
    { "seat": null, "action": "next_round", "t": 90000 }
  ],
  "created_at": "2026-11-13T05:00:00Z"
}
```
- `t` = 從開局起算的毫秒數（宿主填寫；Python 填 0）。
- `seat: null, action: "next_round"` 代表宿主呼叫 `next_round()`。
- 系統動作 `Timeout` / `EndByTime` 也要記錄（seat 為當時決策者）。
- `pub fn replay(r: &Replay) -> Result<(GameState, Vec<Vec<Event>>), EngineError>`：重跑並驗證每步合法。

---

## 5. Python 介面

### 5.1 套件結構
- `crates/uno-py`：maturin 專案，模組名 `uno_engine`（原生，`pyo3` abi3-py310）。`python/pyproject.toml` 使用 maturin 建置後端，`python-source = "."`，`module-name = "uno_rl._engine"`；`uno_rl/__init__.py` 再 re-export。
- 安裝：`cd python && uv venv --python 3.12 && uv pip install maturin && maturin develop --release -m ../crates/uno-py/Cargo.toml`（PLAN 會給完整指令）。

### 5.2 `uno_rl._engine.Game`

```python
class Game:
    def __init__(self, mode: str = "classic", num_players: int = 4, seed: int = 0,
                 config: dict | None = None): ...   # config 覆蓋 preset 欄位
    # 屬性
    num_players: int; mode: str; obs_size: int; action_size: int
    def phase(self) -> str: ...
    def decision_player(self) -> int | None: ...
    def legal_actions(self, seat: int) -> list[dict]: ...        # 與 Rust Action 的 serde JSON 相同
    def apply(self, seat: int, action: dict) -> list[dict]: ...  # 回傳完整事件
    def apply_index(self, seat: int, idx: int) -> list[dict]: ...
    def next_round(self) -> list[dict]: ...
    def observe(self, seat: int) -> "np.ndarray[float32, (D,)]": ...
    def action_mask(self, seat: int) -> "np.ndarray[bool, (A,)]": ...
    def view(self, seat: int) -> dict: ...
    def god_view(self) -> dict: ...
    def is_over(self) -> bool: ...
    def rankings(self) -> list[int] | None: ...
    def scores(self) -> list[int]: ...
    def clone(self) -> "Game": ...
    def state_hash(self) -> int: ...
    def to_replay(self) -> str: ...                                # JSON 字串
    @staticmethod
    def from_replay(s: str) -> "Game": ...

def bot_act(kind: str, game: Game, seat: int, seed: int) -> int: ...   # 回傳行動索引；kind: random|heuristic|hard
```

### 5.3 `VecGame`（Rust 內批次，給高吞吐訓練）

```python
class VecGame:
    def __init__(self, num_envs: int, mode: str = "classic", num_players: int = 4,
                 seed: int = 0, opponents: str | list[str] = "heuristic",
                 learner_seat: int | None = 0,       # None = 每局隨機座位
                 reward: str = "win_loss", config: dict | None = None): ...
    def reset(self) -> tuple[np.ndarray, np.ndarray]: ...            # obs[N,D] f32, mask[N,A] bool
    def step(self, actions: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, list[dict]]:
        """回傳 obs, mask, reward[N] f32, done[N] bool, infos。
        done 的那一格會自動重開新局；infos[i] 有 'terminal_obs'、'rankings'、'episode_len'。"""
```
- 內部：每個 env 套用 learner 動作後，用 Rust Bot 推進其他座位直到 learner 需要決策或遊戲結束；`RoundOver` 自動 `next_round()`。
- 使用 `rayon` 在 `num_envs >= 64` 時平行（`py.allow_threads` 釋放 GIL）。
- 獎勵（`reward=`）：
  - `win_loss`：遊戲結束時 rank 0 → +1，其他 → −1；2 人以上用 `1 − 2·rank/(n−1)`。其他步 0。
  - `score`：遊戲結束時 `scores[learner] / 100`。
  - `rank`：同 `win_loss` 但中途每輪結束也給該輪的 rank 獎勵 × 0.2。

### 5.4 Gymnasium 環境 `uno_rl.UnoEnv`
- 註冊 id：`"Uno-v0"`（`gymnasium.register`，entry point `uno_rl.envs:UnoEnv`）。
- `__init__(mode="classic", num_players=4, opponents="heuristic", learner_seat=0, reward="win_loss", config=None, render_mode=None)`
- `observation_space = Box(-1, 50, (D,), float32)`；`action_space = Discrete(A)`。
- `reset(seed=None, options=None) -> (obs, info)`，`info = {"action_mask": mask}`。
- `step(a) -> (obs, reward, terminated, truncated=False, info)`；非法動作 → 丟 `ValueError`（不默默修正）。
- `action_masks()` 方法回傳 mask（sb3-contrib MaskablePPO 會呼叫）。
- `render_mode="ansi"`：回傳文字桌面（手牌、頂牌、各人張數）。

### 5.5 PettingZoo 環境
- `uno_rl.env(**kw)`（AEC，含 `wrappers.OrderEnforcingWrapper`）、`uno_rl.raw_env`。
- agents：`"player_0" … "player_{n-1}"`。
- `observation_space(agent) = Dict({"observation": Box(D), "action_mask": Box(0,1,(A,),int8)})`（PettingZoo 經典棋類慣例）。
- `agent_selection` = `decision_player()`。
- 獎勵只在遊戲結束時給（`win_loss` 規則）；團隊模式隊友同分。
- 必須通過 `pettingzoo.test.api_test(env, num_cycles=1000)`。

### 5.6 ONNX 匯出 metadata（`models/<name>.json`）

```json
{ "name": "ppo_classic_4p_v1", "mode": "classic", "num_players": 4,
  "obs_version": 1, "obs_size": 295, "action_size": 81,
  "input_names": ["obs", "mask"], "output_names": ["logits"],
  "algo": "MaskablePPO", "trained_steps": 50000000,
  "eval": { "vs_heuristic_winrate": 0.41 }, "created_at": "..." }
```
`mask` 輸入在模型內部的用法：`logits = logits + (mask - 1) * 1e9`。

### 5.7 RLCard 相容層（`uno_rl.rlcard_compat`）
提供 `RLCardUnoEnv`，具有 RLCard `Env` 的 `reset()`, `step(action)`, `get_state(player_id)` 等方法與 `state['legal_actions']` 格式（OrderedDict 索引→None），讓用 RLCard 寫的 DQN/NFSP agent 可直接跑。只支援 classic 模式。

---

## 6. WASM 介面（`uno-wasm`，`wasm-pack build --target web`）

```rust
#[wasm_bindgen] pub struct WasmGame { /* GameState */ }
#[wasm_bindgen] impl WasmGame {
    #[wasm_bindgen(constructor)] pub fn new(config: JsValue, seed: u64) -> Result<WasmGame, JsError>;
    pub fn initial_events(&self) -> JsValue;                         // Event[]（完整，宿主自行 filter）
    pub fn view(&self, seat: u8) -> JsValue;                         // PlayerView
    pub fn spectator_view(&self) -> JsValue;
    pub fn decision_player(&self) -> Option<u8>;
    pub fn legal(&self, seat: u8) -> JsValue;                        // Action[]
    pub fn apply(&mut self, seat: u8, action: JsValue) -> Result<JsValue, JsError>; // Event[]
    pub fn next_round(&mut self) -> Result<JsValue, JsError>;
    pub fn is_over(&self) -> bool;
    pub fn observe(&self, seat: u8) -> Vec<f32>;                     // Float32Array
    pub fn action_mask(&self, seat: u8) -> Vec<u8>;                  // Uint8Array
    pub fn index_to_action(&self, seat: u8, idx: u32) -> Result<JsValue, JsError>;
    pub fn replay_json(&self) -> String;
}
#[wasm_bindgen] pub fn filter_events(events: JsValue, seat: Option<u8>) -> JsValue;
#[wasm_bindgen] pub fn preset(mode_id: &str, num_players: u8) -> Result<JsValue, JsError>;
#[wasm_bindgen] pub struct WasmBot { /* Box<dyn Bot> */ }
#[wasm_bindgen] impl WasmBot {
    #[wasm_bindgen(constructor)] pub fn new(kind: &str, seed: u64) -> Result<WasmBot, JsError>; // random|heuristic|hard
    pub fn act(&mut self, view: JsValue) -> JsValue;                 // Action
}
#[wasm_bindgen] pub fn replay_validate(json: &str) -> Result<JsValue, JsError>; // 回傳 rankings
```
- `WasmGame` 內部要記錄所有 steps，以便 `replay_json()` 輸出。
- 序列化使用 `serde_wasm_bindgen::Serializer::json_compatible()`（u64 → number，確保 seed 不會變成 BigInt；seed 在 JS 端限制在 `0..2^53`）。

---

## 7. WebSocket 協議（`uno-protocol`，路徑 `/ws`，JSON 文字訊息）

所有型別以 `#[derive(Serialize, Deserialize, TS)] #[serde(tag = "type", rename_all = "snake_case")]` 定義，`ts-rs` 匯出到 `apps/web/src/generated/`。

### 7.1 Client → Server

```rust
pub enum ClientMsg {
    Hello { token: String, protocol: u32 },                 // 連線後第一則；protocol = 1
    CreateRoom { mode_id: String, config: Option<RuleConfig>, public: bool },
    JoinRoom { code: String, as_spectator: bool },
    LeaveRoom,
    TakeSeat { seat: u8 },
    AddBot { seat: u8, bot: String },                       // 房主；bot: random|heuristic|hard|onnx:<model>
    RemoveSeat { seat: u8 },                                // 房主：踢人/移除 bot
    UpdateConfig { config: RuleConfig },                    // 房主，等待中才可
    SetReady { ready: bool },
    StartGame,                                              // 房主，所有真人 ready
    Action { seq: u32, action: Action },                    // seq 每個客戶端遞增
    NextRound,                                              // 房主（RoundOver 時；也會在 8 秒後自動）
    Rematch,                                                // 房主，GameOver 後
    Chat { text: String },                                  // ≤ 200 字
    Emote { emote: String },                                // "thumbs_up" | "laugh" | "angry" | "cry" | "fire" | "uno"
    Ping { t: u64 },
}
```

### 7.2 Server → Client

```rust
pub enum ServerMsg {
    Welcome { user: UserPublic, server_version: String },
    Error { code: ErrorCode, message: String, seq: Option<u32> },
    RoomState { room: RoomInfo },                           // 房間任何變動時整份送
    GameSnapshot { view: PlayerView, timer: Option<TurnTimer> },  // 加入/重連/每次動作後
    GameEvents { events: Vec<Event>, view: PlayerView, timer: Option<TurnTimer> }, // 已依接收者 filter
    ChatMsg { from: String, seat: Option<u8>, text: String, t: u64 },
    EmoteMsg { seat: u8, emote: String },
    MatchSaved { match_id: i64, elo_changes: Vec<EloChange> },
    Pong { t: u64 },
}

pub struct RoomInfo {
    pub code: String, pub host_user_id: i64, pub public: bool,
    pub config: RuleConfig, pub status: RoomStatus,          // waiting | playing | round_over | game_over
    pub seats: Vec<SeatInfo>,                                 // 長度 = config.num_players
    pub spectators: Vec<UserPublic>,
    pub join_url: String,                                     // http://<lan-ip>:<port>/r/<code>
}
pub struct SeatInfo { pub seat: u8, pub occupant: Occupant, pub ready: bool, pub connected: bool }
pub enum Occupant { Empty, Human { user: UserPublic }, Bot { kind: String } }
pub struct TurnTimer { pub seat: u8, pub deadline_ms: u64, pub total_ms: u32 } // deadline 為伺服器 epoch ms；客戶端用 Ping/Pong 估時差
pub struct UserPublic { pub id: i64, pub name: String, pub is_guest: bool }
pub struct EloChange { pub user_id: i64, pub before: i32, pub after: i32 }
pub enum ErrorCode { BadRequest, Unauthorized, RoomNotFound, RoomFull, NotHost, NotYourTurn, IllegalAction, GameInProgress, RateLimited, Internal }
```

### 7.3 伺服器行為
- 每個房間一個 tokio task（actor），透過 `mpsc` 接收命令；房間狀態只在該 task 內修改。
- 收到 `Action`：驗證 → `apply` → 對每位連線者 `filter_events(events, seat)` + `view(seat)` 送出 `GameEvents`。
- Bot 座位：輪到 bot（或 bot 有 CatchUno 機會）時，排程延遲 `600–1200ms`（隨機）後執行；CatchUno 反應：簡單 0%、普通 30%、困難 70% 機率在 1.5 秒內抓。
- 斷線：座位 `connected=false`，60 秒內由 Heuristic bot 代打（延遲 1s）；逾時則該座位永久轉成 bot。
- 回合計時：`turn_seconds > 0` 時，deadline 到期送 `Timeout`。`game_seconds > 0` 時，到期送 `EndByTime`。
- 速率限制：每連線每秒最多 10 則訊息，超過回 `RateLimited`。
- 房間在所有真人離開 5 分鐘後刪除。
- GameOver → 寫入資料庫（§9）→ 送 `MatchSaved`。

---

## 8. HTTP API（axum，前綴 `/api`，JSON）

| 方法 | 路徑 | 請求 | 回應 |
|------|------|------|------|
| POST | `/api/guest` | `{nickname}` | `{token, user}` |
| POST | `/api/register` | `{username, password}`（若帶訪客 token 則升級該帳號） | `{token, user}` |
| POST | `/api/login` | `{username, password}` | `{token, user}` |
| POST | `/api/logout` | — | `204` |
| GET | `/api/me` | — | `{user, ratings: {mode_id: elo}}` |
| GET | `/api/server-info` | — | `{version, lan_urls: string[], port}` |
| GET | `/api/rooms` | — | 公開且等待中的房間 `RoomInfo[]` |
| GET | `/api/users/:id/stats` | — | `{games, wins, by_mode: {mode_id: {games, wins, elo}}, elo_history: [{t, mode_id, elo}]}` |
| GET | `/api/users/:id/matches?limit=50&offset=0` | — | `[{match_id, mode_id, created_at, rank, num_players, score}]` |
| GET | `/api/matches/:id/replay` | — | Replay JSON |
| POST | `/api/matches/solo` | Replay JSON | `{match_id}`（伺服器重跑驗證，seat 0 必須是自己） |
| GET | `/api/leaderboard?mode=classic&limit=50` | — | `[{user, elo, games}]` |
| GET | `/api/daily` | — | `{date, seed, config, my_best: number|null}` |
| POST | `/api/daily/submit` | Replay JSON | `{score, counted: bool, rank}` |
| GET | `/api/models` | — | `models/*.json` 內容陣列 |

靜態：`/models/*`（ONNX 檔）、其餘路徑回傳 `apps/web/dist`（SPA fallback 到 `index.html`）。
驗證：需要登入的 API 讀 `Authorization: Bearer <token>`，失敗回 `401 {error}`。

---

## 9. 資料庫（SQLite，`data/uno.db`，啟動時執行 migration）

```sql
CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  username TEXT UNIQUE,                -- 訪客為 NULL
  display_name TEXT NOT NULL,
  password_hash TEXT,                  -- 訪客為 NULL
  is_guest INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS sessions (
  token TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  expires_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS matches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mode_id TEXT NOT NULL,
  num_players INTEGER NOT NULL,
  is_solo INTEGER NOT NULL DEFAULT 0,
  seed INTEGER NOT NULL,
  replay_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS match_players (
  match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
  seat INTEGER NOT NULL,
  user_id INTEGER REFERENCES users(id),   -- bot 為 NULL
  bot_kind TEXT,
  rank INTEGER NOT NULL,                  -- 0 = 第一名
  score INTEGER NOT NULL,
  PRIMARY KEY (match_id, seat)
);
CREATE TABLE IF NOT EXISTS ratings (
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  mode_id TEXT NOT NULL,
  elo INTEGER NOT NULL DEFAULT 1200,
  games INTEGER NOT NULL DEFAULT 0,
  wins INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (user_id, mode_id)
);
CREATE TABLE IF NOT EXISTS rating_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL, mode_id TEXT NOT NULL, elo INTEGER NOT NULL,
  match_id INTEGER NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS daily_scores (
  user_id INTEGER NOT NULL REFERENCES users(id),
  date TEXT NOT NULL,
  score INTEGER NOT NULL,
  match_id INTEGER NOT NULL,
  PRIMARY KEY (user_id, date)
);
CREATE INDEX IF NOT EXISTS idx_mp_user ON match_players(user_id);
```

**Elo**：只在 ≥ 2 位真人的連線對局計算。對每對真人 (i, j)：`S = 1 if rank_i < rank_j, 0.5 if 相等, else 0`；`E = 1/(1+10^((R_j−R_i)/400))`；`ΔR_i = 32 × Σ(S−E) / (真人數−1)`，四捨五入。團隊模式以隊伍平均 Elo 計算，同隊成員變動相同。

---

## 10. 前端內部介面（`apps/web/src/game/`）

```ts
// GameClient：UI 唯一依賴的遊戲介面
export interface GameClient {
  readonly mySeat: number | null;
  subscribe(listener: (u: GameUpdate) => void): () => void;
  send(action: Action): void;            // 不合法時 client 會收到 error update
  nextRound(): void;
  leave(): void;
}
export type GameUpdate =
  | { kind: 'snapshot'; view: PlayerView; timer?: TurnTimer | null }
  | { kind: 'events'; events: Event[]; view: PlayerView; timer?: TurnTimer | null }
  | { kind: 'error'; message: string }
  | { kind: 'chat'; from: string; text: string }
  | { kind: 'emote'; seat: number; emote: string };

// LocalGameClient：Web Worker 中執行 WasmGame + WasmBot（避免 Hard bot 卡住 UI）
// RemoteGameClient：包裝 WebSocket（含自動重連：1s, 2s, 4s, 最多 10s 間隔）
```

- `useGameStore`（zustand）：`view`、`displayView`（動畫進行中顯示的畫面）、`animQueue`、`selectedCardId`、`timer`。
- `AnimationQueue`：`enqueue(events, finalView)`；逐一播放；全部播完後 `displayView = finalView`。
- Worker 訊息協議（`game.worker.ts`）：`{type:'init', config, seed, seats: ('human'|BotKind)[]}`、`{type:'action', action}`、`{type:'next_round'}` → 回傳與 `GameUpdate` 相同格式。
