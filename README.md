# Simple Rust pseudoterminal-based login session example to read user input and pty/terminal output of a terminal session

I have been playing around with different OSX utilities for developer UX for a few years. Mostly stuff similar to [Flashlight](https://github.com/nate-parrott/Flashlight).

I saw this new OSX application [Fig](https://fig.io/) and was wondering how they were getting the information from the terminal.

I saw they have a few executables and I am pretty sure their _figterm_ binary does the same thing as a project of mine.

I simplified out the relevant code from my project into this one so it is super easy to follow.

The idea would be that you have your terminal emulator or shell running this binary when they start so that it is always running.

This binary would then be sending the user input and pty/shell/process output of each open terminal to something read by another process which is responsible

for doing something like analyzing the input and output and doing something useful. Maybe offering to autocomplete the command you are typing?

# What would be next?

I left comments starting with

```
    HOOK HERE
```

where you can take the data being typed and the data from the pty/shell/process and send that to something read by another process.

Maybe your GUI process is doing the analysis of this information and then showing you something that can enhance the development process.

# OS Support

This should work on both Linux and OSX.

# TODO

- Check isatty and if not, don't do this whole thing

# Credits

Again, I have learned a TON from nw0's [session-manager](https://github.com/nw0/session-manager). Great project..
