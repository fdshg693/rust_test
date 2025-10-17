# コーヒーゲーム - 実装概略

## モジュール構成

```
src/
├── coffee_game/              # ゲーム全体モジュール
│   ├── mod.rs               # 公開インターフェース & イベントハンドリング
│   ├── requirements.md       # 要件定義
│   ├── implementations.md    # このファイル
│   ├── state.rs             # ゲーム状態管理 (CoffeeGameApp)
│   ├── models.rs            # データモデル (CoffeeMaker, Customer, etc.)
│   ├── simulation.rs        # ゲームロジック・シミュレーション
│   └── ui.rs                # UI レンダリング
└── main.rs / lib.rs         # 既存の TUI フレームワーク統合
```

---

## 各モジュールの詳細設計

### 1. `models.rs` - データモデル

#### `CoffeeMaker`
```rust
pub struct CoffeeMaker {
    id: usize,
    temperature: f32,           // 現在の温度 (℃)
    is_heating: bool,           // 加熱中か
    current_customer: Option<Customer>,  // 現在対応中の客
    total_served: usize,        // 対応客数の統計
}
```

#### `Customer`
```rust
pub struct Customer {
    id: usize,
    arrival_time: f32,          // 到着時刻（ゲーム開始からの秒数）
    wait_start_time: f32,       // 列に入った時刻
}
```

#### `CoffeeTemperatureRating`
```rust
pub enum CoffeeTemperatureRating {
    TooHot,       // 50℃未満 -> 0点
    Cold,         // 50℃～60℃ -> 20点
    Cool,         // 60℃～70℃ -> 50点
    Perfect,      // 70℃～80℃ -> 100点
    Hot,          // 80℃～90℃ -> 150点
}

pub fn calculate_score(temperature: f32) -> u32 { ... }
pub fn rate_temperature(temperature: f32) -> CoffeeTemperatureRating { ... }
```

#### `GameEvent`
```rust
pub enum GameEvent {
    CustomerArrived(Customer),
    CoffeeDispensed { maker_id: usize, temperature: f32, score: u32 },
    MakerHeatingStarted(usize),
    MakerHeatingEnded(usize),
}
```

---

### 2. `state.rs` - ゲーム状態管理

#### `CoffeeGameApp`
```rust
pub struct CoffeeGameApp {
    // ゲーム基本情報
    elapsed_time: f32,          // 経過時間（秒）
    total_score: u32,           // 累積得点
    game_active: bool,          // ゲーム実行中フラグ
    
    // ゲーム要素
    coffee_makers: Vec<CoffeeMaker>,  // 複数台のメーカー
    customer_queue: VecDeque<Customer>,  // 待機客の列
    next_customer_id: usize,    // 次の客ID
    
    // 時間管理（客到着のランダム性）
    time_since_last_customer: f32,  // 前回の客到着からの経過時間
    next_customer_interval: f32,    // 次の客到着までの時間（2～5秒のランダム値）
    
    // UI 関連
    selected_maker_id: Option<usize>,  // 現在選択中のメーカーID
    last_action_message: Option<String>,  // 最後のアクション結果メッセージ
    
    // 統計情報
    total_customers_served: usize,
    events: Vec<GameEvent>,     // 最近のイベント履歴
}

impl CoffeeGameApp {
    pub fn new(num_makers: usize) -> Self { ... }
    pub fn tick(&mut self, dt: f32) { ... }  // ゲームロジック更新
    pub fn select_maker(&mut self, id: usize) { ... }
    pub fn set_coffee(&mut self) -> Result<String> { ... }
    pub fn dispense_coffee(&mut self) -> Result<String> { ... }
    pub fn get_maker_status(&self) -> String { ... }
    pub fn is_game_active(&self) -> bool { ... }
}
```

**主な責務**:
- ゲーム状態の保持
- アクション結果メッセージの生成
- ゲーム終了判定

---

### 3. `simulation.rs` - ゲームロジック

#### 温度シミュレーション
```rust
pub fn update_temperature(
    maker: &mut CoffeeMaker,
    dt: f32,  // デルタタイム（秒）
) {
    const HEATING_RATE: f32 = 5.0;    // ℃/秒
    const COOLING_RATE: f32 = 1.0;    // ℃/秒
    const MAX_TEMP: f32 = 90.0;
    const MIN_TEMP: f32 = 0.0;
    
    if maker.is_heating {
        maker.temperature = (maker.temperature + HEATING_RATE * dt).min(MAX_TEMP);
        if maker.temperature >= MAX_TEMP {
            // 最高温度に達した → 加熱停止、一定温度を維持
            maker.is_heating = false;
        }
    } else {
        // 冷却
        maker.temperature = (maker.temperature - COOLING_RATE * dt).max(MIN_TEMP);
    }
}
```

#### 客到着ロジック
```rust
pub fn try_spawn_customer(
    app: &mut CoffeeGameApp,
    dt: f32,
    rng: &mut impl Rng,
) {
    app.time_since_last_customer += dt;
    
    if app.time_since_last_customer >= app.next_customer_interval {
        let new_customer = Customer {
            id: app.next_customer_id,
            arrival_time: app.elapsed_time,
            wait_start_time: app.elapsed_time,
        };
        app.customer_queue.push_back(new_customer);
        app.next_customer_id += 1;
        
        // 次の客到着時間を更新（2～5秒のランダム）
        app.next_customer_interval = rng.gen_range(2.0..5.0);
        app.time_since_last_customer = 0.0;
    }
}
```

#### コーヒー提供ロジック
```rust
pub fn dispense_coffee(
    maker: &mut CoffeeMaker,
    customer: Customer,
) -> u32 {
    let score = calculate_score(maker.temperature);
    maker.temperature = 0.0;  // 温度リセット
    maker.is_heating = false;
    maker.current_customer = None;
    maker.total_served += 1;
    
    // イベント記録
    score
}
```

### 4. `mod.rs` - モジュール公開インターフェース

```rust
pub mod models;
pub mod state;
pub mod simulation;
pub mod ui;

pub use state::CoffeeGameApp;
pub use models::{CoffeeMaker, Customer, GameEvent};

// コーヒーゲーム用のイベント処理
pub fn handle_coffee_game_key(app: &mut CoffeeGameApp, key: KeyCode) {
    match key {
        KeyCode::Char('1')..=KeyCode::Char('9') => {
            let id = (key as usize) - ('1' as usize);
            app.select_maker(id);
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            if let Ok(msg) = app.set_coffee() {
                info!("{}", msg);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Ok(msg) = app.dispense_coffee() {
                info!("{}", msg);
            }
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            let status = app.get_maker_status();
            info!("{}", status);
        }
        _ => {}
    }
}
```
## 実装の段階的アプローチ

### Phase 1: 基礎モデル
1. `models.rs` で `CoffeeMaker`, `Customer`, スコア計算を実装
2. `state.rs` で `CoffeeGameApp` の基本機能を実装
3. 単体テストで動作確認

### Phase 2: ゲームロジック
1. `simulation.rs` で温度更新・客到着・スコア計算を実装
2. `state.rs` に `tick()` メソッドを統合
3. 統合テストで各システムが協調動作するか確認

### Phase 3: イベントハンドリング
1. `mod.rs` で `handle_coffee_game_key()` を実装
2. 既存の `event.rs` と統合

---

