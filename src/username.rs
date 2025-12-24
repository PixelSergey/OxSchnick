use rand::prelude::*;

pub fn generate_username() -> String {
    let mut rng = rand::rng();
    let weapon = ["Rock", "Paper", "Scissors"].choose(&mut rng).unwrap();

    let suffix: String = (0..6)
        .map(|_| rand::random_range(0..10))
        .map(|n| char::from_digit(n, 10).unwrap())
        .collect();

    format!("{} fan #{}", weapon, suffix)
}
