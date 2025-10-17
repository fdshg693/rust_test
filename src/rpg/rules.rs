use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyTemplate {
    pub name: String,
    pub hp: i32,
    pub atk: i32,
    pub gold_reward: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpgRules {
    pub game_name: String,
    pub player_default_max_hp: i32,
    pub player_default_atk: i32,
    pub player_default_potions: i32,
    pub player_default_gold: i32,
    pub heal_min: i32,
    pub heal_max: i32,
    pub run_success_rate: f64,
    pub potion_drop_rate: f64,
    pub enemy_templates: Vec<EnemyTemplate>,
}

impl Default for RpgRules {
    fn default() -> Self {
        Self {
            game_name: "Tiny CLI RPG".to_string(),
            player_default_max_hp: 30,
            player_default_atk: 5,
            player_default_potions: 2,
            player_default_gold: 0,
            heal_min: 8,
            heal_max: 15,
            run_success_rate: 0.5,
            potion_drop_rate: 0.3,
            enemy_templates: vec![
                EnemyTemplate { name: "Slime".into(), hp: 12, atk: 3, gold_reward: 6 },
                EnemyTemplate { name: "Goblin".into(), hp: 18, atk: 4, gold_reward: 9 },
                EnemyTemplate { name: "Wolf".into(), hp: 22, atk: 5, gold_reward: 12 },
            ],
        }
    }
}
