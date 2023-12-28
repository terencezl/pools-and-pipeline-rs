use tokio::sync::oneshot;

enum ActorMessage {
    GetUniqueId { respond_to: oneshot::Sender<u32> },
}

struct MyActor {
    receiver: async_channel::Receiver<ActorMessage>,
    next_id: u32,
}

impl MyActor {
    fn new(receiver: async_channel::Receiver<ActorMessage>) -> Self {
        MyActor {
            receiver,
            next_id: 0,
        }
    }

    async fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::GetUniqueId { respond_to } => {
                self.next_id += 1;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // The `let _ =` ignores any errors when sending.
                //
                // This can happen if the `select!` macro is used
                // to cancel waiting for the response.
                let _ = respond_to.send(self.next_id);
            }
        }
    }

    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }
}

#[derive(Clone)]
pub struct MyActorPool {
    sender: async_channel::Sender<ActorMessage>,
}

impl MyActorPool {
    pub fn new(num_actors: usize) -> Self {
        let (sender, receiver) = async_channel::bounded(8);
        for _ in 0..num_actors {
            let receiver = receiver.clone();
            tokio::spawn(async move { MyActor::new(receiver).run().await });
        }

        Self { sender }
    }

    pub async fn get_unique_id(&self) -> u32 {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetUniqueId { respond_to: send };

        // Ignore send errors. If this send fails, so does the
        // recv.await below. There's no reason to check the
        // failure twice.
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}
