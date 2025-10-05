use crossbeam_channel as xchan;

pub struct LoopChans<'a, M> {
    pub rx: &'a xchan::Receiver<M>,
    pub interrupt_rx: &'a xchan::Receiver<()>,
    pub tick_rx: Option<&'a xchan::Receiver<std::time::Instant>>,
}

pub struct Env<'a, A, L, S> {
    pub args: &'a A,
    pub logger: &'a L,
    pub services: &'a S,
}

