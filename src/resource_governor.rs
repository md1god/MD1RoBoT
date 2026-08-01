pub struct ResourceGovernor {
    max_retries: u32,
}

impl ResourceGovernor {
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }

    pub fn check_retry_limit(&self, retry_count: u32) -> bool {
        retry_count <= self.max_retries
    }

    pub fn compute_error_hash(&self, error_output: &str) -> String {
        blake3::hash(error_output.as_bytes()).to_hex().to_string()
    }
}
