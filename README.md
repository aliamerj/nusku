# Nusku
 
I've always wanted to see the complete performance picture before pushing any code or building something. Not after it's slow in production, not after a user complains, before. Click a button, see exactly what that click cost. Nusku is that, for native apps on Linux.
 
Point it at a running process, or launch one through it, and watch what's actually happening while you use the app. CPU time per function, live. Memory as it's allocated, not just a number that goes up, but what called for it and where. Threads that look idle but are actually stuck waiting on a lock, a disk read, or another process, and for how long. Syscalls that are quietly eating milliseconds you never see in a normal CPU graph. Network calls that are slow because packets are getting retransmitted, not because your code is doing anything wrong.
 
It runs locally with nothing to configure, you just point it at a process and watch. It can also run on a remote machine and stream the same live picture back to you over a secure connection, so you can watch a server the same way you'd watch something on your own laptop.
 
You can list everything running and worth watching, start watching any of it, stop whenever you want, and check what's currently being watched at any time. Everything you see is close to the moment it actually happened, not a report you read an hour later.
 
The goal is simple: stop guessing why something feels slow, and actually see it.
 
---
 
A note on why: this is me practicing Zig and systems programming, for real. I'm challenging myself to see how far I can take this. No AI generated slop, no shortcuts, just me, textbooks, and the Linux kernel docs, Back to old days :)
