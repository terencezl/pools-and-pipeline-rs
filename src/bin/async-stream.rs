use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

const N_TASKS: usize = 8;
const CONCURRENT_REQUESTS_1: usize = 4;
const CONCURRENT_REQUESTS_2: usize = 4;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();

    let writer = Rc::new(RefCell::new(BufWriter::new(
        File::create("output.txt").await?,
    )));

    let urls = vec!["https://api.ipify.org"; N_TASKS];

    stream::iter(urls)
        .enumerate()
        .map(|(idx, url)| {
            let client = &client;
            async move {
                let t = std::time::Instant::now();
                println!("Stage 1 - {idx}: Fetching {}...", url);
                let resp = client.get(url).send().await?;
                let bytes = resp.bytes().await?;
                println!("Stage 1 - {idx}: Took {:.2}ms", t.elapsed().as_millis());
                anyhow::Ok((idx, bytes))
            }
        })
        .buffered(CONCURRENT_REQUESTS_1)
        .map(|res| async move {
            match res {
                Ok((idx, b)) => {
                    let t = std::time::Instant::now();
                    println!("Stage 2 - {idx}: Starting...");
                    let s = String::from_utf8(b.to_vec())?;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    println!("Stage 2 - {idx}: Took {:.2}ms", t.elapsed().as_millis());
                    anyhow::Ok((idx, s))
                }
                Err(e) => Err(e),
            }
        })
        .buffered(CONCURRENT_REQUESTS_2)
        .for_each(|res| async {
            match res {
                Ok((idx, s)) => {
                    println!("Finally: {idx}: Got string with length {}", s.len());
                    writer
                        .borrow_mut()
                        .write_all(format!("{}\n", s).as_bytes())
                        .await
                        .unwrap();
                }
                Err(e) => eprintln!("Finally: Got an error: {}", e),
            }
        })
        .await;

    writer.borrow_mut().flush().await?;

    Ok(())
}
