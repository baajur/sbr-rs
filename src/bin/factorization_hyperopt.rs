extern crate csv;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate wheedle;
extern crate wyrm;
#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io::{BufReader, Read};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use rand::distributions::{IndependentSample, Range};
// use rand::{Rng, SeedableRng, XorShiftRng};

use wheedle::data::{train_test_split, user_based_split, Interaction, Interactions,
                    TripletInteractions};
use wheedle::evaluation::mrr_score;
use wheedle::models::factorization;

#[derive(Deserialize, Serialize)]
struct GoodbooksInteraction {
    user_id: usize,
    book_id: usize,
    rating: usize,
}

fn load_goodbooks(path: &str) -> Interactions {
    let mut reader = csv::Reader::from_path(path).unwrap();
    let interactions: Vec<Interaction> = reader
        .deserialize::<GoodbooksInteraction>()
        .map(|x| x.unwrap())
        .enumerate()
        .map(|(i, x)| Interaction::new(x.user_id, x.book_id, i))
        .take(100_000)
        .collect();

    Interactions::from(interactions)
}

#[derive(Debug, Serialize, Deserialize)]
struct Result {
    test_mrr: f32,
    train_mrr: f32,
    elapsed: Duration,
    hyperparameters: factorization::Hyperparameters,
}

fn load_movielens(path: &str) -> Interactions {
    let mut reader = csv::Reader::from_path(path).unwrap();
    let interactions: Vec<Interaction> = reader.deserialize().map(|x| x.unwrap()).collect();

    Interactions::from(interactions)
}

fn fit(
    train: &TripletInteractions,
    hyper: factorization::Hyperparameters,
) -> factorization::ImplicitFactorizationModel {
    let mut model = factorization::ImplicitFactorizationModel::new(hyper);
    model.fit(train).unwrap();

    model
}

fn main() {
    //let mut data = load_movielens("data.csv");
    let mut data = load_goodbooks("ratings.csv");
    let mut rng = rand::thread_rng();

    let (mut train, test) = user_based_split(&mut data, &mut rng, 0.2);
    //let (mut train, test) = train_test_split(&mut data, &mut rng, 0.2);

    train.shuffle(&mut rng);

    for _ in 0..100 {
        let mut results: Vec<Result> = File::open("factorization_results.json")
            .map(|file| serde_json::from_reader(&file).unwrap())
            .unwrap_or(Vec::new());

        let hyper = factorization::Hyperparameters::random(&mut rng);

        let start = Instant::now();
        let model = fit(&train.to_triplet(), hyper.clone());
        let result = Result {
            train_mrr: mrr_score(&model, &train.to_compressed()).unwrap(),
            test_mrr: mrr_score(&model, &test.to_compressed()).unwrap(),
            elapsed: start.elapsed(),
            hyperparameters: hyper,
        };

        println!("{:#?}", result);

        if !result.test_mrr.is_nan() {
            results.push(result);
            results.sort_by(|a, b| a.test_mrr.partial_cmp(&b.test_mrr).unwrap());
        }

        println!("Best result: {:#?}", results.last());

        File::create("factorization_results.json")
            .map(|file| serde_json::to_writer_pretty(&file, &results).unwrap())
            .unwrap();
    }
}