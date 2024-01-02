use indicatif::{ProgressBar, ProgressStyle};
use pools_and_pipeline::my_actor_pool::MyActorPool;
use pools_and_pipeline::utils::InterruptIndicator;
use std::sync::Arc;
use tokio::sync::Semaphore;

const N_TASKS: u64 = 8;

#[tokio::main]
async fn main() {
    // attach CTRL+C handler
    let interrupt_indicator = InterruptIndicator::new();

    // create pool
    let pool = MyActorPool::new(4);

    // submitter & receiver comm
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);

    tokio::spawn(async move {
        // concurrency control
        let sem = Arc::new(Semaphore::new(4));

        for _i in 0..N_TASKS {
            if interrupt_indicator.is_set() {
                println!("Interrupted! Exiting gracefully...");
                break;
            }

            let permit = sem.clone().acquire_owned().await.unwrap();

            let pool = pool.clone();
            let sender = sender.clone();
            tokio::spawn(async move {
                let _permit = permit; // own the permit

                // let t = tokio::time::Instant::now();
                // println!("{i} starting...");
                let res = pool.get_unique_id().await;
                // println!("{i} ended in {}ms", t.elapsed().as_millis());
                sender.send(res).await.unwrap();
            });
        }
    });

    // wait for all tasks to finish

    // make pretty progress bar
    let pb = ProgressBar::new(N_TASKS);
    let sty = ProgressStyle::with_template(
        "{spinner:.cyan} [{bar:40.cyan/blue}] {pos:>7}/{len:7} [{elapsed_precise}<{eta_precise} {per_sec:.green}] {msg}"
    ).unwrap().progress_chars("#>-");
    pb.set_style(sty);

    let mut res_all = vec![];
    while let Some(res) = receiver.recv().await {
        res_all.push(res);
        pb.inc(1);
    }
    pb.finish();
    println!("res_all = {:?}", res_all);
}
