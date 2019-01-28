use futures_old::future::{self, Future};
use futures_old::sink::Sink;
use futures_old::stream::{self, Stream};
use futures_old::sync::mpsc::{self, Receiver, Sender};
use futures_old::Poll;

#[allow(dead_code)]
type MyReceiver = Receiver<Result<String, ()>>;
type MySender = Sender<Result<String, ()>>;

pub struct JobFuture(Box<dyn Future<Item = (), Error = ()> + Send>);

impl JobFuture {
    fn new<S: Stream<Item = String, Error = ()> + Send + 'static>(inner: S, tx: MySender) -> Self {
        let future = inner
            .map(|result| Ok(result))
            .forward(tx.sink_map_err(|_| ()))
            .map(|_| ());

        JobFuture(Box::new(future))
    }
}

impl Future for JobFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

fn main() {
    tokio::run(
        future::lazy(|| {
            let (tx, rx) = mpsc::channel(4);

            let deps = vec![
                JobFuture::new(stream::once(Ok("first".into())), tx.clone()),
                JobFuture::new(stream::once(Ok("second".into())), tx.clone()),
            ];

            let final_fut = future::join_all(deps)
                .then(move |_| JobFuture::new(stream::once(Ok("final".into())), tx));

            tokio::spawn(final_fut);

            Ok(rx)
        })
        .flatten_stream()
        .for_each(|e| {
            println!("{:?}", e);
            Ok(())
        }),
    );
}
