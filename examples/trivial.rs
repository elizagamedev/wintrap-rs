use std::sync::{Arc, Condvar, Mutex};
use wintrap;

fn main() {
    let pair = Arc::new((Mutex::new(true), Condvar::new()));
    let pair2 = pair.clone();

    wintrap::trap(
        vec![
            wintrap::Signal::CtrlC,
            wintrap::Signal::CtrlBreak,
            wintrap::Signal::CloseWindow,
        ],
        move |signal| {
            println!("Got \"{:?}\" signal!", signal);
            let &(ref lock, ref cvar) = &*pair2;
            let mut run = lock.lock().unwrap();
            *run = false;
            cvar.notify_one();
        },
        move || {
            println!("Press Ctrl-C, Ctrl-Break, or cleanly kill the process via WM_CLOSE.");
            let &(ref lock, ref cvar) = &*pair;
            let mut run = lock.lock().unwrap();
            while *run {
                run = cvar.wait(run).unwrap();
            }
            println!("It worked!");
        },
    )
    .unwrap();
}
