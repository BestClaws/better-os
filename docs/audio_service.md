# Audio Service Architecture

The audio subsystem lives entirely behind the service defined in
[src/system/services/audio_srv.rs](../src/system/services/audio_srv.rs). This
file explains the flow end-to-end so someone with light concurrency background
can understand how the pieces fit together.

## Goals

- **Single owner of the hardware driver.** We never let applications touch the
  `AsyncAudioSink` directly; the service task owns it.
- **Minimal latency for playback requests.** The minute chime should trigger
  immediately without blocking the UI or timer tasks.
- **No surprises under concurrency.** Even if several tasks request the chime
  simultaneously, we generate the PCM data once and avoid races.

## High-Level Flow

```
Watch app / RTC ── play_minute_beep() ──▶ AUDIO_COMMANDS channel ──▶
                                           audio_service task ──▶ I2S driver
                                        ▲                         │
                                        └──── AUDIO_RESPONSES ◀───┘
```

1. **Caller API** – Applications ask `AudioService::play_minute_beep()` to start
   the chime and await a result.
2. **Command queue** – The request is enqueued on the `AUDIO_COMMANDS` channel.
3. **Service task** – `audio_service` (an Embassy task) owns the driver. It
   pulls the next command, locks the `AsyncAudioSink`, and kicks playback.
4. **Response queue** – Success or error is pushed back to
   `AUDIO_RESPONSES` so the caller resumes with the right `Result`.

If the hardware sink is absent, we spawn `audio_service_unavailable()` instead.
It still drains incoming commands but always responds with
`AudioError::Unavailable`, preventing deadlocks.

## Lazy Clip Generation

We ship with a single chime generated at runtime. The code avoids recomputing
it by caching the PCM buffer in a custom `AudioClip` wrapper: details in
[src/system/services/audio_srv.rs#L31-L111](../src/system/services/audio_srv.rs#L31-L111).

  The **first caller** that loads a null pointer allocates and fills the sample
  buffer, then publishes the raw pointer with
  [`compare_exchange`](../src/system/services/audio_srv.rs#L52-L58).
- Inside `AudioClip::ensure()` the **first caller** that loads a null pointer
  generates the samples and publishes them by calling
  [`self.ptr.compare_exchange(...)`](../src/system/services/audio_srv.rs#L52-L58)
  right there in `ensure`. Subsequent callers reuse the pointer the first task
  stored.
  observing the stored pointer will also observe the fully written sample data
  (no reordering).
- Competing constructors may allocate simultaneously. The losing threads are
  detected by compare_exchange returning `Err(existing)`; they safely turn their
  raw pointer back into a `Box` and let it drop.
- The final `&'static [i16]` comes from `&*ptr`. This is `unsafe`, but sound
  because the allocation is never freed after publication.

Why not a mutex? Because every chime request (even the fast path) would then
enter a critical section. On a cooperative async executor that can delay higher
priority tasks and risk I2S underruns. The atomic pattern keeps the steady-state
path wait-free. If the code ever needs simplification over performance, it could
switch to `once_cell::sync::OnceCell<&'static [i16]>`—it would remove the atomics
and `unsafe` at the cost of a small lock.

## Channel Design

Two bounded channels coordinate callers and the service task:

- `AUDIO_COMMANDS: Channel<..., AudioCommand, 4>` – queues playback requests.
  Capacity 4 is enough to absorb bursts without sacrificing memory.
- `AUDIO_RESPONSES: Channel<..., Result<(), AudioError>, 1>` – each request is
  synchronous, so a depth of 1 suffices (caller waits until its response arrives).

Channels wrap a `CriticalSectionRawMutex`, which disables interrupts briefly for
push/pop. That’s cheaper than allocating a heap buffer or running a full async
mutex per call.

## Service Task Lifecycle

`audio_service` is spawned from the kernel once the platform hands us an
`AsyncAudioSink`. Its loop is simple:

```rust
let request_rx = AUDIO_COMMANDS.receiver();
let response_tx = AUDIO_RESPONSES.sender();

loop {
    let cmd = request_rx.receive().await;
    let result = handle_command(driver, cmd).await;
    response_tx.send(result).await;
}
```

The driver is behind a `Mutex<CriticalSectionRawMutex, Box<dyn AsyncAudioSink>>`,
so `handle_command` grabs the lock only while invoking `play_once`. We log
failures with `defmt::warn!` and propagate the error to the caller.

The stub task (`audio_service_unavailable`) mirrors the same protocol so
callers never block indefinitely even when audio hardware is missing.

## Error Handling

`AudioResult` captures either success or an `AudioError` coming from the HAL.
Typical errors include:

- `AudioError::Unavailable` – driver missing (stub reports this).
- `AudioError::AlreadyRunning` – the hardware refuses to start another clip.
- `AudioError::I2s(_)` – low-level bus failure.

The UI can surface the message from the `Result` to inform the user.

## Summary

- The service owns hardware access and exposes a narrow async API.
- Requests and results are passed via bounded channels, keeping callers async.
- The chime PCM data is generated once in a lock-free manner to avoid latency.
- `unsafe` and explicit memory ordering are confined to the lazy clip cache and
  are justified by the invariants; they can be replaced with higher-level tools
  if determinism is less critical.

This layout scales if we add more audio commands: extend `AudioCommand`, add new
`AudioService` helpers, and handle the command in `handle_command`.
