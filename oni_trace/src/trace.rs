struct xEvent {
    /// The name of the event.
    name: String,
    /// The event categories.
    cat: String,
    /// The event type
    ph: String,
    /// The tracing clock timestamp of the event.
    ts: u64,
    /// The Thread clock timestamp of the event.
    tts: Option<u64>,

    /// The process ID.
    pid: u32,

    args: Vec<()>,

    cname: Option<String>,
}

enum Kind {
    DurationBegin, // B
    DurationEnd, // E

    Complete, // X
    //Instant, // i, I (deprecated)
    Counter, // C
    AsyncNestableStart, // b
    AsyncNestableInstant, // n
    AsyncNestableEnd, // e
    /* Async deprecated
        S - start, T - step into, p - step past, F - end
    */

    FlowStart, // s
    FlowStep, // t
    FlowEnd, // f

    // Sample, // P (deprecated)
    ObjectCreated, // N
    ObjectSnapshot, // O
    ObjectDestroyed, // D

    Metadata, // M
    MemoryDumpGlobal, // V
    MemoryDumpProcess, // v

    Mark, // R
    ClockSync, // c

    Context, // (,)
}

struct ID {
    pid: usize,
    tid: usize,
}

enum DurationEvent {
    begin: bool,
    id: ID,
    timestamp: u64,
    name: Option<String>,
    categories: Vec<String>,
}
