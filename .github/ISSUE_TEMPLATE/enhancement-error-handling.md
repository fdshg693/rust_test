---
name: 🔧 エラーハンドリングと復旧機能の強化
about: OpenAI API接続エラーやネットワーク障害からの自動復旧機能
title: '[Enhancement] エラーハンドリングと復旧機能の強化'
labels: enhancement, reliability, error-handling
assignees: ''
---

## 概要

現在のアプリケーションでは、OpenAI APIの接続エラーやネットワーク障害が発生した場合の復旧機能が不十分です。ユーザーがエラー状態から回復するための手段が限られており、アプリケーションの信頼性に影響を与えています。

## 現在の問題

1. **エラー状態の不明瞭さ**: `src/event.rs`でエラーを無視
   ```rust
   let _ = app.tx.send(to_send); // ワーカーが終了している場合は送信エラーを無視
   ```

2. **ワーカースレッドの死活監視不足**: OpenAIワーカーがクラッシュしても検出できない

3. **ユーザーへのフィードバック不足**: 
   - ネットワークエラーの詳細が表示されない
   - リトライ機能がない
   - エラー状態からの回復方法が不明

4. **ログ情報の不十分さ**: エラーの根本原因分析が困難

## 提案する改善

### 1. 包括的エラーハンドリング

```rust
// src/app.rs での改善例
#[derive(Debug, Clone)]
pub enum AppError {
    NetworkError(String),
    ApiError { code: u16, message: String },
    WorkerDisconnected,
    ConfigurationError(String),
    TimeoutError,
}

pub struct App {
    // 既存フィールド...
    pub error_state: Option<AppError>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_error_time: Option<Instant>,
}
```

### 2. 自動復旧機能

```rust
impl App {
    pub fn handle_error(&mut self, error: AppError) {
        self.error_state = Some(error.clone());
        self.last_error_time = Some(Instant::now());
        
        match error {
            AppError::NetworkError(_) | AppError::ApiError { .. } => {
                if self.retry_count < self.max_retries {
                    self.schedule_retry();
                }
            },
            AppError::WorkerDisconnected => {
                self.restart_worker();
            },
            _ => {}
        }
    }
    
    fn restart_worker(&mut self) {
        // ワーカースレッドの再起動ロジック
        info!(target: "app", "Restarting OpenAI worker due to disconnection");
        let config = Config::new();
        openai::start_openai_worker(self.rx.clone(), self.tx.clone(), config);
    }
    
    fn schedule_retry(&mut self) {
        self.retry_count += 1;
        // 指数バックオフでリトライ間隔を調整
        let delay = std::time::Duration::from_secs(2_u64.pow(self.retry_count.min(5)));
        // リトライスケジューリング実装
    }
}
```

### 3. ユーザーフレンドリーなエラー表示

```rust
// src/ui.rs での改善
fn render_error_panel(f: &mut Frame, error: &AppError, area: Rect) {
    let (error_text, actions) = match error {
        AppError::NetworkError(msg) => (
            format!("🌐 接続エラー: {}", msg),
            "[R]リトライ | [C]キャンセル | [H]ヘルプ"
        ),
        AppError::ApiError { code, message } => (
            format!("🚨 API エラー ({}): {}", code, message),
            "[S]設定確認 | [R]リトライ | [C]キャンセル"
        ),
        AppError::WorkerDisconnected => (
            "⚠️ バックグラウンドワーカーが停止しました".to_string(),
            "[R]再起動 | [Q]終了"
        ),
        AppError::TimeoutError => (
            "⏱️ リクエストタイムアウト".to_string(),
            "[R]リトライ | [C]キャンセル"
        ),
        _ => (
            "予期しないエラーが発生しました".to_string(),
            "[R]リトライ | [Q]終了"
        ),
    };
    
    let error_widget = Paragraph::new(vec![
        Line::from(error_text),
        Line::from(""),
        Line::from(actions),
    ])
    .block(Block::default()
        .borders(Borders::ALL)
        .title("⚠️ エラー")
        .border_style(Style::default().fg(Color::Red)))
    .style(Style::default().fg(Color::Yellow))
    .wrap(Wrap { trim: true });
    
    f.render_widget(error_widget, area);
}
```

### 4. ヘルスチェック機能

```rust
// src/openai/worker.rs での改善
#[derive(Debug, Clone)]
pub struct WorkerHealth {
    pub last_heartbeat: Instant,
    pub request_count: u64,
    pub error_count: u64,
    pub status: WorkerStatus,
    pub uptime: Duration,
}

#[derive(Debug, Clone)]
pub enum WorkerStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Stopped,
}

impl WorkerHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, WorkerStatus::Healthy) &&
        self.last_heartbeat.elapsed() < Duration::from_secs(30)
    }
}
```

## 実装計画

### Phase 1: エラー検出と表示 (Week 1)
- [ ] `AppError`型の定義と実装
- [ ] エラー状態のUI表示
- [ ] 基本的なエラーログ機能
- [ ] エラー処理の単体テスト

### Phase 2: 自動復旧 (Week 2)
- [ ] リトライロジックの実装
- [ ] ワーカー再起動機能
- [ ] 設定検証機能
- [ ] 指数バックオフの実装

### Phase 3: 監視とヘルスチェック (Week 3)
- [ ] ヘルスチェック機能
- [ ] メトリクス収集
- [ ] 詳細なエラー分析
- [ ] 統合テストの実装

## 期待される効果

1. **アプリケーションの信頼性向上**: エラーからの自動復旧
2. **ユーザー体験の改善**: 明確なエラーメッセージとアクション指示
3. **運用性の向上**: 問題の迅速な特定と解決
4. **プロダクション対応**: 実用レベルのエラーハンドリング

## 実装優先度

**High** - アプリケーションの安定性と信頼性に直接関わる重要な機能

## 関連ファイル

- `src/app.rs` - エラー状態管理
- `src/event.rs` - エラー処理ロジック
- `src/openai/worker.rs` - ワーカーヘルスチェック
- `src/ui.rs` - エラー表示UI
- `src/config.rs` - エラー設定パラメータ

## テスト計画

- [ ] ネットワーク障害のシミュレーション
- [ ] API制限エラーのテスト
- [ ] ワーカースレッドクラッシュのテスト
- [ ] 自動復旧機能の検証
- [ ] リトライロジックのテスト
- [ ] エラーUI表示のテスト

## 受け入れ基準

- ネットワークエラー時に適切なエラーメッセージが表示される
- 自動リトライが設定回数まで実行される
- ワーカーの停止が検出され、自動再起動される
- ユーザーが手動でエラー状態から復旧できる
- エラーの詳細がログに記録される