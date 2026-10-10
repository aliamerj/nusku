---
date: 2026-10-08
---

### Ep5: First Measurement our Agent can do

After understanding the 4 core concepts for this project which are Loader, ELF, Object files + BPF (the core of the prject) 

we going to talk about BPF more here also

to keep it simple let's start with `on-CPU profiler` but there is more concepts to understand 

Imagine you have a program running:
```
my_program
    │
    ├── function A
    │     └── function B
    │           └── function C
    │
    └── function X
```
At any particular moment, the CPU is executing one instruction of that program.

Nusku wants to repeatedly ask:
> "What was this process doing when I looked at it?"

For example, 99 times per second:

```
sample 1 → A → B → C
sample 2 → A → B → C
sample 3 → A → B → D
sample 4 → A → X
sample 5 → A → B → C
...
```
After collecting thousands of samples, we can say:
```
B → C     72%
B → D     18%
A → X     10%
```
so sampling profiling does not trace every function call. 
- It interrupts the process many times per second
- records the call stack each time 
- counts how often each stack appears. 

A function that shows up in 30 percent of the samples is using about 30 percent of the CPU time. 

This costs very little and needs no changes to the profiled program.

That is the basic idea behind profiling.


## 1. Why does the kernel need to be involved?
The interesting question is:

> How do we get the stack of the process at the exact moment the CPU samples it?

The profiler cannot simply sit in user space and ask: "Hey process, give me your stack."

because the process might be:
- running
- sleeping
- inside a syscall
- executing kernel code
- executing user code

And we want the kernel to tell us what was happening when the CPU sample occurred.

Linux already has a mechanism for this:

### perf events
Conceptually:
```
CPU
 │
 │ periodically generates sample
 ▼
Linux perf event
 │
 ▼
BPF program runs
 │
 ▼
BPF asks:
"What process is this?"
"What is its user stack?"
"What is its kernel stack?"
```
So our BPF program becomes the small piece of code that executes **inside the kernel whenever a perf sample happens.**

So to run the program 99 times per second we use a **perf event**: 
- A kernel timer that fires per CPU. 
- Attaching our program to it makes the kernel run the program on every tick, on the CPU that is running at that moment

## 2. What exactly is the BPF program doing?
Very simply:

```
perf sample
     │
     ▼
BPF program starts
     │
     ▼
Which process generated this sample?
     │
     ▼
Is it the process Nusku wants to profile?
     │
   no ──────────────► stop
     │
    yes
     │
     ▼
Capture user stack
     │
     ▼
Capture kernel stack
     │
     ▼
Create a key describing this stack combination
     │
     ▼
Increment counter
     │
     ▼
stop
```
That's basically the entire Agent.

so **eBPF** is a program that runs inside the kernel. 
eBPF lets you load a small program into the Linux kernel and attach it to an event, 
such as a timer tick or a system call. 
The kernel runs it every time the event happens.

#### And other thing need to understand is
**The Verifier** :
before accepting a program, 
the kernel checks it: it must terminate, 
    - may not read memory it has not been given 
    - may only call approved functions. 

If the check fails the load is **refused and the kernel returns a log explaining why**. 

This is why kernel-side code is **restricted**:
- a **512-byte stack** 
- no unbounded loops 
- no calls to ordinary library functions (not even `memcpy`)



## 3. Why do we need to know the PID?
Suppose your CPU is running: Firefox, Terminal, Nusku, Docker, Postgres...etc

The perf event can fire while any of them is running.

But maybe the user says: `nusku prof --pid 1234` , we only care about process 1234. 

So every sample needs a check:

```
Who is currently running?

        ↓

PID = 1234?

   ┌────┴────┐
   │         │
  yes        no
   │         │
   ▼         ▼
capture    ignore
stack
```
This is the first filtering step.

## 4. Stack traces and frame pointers

The helper that captures a stack walks frame pointers: 
it follows a chain of saved base pointers up the stack. 

Programs compiled without frame pointers give short or empty user stacks. 

Zig in Debug mode keeps them. 
Go keeps them. 
Most C programs from distributions do not. 
**Kernel stacks do not depend on this**

### What is a stack?
This is extremely important for the rest of Nusku.
Imagine the program is executing:
```
main()
  └── handle_request()
        └── process_request()
              └── calculate()
                    └── expensive_function()
```
The call stack represents this chain.

Conceptually:
```
expensive_function()
calculate()
process_request()
handle_request()
main()
```
When we take a sample, we want to capture this stack.

Why?
Because a profiler doesn't just want: PID 1234 used CPU 

It wants: PID 1234 was executing:
```
main
 └── handle_request
      └── process_request
           └── calculate
                └── expensive_function
```
Then we can count how often each stack appears.

## 5. Why are there TWO stacks?
This is one of the important concepts for Nusku.

A Linux process can execute in:


### User space

Your application's code:
```
main()
foo()
bar()

```

or in:

### Kernel space
```
Linux kernel code:

your program
    ↓
syscall
    ↓
Linux kernel
    ↓
kernel functions
```

So at a sample we potentially have:
```
User stack:

main
 └── foo
      └── bar


Kernel stack:

sys_read
 └── vfs_read
      └── ...
```

Nusku wants both.

So one sample conceptually becomes:
```
User stack   = [main, foo, bar]
Kernel stack = [sys_read, vfs_read, ...]

```
This gives us much more useful profiling information.

## 6. We don't want to store the entire stack every time 
Imagine 99 samples per second.
After one minute: 99 × 60 = 5940 samples
And after an hour: 356,400 samples

If every sample stored all its stack addresses directly, we'd waste a lot of memory.

Instead, we use a stack table.

Conceptually:
```
stack ID 100 → [address1, address2, address3, ...]
stack ID 101 → [address4, address5, address6, ...]
stack ID 102 → [address7, address8, address9, ...]
```
Then a sample doesn't need to store the entire stack.

It can simply say:
```
user_stack_id   = 100
kernel_stack_id = 102
```
This is much cheaper.

### But why 99 Hz not 100 Hz ?

If you sample at exactly 100 Hz you can line up with the kernel's own 100 Hz timer and keep sampling the same moment of a repeating pattern. **99 Hz avoids that**.

## 7. What is a BPF map?

A **BPF map** is basically kernel-managed storage that a BPF program can read and write.

Think of it as a data structure living in the kernel.

For Nusku we need three pieces of storage.

### Map 1 : stacks
We need somewhere to store captured stacks:
```
stack ID
    ↓
stack addresses
```
For example:

```
100 → [0x401234, 0x401500, 0x402100, ...]
101 → [0x401234, 0x401600, 0x402100, ...]
```
### Map 2: counts
We need to count how many times each stack combination appeared.

So:
```
(user stack ID, kernel stack ID)
                ↓
              count
```
For example: 
```
(100, 200) → 723
(101, 200) → 152
(100, 201) → 89
```
This is the actual profiling data.

### Map 3: target PID
We need to tell the BPF program:

> "This is the process I currently want you to profile."

So conceptually:
```
target_pid
    ↓
1234
```
The BPF program checks this on every sample.

 
 **So we have three maps**

The architecture becomes:
```
                 BPF PROGRAM
                      │
          ┌───────────┼───────────┐
          │           │           │
          ▼           ▼           ▼
      target_pid    stacks      counts
          │           │           │
          │           │           │
       PID to      captured     number of
       profile      stacks       samples
```
And the flow is:
```
                    perf sample
                         │
                         ▼
                  BPF program runs
                         │
                         ▼
                  read target PID
                         │
                         ▼
               Is this the target?
                    /        \
                  no          yes
                  │             │
                stop            ▼
                         capture user stack
                                │
                                ▼
                        capture kernel stack
                                │
                                ▼
                        get stack IDs
                                │
                                ▼
                        create StackKey
                                │
                                ▼
                         counts[key]++
```

So **The Maps**
- A program has no global memory of its own. 
- It stores data in maps, which are key-value tables owned by the kernel and shared with userspace. 

Our program writes counters into maps, and the agent reads them out through file descriptors.

Map types used here: 
- array (fixed slots)
- hash (any key)
- stack-trace (a table the kernel fills with call stacks, indexed by a stack id)

## 8.What is StackKey conceptually?
We need a way to uniquely identify one profiling sample.

We could say:
```
PID + user stack + kernel stack
```
So conceptually:

```
StackKey = {
    process ID,
    user stack ID,
    kernel stack ID
}
```
For example:
```
{
    pid: 1234,
    user_stack: 100,
    kernel_stack: 200
}
```
Then:
```
counts[
    {1234, 100, 200}
] = 723
```

Meaning:

> We observed process 1234 with this particular user/kernel stack combination 723 times.

That's the heart of Nusku's profiling data.

## 9. What happens if we see the same stack again?

Suppose 
- the first sample gives: `{1234, 100, 200}`
- There isn't a counter yet: `counts[{1234,100,200}] = ?`
- So we create it: `counts[{1234,100,200}] = 1`
- Next sample: `{1234,100,200}`
- already exists: `counts[{1234,100,200}] = 2`
- Then: 
```
3
4
5
...
723
```
That's how sampling becomes profiling data.

## 11. Where does the BPF program get these capabilities?
This is where **BPF helpers** come in.

**The Helpers** :  

**The program cannot call normal functions ( cannot freely call arbitrary Linux functions) ** 

It calls **numbered kernel helpers**: 

for example 

- helper 1 looks up a map entry
- helper 14 returns the current process and thread ids 
- helper 27 captures the current stack


Instead, Linux exposes a controlled set of operations called BPF helpers.

Conceptually:
```
BPF program
     │
     ├── lookup map
     ├── update map
     ├── get current process
     └── get stack
```
Linux gives each helper a number.

So conceptually:

```
helper #1  → lookup something in a map
helper #2  → update a map
helper #14 → get current PID/TGID
helper #27 → get stack ID
```
The BPF program says: "I want helper 27"

and the kernel executes that operation.

## 12. What is the relationship between BPF and normal Zig?

We're using Zig to write the BPF program, but this is not a normal Zig program.

There are effectively two environments:

```
                Zig source
                   │
          ┌────────┴────────┐
          │                 │
          ▼                 ▼
     Nusku userspace     BPF program
       Zig program          Zig code
          │                   │
          ▼                   ▼
       Linux                  BPF
       process              execution
```
The BPF program is compiled specifically so Linux can execute it as eBPF.

It cannot simply do things like:
```
open()
malloc()
printf()
```
the way a normal Linux Zig application can.

It operates under the BPF execution model and uses BPF helpers.

## 13. What is the `.maps` section conceptually?

let's say our BPF source says:

- I need these three maps.
    - stacks
    - counts
    - target_pid

But **the userspace loader is going to create the actual kernel maps.**

So there is a separation:
```
BPF object
    │
    ├── "I need a stack map"
    ├── "I need a hash map"
    └── "I need an array map"
```
Then Nusku's loader says:
```
Okay.

I'll create those maps in the kernel.
```
So `.maps` is essentially a place inside the BPF object where we leave the description of the maps for the loader.

Think:
```
BPF object

.maps
 ├── stacks definition
 ├── counts definition
 └── target_pid definition
```
Then:
```
Nusku loader
      │
      ├── reads .maps
      │
      ├── creates actual kernel map #1
      ├── creates actual kernel map #2
      └── creates actual kernel map #3
```

## 14. Then there is one missing connection

The BPF program says:
```
use target_pid
use stacks
use counts
```
But the actual kernel maps don't exist yet when the object is compiled.

They will be created later by Nusku.

So we have:
```
BPF program
      │
      │ "I need stacks"
      ▼
   ??? reference
      │
      ▼
loader creates actual map
      │
      ▼
kernel map FD
```
The loader needs to connect these two worlds. That's what relocations are for.

Conceptually:
```
BPF object
   │
   │ relocation says:
   │ "this instruction refers to this map"
   ▼
Nusku loader
   │
   │ creates map
   ▼
real kernel map
   │
   │ loader patches reference
   ▼
BPF program ready to load
```
## 15. The complete picture

Now put everything together:

```
                 BPF SOURCE
                     │
                     ▼
             compiled BPF object
                     │
        ┌────────────┼─────────────┐
        │            │             │
        ▼            ▼             ▼
     program       .maps       relocations
        │            │             │
        │            │             │
        └────────────┼─────────────┘
                     │
                     ▼
              Nusku loader
                     │
             ┌───────┼────────┐
             │       │        │
             ▼       ▼        ▼
          read     create   resolve
         program   maps    references
             │       │        │
             └───────┼────────┘
                     ▼
                Linux kernel
                     │
                     ▼
              loaded BPF program
                     │
                     ▼
                perf event
                     │
                     ▼
              BPF runs ~99 Hz
                     │
                     ▼
              capture stacks
                     │
                     ▼
                count them
                     │
                     ▼
              Nusku reads data
```

### The three most important ideas to remember
#### 1. BPF program
> Runs in the kernel whenever a perf sample happens and collects profiling information.

#### 2. BPF maps
> Kernel-side storage used to hold stacks, sample counts, and the PID we're profiling.

#### 3. Loader
> Runs in userspace, creates the real maps, connects the BPF program's map references to those maps, loads the program, and eventually attaches it to perf.
