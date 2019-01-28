#![feature(async_await, await_macro, futures_api)]

use std::future::Future;
use std::pin::Pin;
use std::task::{LocalWaker, Poll};

use futures::channel::mpsc::{self, Receiver, Sender};
use futures::executor::{self, ThreadPool};
use futures::future::{self, FutureExt};
use futures::sink::SinkExt;
use futures::stream::{self, Stream, StreamExt};
use futures::task::SpawnExt;

#[allow(dead_code)]
type MyReceiver = Receiver<Result<String, ()>>;
type MySender = Sender<Result<String, ()>>;

pub struct JobFuture(Pin<Box<dyn Future<Output = ()> + Send>>);

impl JobFuture {
    fn new<S: Stream<Item = Result<String, ()>> + Send + 'static>(inner: S, tx: MySender) -> Self {
        let future = inner
            .map(|result| Ok(result))
            .forward(tx.sink_map_err(|_| ()))
            .map(|_| ());

        JobFuture(Box::pin(future))
    }
}

impl Future for JobFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output> {
        Future::poll(Pin::new(&mut self.0), lw)
    }
}

fn main() {
    executor::block_on(async {
        let (tx, mut rx) = mpsc::channel(4);

        let deps = vec![
            JobFuture::new(stream::once(future::ok("first".into())), tx.clone()),
            JobFuture::new(stream::once(future::ok("second".into())), tx.clone()),
        ];

        let final_fut = future::join_all(deps)
            .then(move |_| JobFuture::new(stream::once(future::ok("final".into())), tx));

        let mut pool = ThreadPool::new().unwrap();
        pool.spawn(final_fut).unwrap();

        for entry in await!(rx.next()) {
            println!("{:?}", entry);
        }
    });
}
