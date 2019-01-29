use futures::stream::Stream;
use tokio;
use wintrap;

fn main() {
    wintrap::trap_stream(
        &[
            wintrap::Signal::CtrlC,
            wintrap::Signal::CtrlBreak,
            wintrap::Signal::CloseWindow,
        ],
        |stream| {
            println!("Send a CtrlC, CtrlBreak, or CloseWindow signal.");
            let program = stream.take(1).for_each(|signal| {
                println!("Got a signal: {:?}", signal);
                Ok(())
            });
            tokio::run(program);
        },
    )
    .unwrap();
}
