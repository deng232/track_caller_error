use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct NetError;

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("network unavailable")
    }
}

impl std::error::Error for NetError {}

pub fn always_fails() -> std::result::Result<(), crate::AppError> {
    Err(NetError)?;
    Ok(())
}

#[macro_export]
macro_rules! net_error_variants {
    () => {
        Net(crate::net::NetError),
    };
}
