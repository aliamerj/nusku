---
date: 2026-10-09
---

# Ep6: Advance ZIG features we need 

Before continuing with the BPF logic, let's separate the Zig language features from what the profiler is doing. 
Otherwise you'll end up understanding the profiler concept but getting stuck on syntax like `extern`, `linksection`, `@ptrFromInt`, `@sizeOf`, `@truncate`, `pointers`, `callconv`, etc.


# Quick reference for the Zig features used in the Nusku eBPF loader/program.

---

## 1. `extern struct`

```zig
const MapDef = extern struct {
    map_type: u32,
    key_size: u32,
    value_size: u32,
    max_entries: u32,
};
```

**Meaning:** Use an ABI-oriented, predictable memory layout for the struct.

Useful when another system will interpret the bytes directly.

For `MapDef`:

```text
offset 0  → map_type      4 bytes
offset 4  → key_size      4 bytes
offset 8  → value_size    4 bytes
offset 12 → max_entries   4 bytes
total     → 16 bytes
```

Think:

> `extern struct` = "the exact binary layout matters."

---

## 2. `export`

```zig
export var stacks: MapDef = ...;
```

**Meaning:** Make the declaration externally visible in the generated object.

In Nusku, this helps the compiled ELF object contain named symbols such as:

```text
stacks
counts
target_pid
```

Think:

> `export` = "make this declaration visible outside this Zig code."

---

## 3. `linksection`

```zig
export var stacks: MapDef linksection(".maps") = .{ ... };
```

**Meaning:** Put the variable into a specific ELF section.

Here:

```text
.maps
├── stacks
├── counts
└── target_pid
```

Think:

> `linksection(".maps")` = "put this object in the `.maps` section."

This is important because Nusku's loader later reads that section.

---

## 4. `@sizeOf`

```zig
@sizeOf(StackKey)
```

**Meaning:** Get the size of a type in bytes.

Examples:

```zig
@sizeOf(u32)       // 4
@sizeOf(u64)       // 8
@sizeOf(StackKey)  // size of StackKey
```

It is evaluated at compile time.

Think:

> `@sizeOf(T)` = "How many bytes does type `T` occupy?"

---

# Pointers

## 5. Pointer: `*T`

```zig
var value: u32 = 42;
const ptr: *u32 = &value;
```

`ptr` contains the address of `value`.

```text
ptr
 │
 ▼
┌──────┐
│  42  │
└──────┘
```

Think:

> `*T` = pointer to a `T`.

---

## 6. `*const T`

```zig
const ptr: *const u32 = &value;
```

The pointer can read the value but cannot modify it through that pointer.

```zig
const x = ptr.*; // OK
ptr.* = 50;      // not allowed
```

Think:

> `*const T` = pointer to read-only `T`.

---

## 7. `anyopaque`

```zig
*const anyopaque
```

Means:

> "A pointer to some memory, but I don't know its concrete type."

Useful for generic APIs.

In Nusku, a map lookup can return generic memory because different maps contain different value types.

Example:

```text
target_pid → u32
counts     → u64
```

The generic pointer does not know which one it is until we interpret it.

---

# Type conversions

## 8. `@as`

```zig
const x: u32 = 10;
const y = @as(u64, x);
```

Explicitly gives a value a desired type.

```text
u32 10
  ↓
u64 10
```

Think:

> `@as(T, value)` = "use this value as type `T`."

Zig uses explicit conversions heavily because it is strict about types.

---

## 9. `@truncate`

```zig
const x: u64 = 0x1234567890ABCDEF;
const y: u32 = @truncate(x);
```

Keeps only the bits that fit into the destination type.

```text
u64
12345678 90ABCDEF
         ↓
u32
90ABCDEF
```

Think:

> `@truncate` = "keep the lower bits that fit."

Nusku example:

```zig
const tgid: u32 =
    @truncate(get_current_pid_tgid() >> 32);
```

The helper returns `u64`; after shifting, the TGID is in the lower 32 bits, so `@truncate` extracts it as `u32`.

---

# Pointer conversions

## 10. `@ptrCast`

```zig
const other: *u8 = @ptrCast(ptr);
```

Changes the **pointer type**, not the memory.

The pointers still refer to the same address.

```text
same memory
    ▲
    │
 ┌──┴──┐
*u32  *u8
```

Think:

> `@ptrCast` = "view this address as another pointer type."

---

## 11. `@alignCast`

```zig
@alignCast(ptr)
```

Tells Zig that the pointer has the alignment required by the destination type.

It does not move memory or allocate anything.

Think:

> `@alignCast` = "this pointer is properly aligned."

Common combination:

```zig
@ptrCast(@alignCast(p))
```

Read inside-out:

```text
p
 ↓
@alignCast
 ↓
@ptrCast
 ↓
pointer with desired type
```

---

## 12. `@ptrFromInt`

```zig
@ptrFromInt(1)
```

Converts an integer into a pointer value.

Normally this is unusual and dangerous for normal application code.

In Nusku's BPF code it is intentional because BPF helpers are identified by numbers:

```text
helper #1  → map lookup
helper #2  → map update
helper #14 → current PID/TGID
helper #27 → stack ID
```

So:

```zig
@ptrFromInt(1)
```

is being used to represent BPF helper #1 as a function pointer.

Think:

> `@ptrFromInt(n)` = "make a pointer from this integer address/value."

---

# Functions and ABI

## 13. Function pointer

A type such as:

```zig
*const fn (u32) u64
```

means:

> Pointer to a function that accepts `u32` and returns `u64`.

Conceptually:

```text
function address
      │
      ▼
fn(u32) → u64
```

---

## 14. `callconv(.c)`

```zig
fn (u32) callconv(.c) u64
```

Specifies the calling convention.

The C calling convention defines how arguments, return values, registers, etc. are passed between caller and function.

Think:

> `callconv(.c)` = "use the C ABI calling convention."

This is useful when matching an external ABI, such as BPF helper signatures.

---

# Optional values

## 15. Optional: `?T`

```zig
const result: ?u32 = something();
```

Means:

```text
result can be:
    u32
    or null
```

Example:

```zig
const value: ?u32 = null;
```

---

## 16. `orelse`

```zig
const value = maybe_value orelse 100;
```

If `maybe_value` contains a value, use it.

If it is `null`, use `100`.

Nusku:

```zig
const target_ptr =
    map_lookup_elem(&target_pid, &zero) orelse return 0;
```

Meaning:

```text
map lookup
    │
    ├── found → continue
    │
    └── not found → return 0
```

Think:

> `orelse` = "if null, use this fallback."

---

# Dereferencing

## 17. `.*`

```zig
const value = ptr.*;
```

Dereferences the pointer and reads the value at that address.

```text
ptr
 │
 ▼
┌──────┐
│  42  │
└──────┘
  ↑
 ptr.*
```

Think:

> `ptr.*` = "read the value pointed to by `ptr`."

---

## 18. `&`

```zig
const ptr = &value;
```

Gets the address of a value.

Think:

> `&value` = "give me a pointer to `value`."

---

# Initialization

## 19. `undefined`

```zig
var key: StackKey = undefined;
```

Allocates the variable without initializing its contents.

It is valid when you are going to initialize every field before reading it.

Example:

```zig
var key: StackKey = undefined;

key.tgid = tgid;
key.user_stack_id = user_stack_id;
key.kernel_stack_id = kernel_stack_id;
```

Important:

> `undefined` does NOT mean `null` and does NOT mean zero.

It means the initial contents are unspecified.

---

# Concurrency

## 20. `@atomicRmw`

```zig
@atomicRmw(u64, count, .Add, 1, .monotonic);
```

Performs an atomic read-modify-write operation.

Here it means:

```text
type       → u64
target     → count
operation  → Add
value      → 1
ordering   → monotonic
```

Conceptually:

```text
count = 10
atomic +1
count = 11
```

Why atomic?

Multiple CPUs can execute BPF programs at the same time.

Without an atomic increment:

```text
CPU 1 reads 10
CPU 2 reads 10
CPU 1 writes 11
CPU 2 writes 11
```

One sample would be lost.

With atomic increment:

```text
CPU 1 → +1
CPU 2 → +1

10 → 11 → 12
```

Think:

> `@atomicRmw` = "change shared memory safely as one atomic operation."

---

# Quick mental table

| Zig feature | Mental model |
|---|---|
| `extern struct` | Exact ABI/binary layout matters |
| `export` | Make declaration externally visible |
| `linksection` | Put object in a named ELF section |
| `@sizeOf` | Get type size in bytes |
| `*T` | Pointer to `T` |
| `*const T` | Read-only pointer to `T` |
| `anyopaque` | Pointer to unknown-type memory |
| `@as` | Explicitly use/convert to a type |
| `@truncate` | Keep lower bits that fit |
| `@ptrCast` | Change pointer type |
| `@alignCast` | Assert required pointer alignment |
| `@ptrFromInt` | Create pointer from integer |
| `callconv(.c)` | Use C ABI calling convention |
| `?T` | `T` or `null` |
| `orelse` | Fallback when optional is `null` |
| `.*` | Dereference pointer |
| `&x` | Get address of `x` |
| `undefined` | Uninitialized value |
| `@atomicRmw` | Atomic read-modify-write |

---

# Nusku-specific mental model

The advanced syntax in the BPF file mostly exists because we're doing three unusual things:

```text
                Zig
                 │
       ┌─────────┼─────────┐
       ▼         ▼         ▼
   Binary       ELF       BPF ABI
   layout     sections    helpers
       │         │         │
       └─────────┼─────────┘
                 ▼
             Linux kernel
```

So when you see something like:

```zig
export var counts: MapDef linksection(".maps")
```

don't think:

> "Why is this Zig code so weird?"

Think:

> "We're deliberately constructing an ELF object that another program, our loader, will inspect."

That is the key to understanding the rest of Nusku.
