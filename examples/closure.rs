
use std::cell::Cell;

fn main() {
    // 1. 基本のクロージャー：引数と戻り値の型は推論可能
    let add_one = |x: i32| x + 1;
    println!("add_one(5) = {}", add_one(5)); // 6
    println!("add_one(10) = {}", add_one(10)); // 11

    // 3. 複数引数、複数式ブロック
    let describe = |name: &str, age: u8| {
        println!("Name: {}, Age: {}", name, age);
        age >= 20
    };
    let is_adult = describe("Alice", 30);
    println!("Is adult? {}", is_adult);

    // 4. 環境のキャプチャ（イミュータブル）
    let mut factor = 3;
    let multiply = |x| x * factor;
    println!("multiply(7) = {}", multiply(7)); // 21
    factor = 5; // factor を変更
    println!("{}", factor); // 5
    //println!("multiply(7) = {}", multiply(7)); // もう使えない（factor はイミュータブルキャプチャ）

    // 5. 可変キャプチャ（FnMut）とクロージャーによる状態変更
    let mut counter = 0;
    {
        let mut inc_counter = |by: i32| {
            counter += by; // counter を可変借用して変更
            println!("counter = {}", counter);
        };
        inc_counter(2); // counter = 2
        inc_counter(5); // counter = 7
    }
    // inc_counter のスコープを抜けると借用解除

    // 6. 所有権を奪うキャプチャ（FnOnce）と move クロージャー
    let s = String::from("Hello");
    let consume_and_print_str = move || {
        // move によって s の所有権がクロージャーに移動
        println!("{}", s);
        // ここで s はドロップされる
    };
    consume_and_print_str();
    // println!("{}", s); // もう使えない（所有権は移動済み）

    let count = 0;
    let consume_and_print_int = move || {
        println!("Count: {}", count);
    };
    consume_and_print_int();
    println!("{}", count); 
    consume_and_print_int();


    // 8. 関数からクロージャーを返す：Box<dyn Fn…>
    fn make_adder(n: i32) -> Box<dyn Fn(i32) -> i32> {
        Box::new(move |x| x + n)
    }
    let add_five = make_adder(5);
    println!("add_five(3) = {}", add_five(3)); // 8

    // Cell<i32> は Copy、かつ内部だけを書き換えられる
    let factor2 = Cell::new(3);
    let multiply = |x: i32| x * factor2.get();

    println!("{}", multiply(5)); // 15
    factor2.set(4);              // 内部をミュータブルに更新
    println!("{}", multiply(5)); // 20

}