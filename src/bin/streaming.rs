use indicatif::{ProgressBar, ProgressStyle};
use pools_and_pipeline::my_actor_streamer_pool::{
    MyActorError, MyActorOutputMessage, MyActorStreamerPool,
};
use pools_and_pipeline::utils::InterruptIndicator;
use std::sync::Arc;
use tokio::sync::RwLock;

const N_TASKS: u64 = 8;

#[tokio::main]
async fn main() {
    // attach CTRL+C handler
    let interrupt_indicator = InterruptIndicator::new();

    // make pretty progress bar
    let pb = ProgressBar::new(N_TASKS);
    let sty = ProgressStyle::with_template(
        "{spinner:.cyan} [{bar:40.cyan/blue}] {pos:>7}/{len:7} [{elapsed_precise}<{eta_precise} {per_sec:.green}] {msg}"
    ).unwrap().progress_chars("#>-");
    pb.set_style(sty);

    // create pool
    let pool = Arc::new(RwLock::new(MyActorStreamerPool::new(4)));

    // submit tasks
    let pool_ = pool.clone();
    tokio::spawn(async move {
        for idx in 0..N_TASKS {
            if interrupt_indicator.is_set() {
                println!("Interrupted! Exiting gracefully...");
                break;
            }

            pool_
                .read()
                .await
                .send(idx)
                .await
                .expect("Failed to send message to pool");
        }
        println!("All tasks submitted. Closing pool.");
        pool_.write().await.close();
    });

    // wait for all tasks to finish
    let mut res_all = vec![];
    loop {
        let t = tokio::time::Instant::now();
        let Some(res) = pool.read().await.recv().await else {
            break;
        };
        match res {
            Ok(MyActorOutputMessage { idx: _idx, data }) => {
                res_all.push(data);
            }
            Err(MyActorError { idx }) => {
                println!("{idx} ended in {}ms", t.elapsed().as_millis());
                println!("idx {}: error", idx);
            }
        }
        pb.inc(1);
    }
    pb.finish();
    println!("res_all = {:?}", res_all);
}
