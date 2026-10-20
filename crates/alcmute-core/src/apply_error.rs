use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyErrorKind {
    Rejected,
    RecoveryFailed,
}

#[derive(Debug)]
pub struct ApplyError {
    pub kind: ApplyErrorKind,
    pub message: String,
}

impl From<String> for ApplyError {
    fn from(message: String) -> Self {
        Self {
            kind: ApplyErrorKind::Rejected,
            message,
        }
    }
}

impl ApplyError {
    pub(crate) fn recovery(message: String) -> Self {
        Self {
            kind: ApplyErrorKind::RecoveryFailed,
            message,
        }
    }
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if self.kind == ApplyErrorKind::RecoveryFailed {
            write!(
                f,
                "\n完全には復旧できませんでした。設定を再保存するか、AlcMuteを再起動してください。"
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ApplyError {}
