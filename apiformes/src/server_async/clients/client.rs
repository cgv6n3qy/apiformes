#[derive(Clone)]
pub struct Client {
    pub session_expirary: u32,
    pub recv_max: u16,
    pub max_packet_size: u32,
    pub topic_alias_max: u16,
    pub response_info: bool,
    pub problem_info: bool,
    pub clientid: String,
}

impl Client {
    pub(super) fn new() -> Self {
        Client {
            session_expirary: 0,
            recv_max: u16::MAX,
            max_packet_size: u32::MAX,
            topic_alias_max: 0,
            response_info: false,
            problem_info: true,
            clientid: String::new(),
        }
    }
}
