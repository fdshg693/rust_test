# Web移行プラン - CLIとWebアプリでロジック共通化

## 目的
現在のRust TUIアプリケーションをWebアプリケーションとしても利用可能にする。コアロジック（OpenAI API連携、RPGゲームロジック、SQLite操作）を共通化し、UI層（TUI/Web）のみを切り替え可能にする。

## 現状分析

### 既存アーキテクチャ
```
rust_test/
├── src/
│   ├── lib.rs              # TUI特化のメインループ
│   ├── main.rs             # TUIエントリーポイント
│   ├── config.rs           # 共通設定（移行可能）
│   ├── openai/             # OpenAI API ロジック（共通化可能）
│   │   ├── worker.rs       # スレッドベースワーカー
│   │   ├── simple.rs
│   │   ├── history.rs
│   │   └── call/          # ツール呼び出しロジック
│   ├── rpg/               # RPGゲームロジック（完全共通化可能）
│   │   ├── game.rs
│   │   ├── models.rs
│   │   ├── rules.rs
│   │   └── ui.rs          # TUI専用（Web版は別実装）
│   ├── sqlite/            # データベース（共通化可能）
│   └── modes/             # TUI専用モードシステム
│       ├── menu.rs
│       ├── openai_chat.rs
│       └── rpg_game.rs
```

### 依存関係の分類

#### 🟢 完全共通化可能（Web/CLI両対応）
- `config.rs` - 設定管理
- `openai/` - OpenAI API連携ロジック全体
- `rpg/game.rs`, `rpg/models.rs`, `rpg/rules.rs` - ゲームロジック
- `sqlite/` - データベース操作

#### 🟡 一部調整必要
- `openai/worker.rs` - スレッド管理をより汎用的に

#### 🔴 UI層特化（別実装が必要）
- `src/lib.rs` - TUIメインループ
- `modes/*` - ratatui依存のMode trait/実装
- `rpg/ui.rs` - ratatuiレンダリング

---

## 移行戦略：3フェーズアプローチ

### Phase 1: コアロジックの抽出と再構成 ✅（優先度：高）

#### 1.1 ディレクトリ構造の再編成

```
rust_test/
├── Cargo.toml              # ワークスペース化
├── crates/
│   ├── rust_test_core/     # 共通ロジッククレート
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs
│   │       ├── services/   # ビジネスロジック層
│   │       │   ├── mod.rs
│   │       │   ├── chat_service.rs
│   │       │   └── rpg_service.rs
│   │       ├── openai/     # 現行のopenai/を移動
│   │       ├── rpg/        # 現行のrpg/（ui.rs除く）を移動
│   │       └── sqlite/     # 現行のsqlite/を移動
│   │
│   ├── rust_test_cli/      # TUIアプリクレート
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── lib.rs      # 現行のlib.rs
│   │       ├── modes/      # 現行のmodes/
│   │       └── ui/
│   │           └── rpg_ui.rs  # 現行のrpg/ui.rs
│   │
│   └── rust_test_web/      # Webアプリクレート（新規）
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs     # Axumサーバー
│           ├── handlers/   # HTTPハンドラ
│           ├── models/     # DTOs
│           └── templates/  # HTMLテンプレート
```

#### 1.2 サービス層の設計

**`crates/rust_test_core/src/services/chat_service.rs`**
```rust
use crate::openai::{multi_step_tool_answer_with_logger, MultiStepAnswer};
use crate::openai::tools::build_number_guess_tool;
use crate::config::Config;

pub struct ChatService {
    config: Config,
}

impl ChatService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// プロンプトを送信してAI回答を取得（同期版）
    pub async fn get_response(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let tools = vec![build_number_guess_tool(8, 10)];
        
        let answer = multi_step_tool_answer_with_logger(
            prompt, 
            &tools, 
            &self.config, 
            Some(10),
            |ev| tracing::info!(target="chat_service", event=%ev, "step")
        ).await?;
        
        Ok(answer.final_answer)
    }

    /// ストリーミング版（将来的にWebSocketで使用）
    pub async fn get_response_stream<F>(&self, prompt: &str, callback: F) 
    -> Result<String, Box<dyn std::error::Error>>
    where F: Fn(&str) + Send + 'static 
    {
        // OpenAI APIのstreaming機能を利用
        todo!("Implement streaming response")
    }
}
```

**`crates/rust_test_core/src/services/rpg_service.rs`**
```rust
use crate::rpg::{Game, Command, GameSnapshot};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RpgService {
    game: Game,
}

impl RpgService {
    pub fn new() -> Self {
        Self { game: Game::new() }
    }

    pub fn execute_command(&mut self, cmd: Command) -> Result<GameSnapshot, String> {
        match self.game.exec_command(cmd) {
            Ok(_) => Ok(self.game.snapshot()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn snapshot(&self) -> GameSnapshot {
        self.game.snapshot()
    }

    pub fn is_over(&self) -> bool {
        self.game.is_over()
    }

    pub fn reset(&mut self) {
        self.game = Game::new();
    }
}
```

---

### Phase 2: Webバックエンドの構築 🚧（優先度：中）

#### 2.1 技術スタック選定

**推奨構成：**
- **Webフレームワーク**: [Axum](https://github.com/tokio-rs/axum) 
  - 理由: tokioベース、async-openaiとの親和性が高い、型安全
- **セッション管理**: [tower-sessions](https://github.com/maxcountryman/tower-sessions)
- **テンプレートエンジン**: [Askama](https://github.com/djc/askama) または [Tera](https://github.com/Keats/tera)
- **WebSocket**: Axumの`extract::ws`（リアルタイムチャット用）

**依存関係追加（`crates/rust_test_web/Cargo.toml`）:**
```toml
[dependencies]
rust_test_core = { path = "../rust_test_core" }
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1.43", features = ["full"] }
tower = "0.4"
tower-sessions = "0.12"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
askama = "0.12"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

#### 2.2 WebサーバーAPI設計

**エンドポイント構成：**

```
[メニュー・ナビゲーション]
GET  /                          # ホーム（メニュー）

[OpenAI Chat]
GET  /chat                      # チャット画面
POST /api/chat                  # 通常リクエスト（JSON）
WS   /api/chat/stream           # ストリーミング（WebSocket）

[RPG Game]
GET  /rpg                       # ゲーム画面
POST /api/rpg/action            # アクション実行（JSON）
GET  /api/rpg/state             # 状態取得
POST /api/rpg/reset             # ゲームリセット

[静的ファイル]
GET  /static/*                  # CSS/JS
```

#### 2.3 実装例

**`crates/rust_test_web/src/main.rs`**
```rust
use axum::{
    routing::{get, post},
    Router,
};
use tower_sessions::{MemoryStore, SessionManagerLayer};
use std::time::Duration;

mod handlers;
mod models;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(tower_sessions::Expiry::OnInactivity(Duration::from_secs(3600)));

    let app = Router::new()
        .route("/", get(handlers::home))
        .route("/chat", get(handlers::chat_page))
        .route("/api/chat", post(handlers::chat_api))
        .route("/api/chat/stream", get(handlers::chat_stream))
        .route("/rpg", get(handlers::rpg_page))
        .route("/api/rpg/action", post(handlers::rpg_action))
        .route("/api/rpg/state", get(handlers::rpg_state))
        .route("/api/rpg/reset", post(handlers::rpg_reset))
        .layer(session_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Server running on http://localhost:3000");
    axum::serve(listener, app).await?;
    Ok(())
}
```

**`crates/rust_test_web/src/handlers/chat.rs`**
```rust
use axum::{
    extract::State,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use rust_test_core::{Config, services::ChatService};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
}

pub async fn chat_api(
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let config = Config::new();
    let service = ChatService::new(config);
    
    match service.get_response(&req.prompt).await {
        Ok(response) => Ok(Json(ChatResponse { response })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

**`crates/rust_test_web/src/handlers/rpg.rs`**
```rust
use axum::{
    extract::{State, Json},
    http::StatusCode,
};
use tower_sessions::Session;
use serde::{Deserialize, Serialize};
use rust_test_core::{rpg::Command, services::RpgService};

const RPG_SESSION_KEY: &str = "rpg_service";

#[derive(Deserialize)]
pub struct RpgActionRequest {
    pub action: String, // "attack", "heal", "run", "quit"
}

pub async fn rpg_action(
    session: Session,
    Json(req): Json<RpgActionRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut service: RpgService = session
        .get(RPG_SESSION_KEY)
        .await
        .unwrap_or_default()
        .unwrap_or_else(|| RpgService::new());

    let cmd = match req.action.as_str() {
        "attack" => Command::Attack,
        "heal" => Command::Heal,
        "run" => Command::Run,
        "quit" => Command::Quit,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    match service.execute_command(cmd) {
        Ok(snapshot) => {
            session.insert(RPG_SESSION_KEY, &service).await.ok();
            Ok(Json(serde_json::to_value(snapshot).unwrap()))
        }
        Err(e) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn rpg_state(session: Session) -> Result<Json<serde_json::Value>, StatusCode> {
    let service: RpgService = session
        .get(RPG_SESSION_KEY)
        .await
        .unwrap_or_default()
        .unwrap_or_else(|| RpgService::new());
    
    Ok(Json(serde_json::to_value(service.snapshot()).unwrap()))
}

pub async fn rpg_reset(session: Session) -> Result<StatusCode, StatusCode> {
    let mut service = RpgService::new();
    service.reset();
    session.insert(RPG_SESSION_KEY, &service).await.ok();
    Ok(StatusCode::OK)
}
```

---

### Phase 3: フロントエンド実装とデプロイ 🎨（優先度：低）

#### 3.1 フロントエンド選択肢

**Option A: サーバーサイドレンダリング（SSR）**
- Askama/TeraでHTMLテンプレート
- Alpine.js/HTMX で動的要素
- 軽量、SEO対応、初期構築が速い

**Option B: SPA（Single Page Application）**
- React/Vue/Svelte + TypeScript
- Axumは純粋なAPIサーバーに
- リッチなUI、モダンな開発体験

**推奨**: まずOption Aで実装し、必要に応じてOption Bへ

#### 3.2 テンプレート例（Askama）

**`templates/chat.html`**
```html
<!DOCTYPE html>
<html>
<head>
    <title>OpenAI Chat</title>
    <script src="https://unpkg.com/alpinejs@3.x.x/dist/cdn.min.js"></script>
</head>
<body x-data="chatApp()">
    <div class="container">
        <h1>OpenAI Chat</h1>
        <div id="messages" x-html="messages"></div>
        <form @submit.prevent="sendMessage">
            <input type="text" x-model="input" placeholder="メッセージを入力...">
            <button type="submit" :disabled="loading">送信</button>
        </form>
    </div>
    
    <script>
        function chatApp() {
            return {
                input: '',
                messages: '',
                loading: false,
                async sendMessage() {
                    if (!this.input.trim()) return;
                    this.loading = true;
                    const response = await fetch('/api/chat', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ prompt: this.input })
                    });
                    const data = await response.json();
                    this.messages += `<p><b>You:</b> ${this.input}</p>`;
                    this.messages += `<p><b>AI:</b> ${data.response}</p>`;
                    this.input = '';
                    this.loading = false;
                }
            }
        }
    </script>
</body>
</html>
```

#### 3.3 WebSocket実装（ストリーミングチャット）

```rust
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};

pub async fn chat_stream(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let axum::extract::ws::Message::Text(text) = msg {
            let config = Config::new();
            let service = ChatService::new(config);
            
            // ストリーミング応答（部分的な結果を逐次送信）
            match service.get_response(&text).await {
                Ok(response) => {
                    socket.send(axum::extract::ws::Message::Text(response)).await.ok();
                }
                Err(_) => {
                    socket.send(axum::extract::ws::Message::Text("Error".into())).await.ok();
                }
            }
        }
    }
}
```

#### 3.4 デプロイ戦略

**開発環境:**
```bash
# CLIアプリ
cd crates/rust_test_cli
cargo run

# Webアプリ
cd crates/rust_test_web
cargo run
# -> http://localhost:3000
```

**本番環境選択肢:**
1. **Railway/Render**: Dockerベースの簡単デプロイ
2. **Fly.io**: エッジロケーション対応
3. **AWS EC2/ECS**: フルコントロール
4. **Shuttle.rs**: Rust特化のプラットフォーム

**Dockerfile例:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p rust_test_web

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/rust_test_web /usr/local/bin/
ENV RUST_LOG=info
EXPOSE 3000
CMD ["rust_test_web"]
```

---

## 実装ロードマップ

### Week 1-2: Phase 1 完了
- [ ] Cargoワークスペース化
- [ ] `rust_test_core`クレート作成
- [ ] サービス層実装（`ChatService`, `RpgService`）
- [ ] 既存TUIアプリを`rust_test_cli`に移行し動作確認

### Week 3-4: Phase 2 開始
- [ ] `rust_test_web`クレート作成
- [ ] Axumサーバー基本構成
- [ ] ChatAPI実装（POST `/api/chat`）
- [ ] RPG API実装（基本4エンドポイント）
- [ ] セッション管理導入

### Week 5-6: Phase 3 フロントエンド
- [ ] HTMLテンプレート作成（Askama）
- [ ] Alpine.js で動的UI実装
- [ ] WebSocketストリーミング対応（オプション）
- [ ] CSSスタイリング

### Week 7-8: テスト・デプロイ
- [ ] 統合テスト追加
- [ ] Dockerイメージ作成
- [ ] 本番デプロイ（Render/Railway）
- [ ] ドキュメント整備

---

## 技術的考慮事項

### セキュリティ
- [ ] OPENAI_API_KEYの安全な管理（環境変数、Secrets Manager）
- [ ] CSRF対策（tower-http の CsrfLayer）
- [ ] Rate limiting（tower-governor）
- [ ] 入力バリデーション

### パフォーマンス
- [ ] OpenAI API呼び出しのタイムアウト設定
- [ ] コネクションプーリング（SQLite）
- [ ] 静的ファイルのキャッシング
- [ ] 非同期処理の最適化

### 監視・ログ
- [ ] tracingの統合（CLI/Web両対応）
- [ ] エラートラッキング（Sentry等）
- [ ] メトリクス収集（Prometheus）

### テスト戦略
- [ ] Coreロジックのユニットテスト（既存継続）
- [ ] WebAPI統合テスト（axum-test）
- [ ] E2Eテスト（Playwright/Selenium）

---

## 代替アプローチ

### Alternative 1: FFI/Wasm
Rustコアをライブラリ化し、他言語から呼び出し
- **メリット**: 既存Webスタック活用可能
- **デメリット**: FFI境界のオーバーヘッド、複雑性増加

### Alternative 2: gRPC
CLI/WebがgRPCサーバーを共有
- **メリット**: 強い型付け、多言語対応
- **デメリット**: ブラウザから直接gRPCは困難（gRPC-Web必要）

### Alternative 3: Tauri
デスクトップアプリ化（Web技術でUI）
- **メリット**: クロスプラットフォーム、リッチUI
- **デメリット**: サーバー不要のローカルアプリに限定

---

## 成功指標

- ✅ `rust_test_core`が単体でビルド・テスト可能
- ✅ CLI版（`rust_test_cli`）が現行と同等の機能を提供
- ✅ Web版（`rust_test_web`）がOpenAI Chat/RPG Gameの基本機能を提供
- ✅ コードの重複率 < 10%（共通ロジック部分）
- ✅ APIレスポンスタイム < 2秒（OpenAI呼び出し除く）
- ✅ 本番環境で安定稼働（Uptime > 99%）

---

## 参考リソース

### 公式ドキュメント
- [Axum](https://docs.rs/axum/)
- [Tokio](https://tokio.rs/)
- [Askama](https://djc.github.io/askama/)

### サンプルプロジェクト
- [realworld-axum-sqlx](https://github.com/launchbadge/realworld-axum-sqlx) - Axumフルスタック例
- [leptos-realworld](https://github.com/leptos-rs/leptos/tree/main/examples/todo_app_sqlite_axum) - Leptos+Axum統合例

### ベストプラクティス
- [Zero To Production In Rust](https://www.zero2prod.com/) - Rustでのバックエンド開発
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

---

## まとめ

本プランでは、**段階的なリファクタリング**により、既存のTUIアプリケーションを破壊することなくWeb対応を実現します。

**コア原則:**
1. **ロジックとUIの分離**: サービス層で共通化
2. **段階的移行**: Phase 1完了後に既存機能が保持される
3. **型安全性**: Rustの強みを活かしたAPI設計
4. **拡張性**: 将来的なモバイルアプリ化やマイクロサービス化も視野

**次のステップ**: Phase 1のワークスペース化から着手することを推奨します。
