//! RpgService
//! 
//! RPGゲームロジックのビジネスロジック層。
//! UI層（TUI/Web）から独立した形でゲーム機能を提供。

use crate::rpg::{Command, game::{Game, GameSnapshot}};
use serde::{Serialize, Deserialize};
use color_eyre::Result;

/// RPGゲームサービス
#[derive(Debug, Serialize, Deserialize)]
pub struct RpgService {
    #[serde(skip)]
    game: Option<Game>,
    // シリアライズ用のスナップショット（Webセッション用）
    snapshot: Option<GameSnapshot>,
}

impl RpgService {
    /// 新しいRpgServiceインスタンスを作成
    pub fn new() -> Self {
        let game = Game::new();
        let snapshot = game.snapshot();
        Self { 
            game: Some(game),
            snapshot: Some(snapshot),
        }
    }

    /// コマンドを実行してゲーム状態を更新
    /// 
    /// # Arguments
    /// * `cmd` - 実行するコマンド（Attack, Heal, Run, Quit）
    /// 
    /// # Returns
    /// 実行後のゲームスナップショット、またはエラー
    pub fn execute_command(&mut self, cmd: Command) -> Result<GameSnapshot> {
        if let Some(game) = &mut self.game {
            // handle_commandはboolを返す（falseはゲーム終了）
            let should_continue = game.handle_command(cmd)
                .map_err(|e| color_eyre::eyre::eyre!("Command execution failed: {}", e))?;
            
            let snapshot = game.snapshot();
            self.snapshot = Some(snapshot.clone());
            
            if !should_continue {
                // Quit コマンドの場合
                return Ok(snapshot);
            }
            
            Ok(snapshot)
        } else {
            Err(color_eyre::eyre::eyre!("Game not initialized"))
        }
    }

    /// 現在のゲーム状態のスナップショットを取得
    pub fn snapshot(&self) -> GameSnapshot {
        if let Some(snapshot) = &self.snapshot {
            snapshot.clone()
        } else if let Some(game) = &self.game {
            game.snapshot()
        } else {
            // フォールバック（通常は到達しない）
            let game = Game::new();
            game.snapshot()
        }
    }

    /// ゲームが終了しているかチェック
    pub fn is_over(&self) -> bool {
        if let Some(game) = &self.game {
            game.is_over()
        } else {
            self.snapshot.as_ref().map(|s| s.is_over).unwrap_or(true)
        }
    }

    /// ゲームをリセット
    pub fn reset(&mut self) {
        let game = Game::new();
        self.snapshot = Some(game.snapshot());
        self.game = Some(game);
    }

    /// プレイヤー名を取得
    pub fn player_name(&self) -> String {
        self.snapshot()
            .player
            .name
    }

    /// 現在のバトル回数を取得
    pub fn battle_count(&self) -> usize {
        self.snapshot().battle_count
    }
}

impl Default for RpgService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpg_service_creation() {
        let service = RpgService::new();
        assert!(!service.is_over());
        assert_eq!(service.battle_count(), 1);
    }

    #[test]
    fn test_rpg_service_reset() {
        let mut service = RpgService::new();
        // 何回かアタック
        let _ = service.execute_command(Command::Attack);
        let _ = service.execute_command(Command::Attack);
        
        service.reset();
        assert_eq!(service.battle_count(), 1);
    }

    #[test]
    fn test_execute_attack_command() {
        let mut service = RpgService::new();
        let result = service.execute_command(Command::Attack);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_heal_command() {
        let mut service = RpgService::new();
        let result = service.execute_command(Command::Heal);
        assert!(result.is_ok());
    }
}
