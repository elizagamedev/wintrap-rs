# wintrap-rs

The `wintrap` crate allows a Windows process to trap one or more abstracted
"signals", running an asynchronous callback function whenever they are caught
while active.

# Examples

```
wintrap::trap(vec![wintrap::Signal::CtrlC, wintrap::Signal::CloseWindow], |signal| {
    // handle signal here
    println!("Caught a signal: {:?}", signal);
}, || {
    // do work
    println!("Doing work");
}).unwrap();
```

# Caveats

Please note that it is not possible to correctly trap Ctrl-C signals when
running programs via `cargo run`. You will have to run them directly via the
target directory after building.
