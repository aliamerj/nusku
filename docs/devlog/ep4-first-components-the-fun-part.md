# Ep4 First component: the fun part (Agent)

So our first component is **Agent**: as we talked about, the agent in short is the component that runs next to a profiled process, collects data, and streams it out.

## Problems

Because there is no standard, good Zig libbpf available, I need to do everything myself (this is the fun part).

In Nusku I need to do this:

```
Linux process
     │
     │ "What is this process doing?"
     ▼
   Nusku
     │
     ▼
  eBPF program
     │
     ▼
  Linux kernel
     │
     ▼
 samples / profiling data
```

The important thing is that **Nusku needs a small program that runs inside the Linux kernel**.

That program is an **eBPF program**.

1. What is eBPF?

Think of eBPF as a way of giving the Linux kernel a small program.

Normally, your program runs in user space:

```
Your application
       │
       ▼
   Linux user space
```

But eBPF lets you put a restricted program inside the kernel:

```
Your application
       │
       │
       ▼
   Linux kernel
       │
       ├── eBPF program
       │
       └── kernel data
```

For Nusku, this is useful because the kernel can observe things that a normal user-space program can't observe as directly or efficiently.

For example, Nusku eventually wants to collect CPU samples.

```
CPU is running process X
          │
          ▼
     sampling event
          │
          ▼
     Nusku's eBPF
          │
          ▼
     record information
```

### Normally, libbpf would help us

But unfortunately, I tried many times in many ways to import it and get it to work in my project, but it did not give me the result I wanted, and I don't want to write C at all.

So that leaves me with looking at how it works and implementing the features I want in Zig, which turns out not to be that hard; I just need to understand some concepts.

### So what does libbpf do?

It can take an eBPF object file and do a lot of complicated work for us:

```
oncpu.bpf.o
      │
      ▼
    libbpf
      │
      ├── read ELF
      ├── understand maps
      ├── process relocations
      ├── prepare program
      ├── load program
      ├── handle verifier errors
      └── attach program
```

This is convenient.

But we can't use it now, so what we need first is a loader, which is my own small implementation in Zig.

Instead of asking libbpf to do the whole job, let's understand the object ourselves and implement the small amount of loading logic that Nusku actually needs.

## So what is a Loader?

Simply put, a loader is a piece of software whose job is to take something that isn't running yet and prepare/load it so it can run.

For example, when you run:

```
./my_program
```

**Linux has a loader involved**:

- It takes the executable file
- maps its required parts into memory
- prepares things like shared libraries, and starts the program

> For Nusku, we're doing something similar, but with an eBPF program.

That's our **BPF loader**.

```
                  Nusku
                    │
                    ▼
              ┌───────────┐
              │   Loader  │
              └───────────┘
                    │
                    ▼
             Linux kernel
                    │
                    ▼
             BPF program
                running
```

## What does our loader actually do?

```
oncpu.bpf.o
     │
     ▼
┌─────────────────────┐
│ Nusku BPF loader    │
│                     │
│ 1. Read object      │
│ 2. Find maps        │
│ 3. Create maps      │
│ 4. Find relocations │
│ 5. Patch references │
│ 6. Load BPF program │
│ 7. Attach program   │
└─────────────────────┘
     │
     ▼
 Linux kernel
```

So "loader" isn't a special Linux object or a special file. It's simply the name for the code we're writing that performs this loading process.

## Why do we need our own loader?

Normally, you could say:

```
oncpu.bpf.o
     │
     ▼
   libbpf
     │
     ▼
 Linux kernel
```

`libbpf` can act as the loader and handle much of this work.

But our plan is:

```
oncpu.bpf.o
     │
     ▼
 Nusku's Zig loader
     │
     ▼
 Linux kernel
```

We're deliberately implementing the small amount of loading logic Nusku needs ourselves.

The loader is our user-space Zig code whose job is to take the compiled eBPF object and get the eBPF program installed and running inside the kernel.

## Object files: the weird files with .o

These files are called object files: An object file is the **output produced by the compiler before the program is fully loaded/executed.**

For our Nusku loader, an object file is basically a **file containing a compiled BPF program plus information that tells us how that program is organized and what it still depends on.**

Think of it as a package:

```
oncpu.bpf.o
│
├── program instructions
├── maps information
├── relocation information
├── symbol information
└── other metadata
```

It is **not the running BPF program yet.**

### Why can't our loader just use the instructions?

```zig
const instructions: []const u8 = ...;
```

We would have the machine code, but we wouldn't know important things such as:

- Which bytes are the actual program?
- Which bytes describe maps?
- Which instruction references which map?
- Where are the relocations?
- What symbols exist?

The object file keeps all of this information together.

So instead of giving our loader just machine code, we give it `oncpu.bpf.o`, and our loader can inspect the whole package.

#### A simple Zig analogy

Imagine we had this Zig structure:

```
const BpfObject = struct {
    program: []const u8,
    maps: []const u8,
    relocations: []const u8,
};
```

You could think of the object file conceptually like this:

```
BPF object
┌─────────────────────────┐
│ program                 │
│                         │
│ maps                    │
│                         │
│ relocations             │
│                         │
│ symbols                 │
│                         │
│ other information       │
└─────────────────────────┘
```

The actual file is more sophisticated than this. It uses the **ELF format** to organize these pieces.

But conceptually, that's what we're dealing with.

Before we go deep on this, let's also understand the ELF format.

## What is ELF?

ELF stands for Executable and Linkable Format.

The easiest way to think about ELF is:

> ELF is a standard format for organizing compiled code and the information associated with that code inside a file.

It is a file format, not a programming language and not a loader.

For example, our file `oncpu.bpf.o` is an ELF file.

## Why do we need a format?

Imagine you compile something and get a binary file containing a bunch of bytes:

```
010101010101010101...
```

If our Zig loader receives only those bytes, how would it know:

- Where does the program start?
- Where are the instructions?
- Where are the maps?
- Where are the symbols?
- Where are the relocations?
- How large is each part?

There needs to be some agreed-upon structure.

That's what ELF provides.

Think of ELF as a container format with rules.

### What does ELF actually contain?

For our purposes, the most important pieces are:

```
ELF
│
├── Header
│
├── Sections
│   ├── perf_event
│   ├── .maps
│   ├── .rodata.cst4
│   ├── .rodata.cst8
│   └── ...
│
├── Symbol table
│
└── Relocation sections
```

We don't need to understand every ELF feature.

For Nusku, we're mainly interested in:

1. **ELF header**

Tells us basic information about the file.

For example:

```
"This is ELF"
"64-bit"
"little endian"
```

Our parser checks this.

2. **Sections**

This is where the interesting data lives.

We have things like:

```
perf_event
.maps
.rodata.cst4
.rodata.cst8
```

Think of sections as named areas inside the ELF file.

For example:

```
┌──────────────────────────────┐
│ ELF file                     │
│                              │
│ [perf_event]                 │
│ BPF instructions             │
│                              │
│ [.maps]                      │
│ map definitions              │
│                              │
│ [.rodata.cst4]               │
│ constants                    │
│                              │
│ [.symtab]                    │
│ symbols                      │
└──────────────────────────────┘
```

3. **Symbols**

ELF can contain information that gives names and locations to things.

For example:

```
on_cpu_sample
stacks
counts
target_pid
```

Our loader can use this information to understand what certain parts of the object refer to.

4. **Relocations**

ELF provides a standard way to say: This part of the compiled data refers to something that needs to be resolved later.

### The relationship between the three things

This is probably the most important thing to keep straight:

1. Object file

The file we have: `oncpu.bpf.o`

2. ELF

The format used to organize that file:

```
ELF
├── header
├── sections
├── symbols
├── relocations
└── ...
```

3. Loader

The Zig code we are writing that reads that file and eventually loads the BPF program:

```
Zig loader
    │
    │ reads
    ▼
oncpu.bpf.o
    │
    │ organized according to
    ▼
   ELF
```

So:

```
oncpu.bpf.o
│
│ is formatted as
▼
ELF
│
│ read by
▼
Zig loader
│
▼
Linux kernel
```

That's the relationship.

And one important correction to keep in mind: ELF itself does not make the BPF program run. It simply gives us a standardized way to organize and describe the compiled object so our loader can understand it.
