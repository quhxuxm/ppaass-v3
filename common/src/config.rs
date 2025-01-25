pub trait ServerConfig {
    fn worker_thread_number(&self) -> usize;
    fn server_port(&self) -> u16;

    fn ip_v6(&self) -> bool;
}
