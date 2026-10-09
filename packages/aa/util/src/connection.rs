#[derive(Debug)]
pub enum ConnectionResult<O> {
    Timeout,
    ConnectionReset,
    Result(anyhow::Result<O>),
}

impl<O> ConnectionResult<O> {
    pub fn into_response(self) -> anyhow::Result<O> {
        match self {
            ConnectionResult::Timeout => anyhow::bail!("Request timed out"),
            ConnectionResult::ConnectionReset => anyhow::bail!("Server reset the connection"),
            ConnectionResult::Result(r) => r,
        }
    }
}

impl<O> From<anyhow::Result<O>> for ConnectionResult<O> {
    fn from(result: anyhow::Result<O>) -> Self { ConnectionResult::Result(result) }
}
