//! Offline, bounded ports of the pinned Diff Checker algorithms and session UI.
pub mod diff;
pub mod json;
pub mod session;

pub const INPUT_LIMIT: usize = 1_048_576;
pub const OUTPUT_LIMIT: usize = 8_388_608;
pub const LINE_LIMIT: usize = 20_000;
pub const WORK_LIMIT: usize = 4_000_000;
pub const LIMIT_ERROR: &str =
    "Comparison limit exceeded (1 MiB per input, 20,000 lines, 4,000,000 work units, 8 MiB output)";

pub(crate) fn input(s: &str) -> Result<(), String> {
    if s.len() > INPUT_LIMIT {
        Err(LIMIT_ERROR.into())
    } else {
        Ok(())
    }
}
pub(crate) fn output(s: String) -> Result<String, String> {
    if s.len() > OUTPUT_LIMIT {
        Err(LIMIT_ERROR.into())
    } else {
        Ok(s)
    }
}
#[derive(Default)]
pub(crate) struct Budget(usize);
impl Budget {
    pub fn spend(&mut self, n: usize) -> Result<(), String> {
        self.0 = self.0.checked_add(n).ok_or(LIMIT_ERROR)?;
        if self.0 > WORK_LIMIT {
            Err(LIMIT_ERROR.into())
        } else {
            Ok(())
        }
    }
}
