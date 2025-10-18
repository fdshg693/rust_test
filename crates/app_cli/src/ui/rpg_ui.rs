use crate::core::rpg::{game::{Game, Result}, Command};
use std::io::{self, Write};

pub fn title() {
    println!("============================");
    println!("   Tiny CLI RPG (demo)   ");
    println!("============================\n");
}

pub fn goodbye() {
    println!("\nThanks for playing! Bye.");
}

pub fn show_status(game: &Game) {
    let p = game.player();
    let e = game.enemy();
    println!("Battle #{}", game.battle_count());
    println!("Player: {} | HP: {}/{} | ATK: {} | Potions: {} | Gold: {}", p.name, p.hp, p.max_hp, p.atk, p.potions, p.gold);
    println!("Enemy:  {} | HP: {} | ATK: {}", e.name, e.hp, e.atk);
    println!("Turn:   {:?}", game.turn());
}

pub fn prompt_action() -> Result<Command> {
    print!("[A]ttack, [H]eal, [R]un, [Q]uit > ");
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    let c = s.trim().to_ascii_lowercase();
    let cmd = match c.as_str() {
        "a" | "attack" => Command::Attack,
        "h" | "heal" => Command::Heal,
        "r" | "run" => Command::Run,
        "q" | "quit" | "exit" => Command::Quit,
        _ => {
            println!("Unknown command. Try a/h/r/q.");
            return prompt_action();
        }
    };
    Ok(cmd)
}
