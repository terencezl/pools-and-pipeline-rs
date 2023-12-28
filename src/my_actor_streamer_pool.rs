use log::error;
use thiserror::Error;

pub struct MyActorInputMessage {
    pub idx: u64,
}

pub struct MyActorOutputMessage {
    pub idx: u64,
    pub data: u32,
}

#[derive(Error, Debug)]
#[error("Actor error: {idx}")]
pub struct MyActorError {
    pub idx: u64,
}

struct MyActor {
    in_receiver: async_channel::Receiver<MyActorInputMessage>,
    out_sender: Option<async_channel::Sender<Result<MyActorOutputMessage, MyActorError>>>,
    next_id: u32,
}

impl MyActor {
    fn new(
        in_receiver: async_channel::Receiver<MyActorInputMessage>,
        out_sender: async_channel::Sender<Result<MyActorOutputMessage, MyActorError>>,
    ) -> Self {
        MyActor {
            in_receiver,
            out_sender: Some(out_sender),
            next_id: 0,
        }
    }

    async fn handle_message(&mut self, msg: MyActorInputMessage) {
        let MyActorInputMessage { idx } = msg;
        self.next_id += 1;
        let next_id = self.next_id;
        let res = async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            anyhow::Ok(MyActorOutputMessage { idx, data: next_id })
        }
        .await
        .map_err(|e| {
            error!("MyActor: idx {}: error: {}", idx, e);
            MyActorError { idx }
        });

        match &self.out_sender {
            // The `let _ =` ignores any errors when sending.
            //
            // This can happen if the `select!` macro is used
            // to cancel waiting for the response.
            Some(out_sender) => {
                let _ = out_sender.send(res).await;
            }
            None => {
                error!("MyActor: idx {}: error: pool is closed.", idx);
            }
        }
    }

    async fn run(&mut self) {
        while let Ok(msg) = self.in_receiver.recv().await {
            self.handle_message(msg).await;
        }
        self.out_sender.take();
    }
}

pub struct MyActorStreamerPool {
    in_sender: Option<async_channel::Sender<MyActorInputMessage>>,
    out_receiver: async_channel::Receiver<Result<MyActorOutputMessage, MyActorError>>,
}

impl MyActorStreamerPool {
    pub fn new(num_actors: usize) -> Self {
        let (in_sender, in_receiver) = async_channel::bounded(8);
        let (out_sender, out_receiver) = async_channel::bounded(8);
        for _ in 0..num_actors {
            let in_receiver = in_receiver.clone();
            let out_sender = out_sender.clone();
            tokio::spawn(async move { MyActor::new(in_receiver, out_sender).run().await });
        }

        Self {
            in_sender: Some(in_sender),
            out_receiver,
        }
    }

    pub async fn send(
        &self,
        idx: u64,
    ) -> Result<(), async_channel::SendError<MyActorInputMessage>> {
        let msg = MyActorInputMessage { idx };
        match &self.in_sender {
            Some(in_sender) => in_sender.send(msg).await,
            None => Err(async_channel::SendError(msg)),
        }
    }

    pub fn send_blocking(
        &self,
        idx: u64,
    ) -> Result<(), async_channel::SendError<MyActorInputMessage>> {
        let msg = MyActorInputMessage { idx };
        match &self.in_sender {
            Some(in_sender) => in_sender.send_blocking(msg),
            None => Err(async_channel::SendError(msg)),
        }
    }

    pub async fn recv(&self) -> Option<Result<MyActorOutputMessage, MyActorError>> {
        self.out_receiver.recv().await.ok()
    }

    pub fn recv_blocking(&self) -> Option<Result<MyActorOutputMessage, MyActorError>> {
        self.out_receiver.recv_blocking().ok()
    }

    pub fn close(&mut self) {
        self.in_sender.take();
    }
}
