---
name: 🎨 改善されたUIレスポンシブデザインの実装
about: 端末サイズに応じた最適化とスクロール機能の追加
title: '[Enhancement] 改善されたUIレスポンシブデザインの実装'
labels: enhancement, ui, user-experience
assignees: ''
---

## 概要

現在のTUIアプリケーションは固定レイアウトを使用しており、端末サイズに応じた最適化が不十分です。特に小さい端末や異なるアスペクト比での使用時に、ユーザビリティが低下する問題があります。

## 現在の問題

1. **固定の高さ制約**: `src/ui.rs`の`render`関数で以下のような固定値を使用
   ```rust
   .constraints([
       Constraint::Length(4),  // ヘッダ
       Constraint::Length(3),  // 入力欄
       Constraint::Length(3),  // 直近送信
       Constraint::Length(30), // AI回答
       Constraint::Min(0),     // 余白
   ])
   ```

2. **小さい端末での表示崩れ**: 端末の高さが50行未満の場合、UI要素が重なったり、スクロールができない

3. **AI回答の表示領域不足**: 長い回答が表示しきれず、ユーザーが内容を確認できない

## 提案する改善

### 1. 動的レイアウト調整
- 端末サイズに応じたConstraintの動的計算
- 最小サイズ要件の定義と警告表示

### 2. スクロール機能の実装
- AI回答エリアでの上下スクロール
- 履歴表示機能（過去の質問・回答の閲覧）

### 3. レスポンシブヘルプ表示
- 小さい端末では簡略化されたヘルプテキスト
- 動的なキーヒント表示

## 実装案

```rust
// src/ui.rs での改善例
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    
    // 端末サイズに応じた動的制約
    let constraints = if area.height < 20 {
        // 小さい端末用のコンパクトレイアウト
        vec![
            Constraint::Length(2),  // ヘッダ（簡略）
            Constraint::Length(3),  // 入力欄
            Constraint::Min(5),     // AI回答（スクロール対応）
        ]
    } else {
        // 通常サイズの端末用レイアウト
        vec![
            Constraint::Length(4),  // ヘッダ
            Constraint::Length(3),  // 入力欄
            Constraint::Length(3),  // 直近送信
            Constraint::Min(10),    // AI回答（可変）
            Constraint::Length(1),  // フッター
        ]
    };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    
    // ... 各コンポーネントの描画
}
```

## 期待される効果

1. **ユーザビリティの向上**: 様々な端末サイズでの使いやすさ
2. **アクセシビリティの改善**: 視覚障害者向けスクリーンリーダーとの互換性
3. **プロフェッショナルな外観**: モダンなTUIアプリケーションとしての品質向上

## 実装優先度

**Medium** - UIの改善はユーザー体験に直接影響するが、現在の機能は動作している

## 関連ファイル

- `src/ui.rs` - メインUI描画ロジック
- `src/app.rs` - スクロール状態管理の追加が必要
- `src/event.rs` - スクロール用キーハンドリングの追加

## 実装タスク

- [ ] 端末サイズ検出ロジックの実装
- [ ] 動的制約計算の実装
- [ ] スクロール状態管理の追加
- [ ] スクロール用キーバインドの追加
- [ ] 最小サイズ警告表示の実装
- [ ] レスポンシブヘルプ表示の実装
- [ ] 各端末サイズでのテスト実行