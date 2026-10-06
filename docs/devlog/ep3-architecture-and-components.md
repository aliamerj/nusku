---
date: 2026-10-06
---
# Components & Architecture
After we complete the infrastructure code that we need in this project, we can move one to next task which is the architecture of this project,
i was thinking how we should do this project , and reading different projects architectures, i have good idea of what we going to do.

Nusku splits into a few pieces (components), each with one job, built and released one at a time.

## Components
### 1. **Agent**: 

The part that actually watches. It sits next to a process, collects data through eBPF (CPU time, blocked time, allocations, syscalls, lock contention, and more), and streams what it sees out to anyone asking for it. 

It can run purely locally with nothing to configure, or remotely over a secured connection. 

This is the piece being built first, since nothing else matters if the data it collects isn't accurate.

### 2. **Controller**:
The part you actually talk to. 

It knows what's running and worth watching, lets you pick a target, starts and stops profiling sessions, and routes the live data wherever it needs to go, whether that's straight to your *terminal* or into *storage* for later. 

Local profiling can work through the Agent directly; 

the Controller becomes necessary once there's more than one thing to watch, or watching needs to happen from somewhere else.

##### this what i have right now , i will update this later when i complete these components 

the important part is each component should stand on its own and gets finished before the next one starts 


For watching one process locally, you can talk to the Agent directly, nothing else needs to be running. 

The Controller becomes the thing in the middle once there's more than one target to manage, or watching needs to happen from somewhere other than the machine the Agent is running on.


Data moves over a streaming connection built to carry a continuous, live flow rather than one-off requests, since the whole point is watching something happen as it happens, not polling for an answer. Locally, nothing needs configuring. 

Remotely, the connection is secured, so watching a server from somewhere else doesn't mean opening it up.

..to be connection :)
