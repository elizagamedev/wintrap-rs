# wintrap-rs

The `wintrap` crate allows a Windows process to trap one or more abstracted
"signals", running an asynchronous callback function whenever they are caught
while active.

# Examples

```
wintrap::trap(&[wintrap::Signal::CtrlC, wintrap::Signal::CloseWindow], |signal| {
    // handle signal here
    println!("Caught a signal: {:?}", signal);
}, || {
    // do work
    // println!("Doing work");
}).unwrap();
```
