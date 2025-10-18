use rand::{thread_rng, Rng};
use serde::{Serialize, Deserialize};

use super::models::*;
use super::rules::RpgRules;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub player: Player,
    pub enemy: Enemy,
    pub turn: Turn,
    pub battle_count: usize,
    pub rules: RpgRules,
    pub is_over: bool,
}

#[derive(Debug)]
pub struct Game {
    pub rules: RpgRules,
    player: Player,
    enemy: Enemy,
    turn: Turn,
    battle_count: usize,
}

impl Game {
    pub fn new() -> Self {
        let rules = RpgRules::default();
        Self::with_rules(rules)
    }

    pub fn with_rules(rules: RpgRules) -> Self {
        let name = std::env::var("RPG_PLAYER").unwrap_or_else(|_| "Hero".into());
        Self {
            player: Player::new_with_rules(name, &rules),
            enemy: Enemy::random_with_rules(&rules),
            turn: if thread_rng().gen_bool(0.5) { Turn::Player } else { Turn::Enemy },
            battle_count: 1,
            rules,
        }
    }

    pub fn is_over(&self) -> bool { self.player.hp <= 0 }

    pub fn player(&self) -> &Player { &self.player }
    pub fn enemy(&self) -> &Enemy { &self.enemy }
    pub fn battle_count(&self) -> usize { self.battle_count }
    pub fn turn(&self) -> Turn { self.turn }

    pub fn snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            player: self.player.clone(),
            enemy: self.enemy.clone(),
            turn: self.turn,
            battle_count: self.battle_count,
            rules: self.rules.clone(),
            is_over: self.is_over(),
        }
    }

    pub fn handle_command(&mut self, cmd: Command) -> Result<bool> {
        match cmd {
            Command::Quit => return Ok(false),
            Command::Run => {
                if thread_rng().gen_bool(self.rules.run_success_rate) {
                    println!("You ran away!");
                    self.next_battle();
                } else {
                    println!("Couldn't escape!");
                    self.turn = Turn::Enemy;
                }
            }
            Command::Heal => {
                let healed = self.player.heal(&self.rules);
                if healed > 0 { println!("You used a potion and healed {healed} HP."); } else { println!("No potions left!"); }
                self.turn = Turn::Enemy;
            }
            Command::Attack => {
                let dmg = player_attack_damage(self.player.atk);
                println!("You hit the {} for {dmg} damage!", self.enemy.name);
                self.enemy.hp -= dmg;
                if self.enemy.hp <= 0 {
                    self.victory();
                } else {
                    self.turn = Turn::Enemy;
                }
            }
        }

        if self.turn == Turn::Enemy && self.player.hp > 0 {
            self.enemy_turn();
        }
        Ok(true)
    }

    fn enemy_turn(&mut self) {
        let dmg = enemy_attack_damage(self.enemy.atk);
        println!("{} hits you for {dmg} damage!", self.enemy.name);
        self.player.hp -= dmg;
        if self.player.hp <= 0 {
            println!("You were defeated... Game Over.");
        } else {
            self.turn = Turn::Player;
        }
    }

    fn victory(&mut self) {
        println!("You defeated the {}!", self.enemy.name);
        println!("You found {} gold.", self.enemy.gold_reward);
        self.player.gold += self.enemy.gold_reward;
        if thread_rng().gen_bool(self.rules.potion_drop_rate) {
            self.player.potions += 1;
            println!("You found a potion!");
        }
        self.next_battle();
    }

    fn next_battle(&mut self) {
        self.enemy = Enemy::random_with_rules(&self.rules);
        self.turn = Turn::Player;
        self.battle_count += 1;
        println!("A wild {} appears!", self.enemy.name);
    }
}
