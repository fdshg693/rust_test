use rand::{thread_rng, Rng};
use serde::{Serialize, Deserialize};

use super::rules::RpgRules;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub atk: i32,
    pub potions: i32,
    pub gold: i32,
}

impl Player {
    pub fn new_with_rules(name: String, rules: &RpgRules) -> Self {
        Self { 
            name, 
            hp: rules.player_default_max_hp, 
            max_hp: rules.player_default_max_hp, 
            atk: rules.player_default_atk, 
            potions: rules.player_default_potions, 
            gold: rules.player_default_gold,
        }
    }
    pub fn heal(&mut self, rules: &RpgRules) -> i32 {
        if self.potions <= 0 { return 0; }
        self.potions -= 1;
        let heal = thread_rng().gen_range(rules.heal_min..=rules.heal_max);
        self.hp = (self.hp + heal).min(self.max_hp);
        heal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enemy {
    pub name: String,
    pub hp: i32,
    pub atk: i32,
    pub gold_reward: i32,
}

impl Enemy {
    pub fn random_with_rules(rules: &RpgRules) -> Self {
        let mut rng = thread_rng();
        let idx = rng.gen_range(0..rules.enemy_templates.len());
        let t = &rules.enemy_templates[idx];
        Self { name: t.name.clone(), hp: t.hp, atk: t.atk, gold_reward: t.gold_reward }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Turn { Player, Enemy }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command { Attack, Heal, Run, Quit }

pub fn player_attack_damage(atk: i32) -> i32 {
    let mut rng = thread_rng();
    let variance = rng.gen_range(-1..=2);
    (atk + variance).max(1)
}

pub fn enemy_attack_damage(atk: i32) -> i32 {
    let mut rng = thread_rng();
    let variance = rng.gen_range(-1..=1);
    (atk + variance).max(1)
}
