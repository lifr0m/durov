mod placeholder;

use crate::Error;
use placeholder::Placeholder;
use std::str::FromStr;

impl Error {
    pub fn is(&self, status: i32, pattern: &str) -> bool {
        match self {
            Self::RpcError { code, message } => *code == status
                && message.starts_with(pattern),
            _ => false,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::RpcError { message, .. } => message,
            _ => panic!("error should be rpc error"),
        }
    }

    pub fn parse<T>(&self, pattern: &str, index: usize) -> Result<T, Error>
    where
        T: FromStr + Placeholder,
    {
        let index = pattern.split("_")
            .enumerate()
            .filter(|&(_, part)| part == T::PLACEHOLDER)
            .map(|(idx, _)| idx)
            .nth(index)
            .expect("pattern should contain placeholder");
        self.message()
            .split("_")
            .nth(index)
            .unwrap_or_else(|| panic!("invalid rpc error: {}", self.message()))
            .parse()
            .map_err(|_| panic!("invalid rpc error: {}", self.message()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        for (status, message, pattern, index, result) in [
            (303, "USER_MIGRATE_3", "USER_MIGRATE_%d", 0, 3),
            (400, "FILE_PART_42_MISSING", "FILE_PART_%d_MISSING", 0, 42),
            (666, "FOO_1_BAR_FEN_DAR_3_LAR", "FOO_%d_BAR_%s_DAR_%d_LAR", 1, 3),
        ] {
            let err = Error::RpcError {
                code: status,
                message: message.to_string(),
            };
            assert_eq!(err.parse::<i32>(pattern, index).unwrap(), result);
        }
    }
}
