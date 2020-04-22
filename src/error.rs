use std::fmt::{Debug, Display, Formatter};
use std::result;

#[derive(Debug)]
pub struct Error(Box<ErrorKind>);

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Error(Box::new(kind))
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }

    pub fn into_kind(self) -> ErrorKind {
        *self.0
    }

    pub fn is_io_error(&self) -> bool {
        match *self.0 {
            ErrorKind::Io(_) => true,
            _ => false,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum ErrorKind {
    Io(std::io::Error),      // 文件系统IO错误
    IncorrectQsvVersion,     // 错误的QSV版本：本程序无法处理
    IncorrectQsvFormat,      // 错误的QSV格式：不符合预期格式/该文件不是QSV
    QsvTagsIsEmpty,          // 该QSV文件TAG块数量小于1个
    MediaDurationIsTooShort, // 媒体时长过短
}

impl ErrorKind {
    pub(crate) fn to_string(&self) -> String {
        match self {
            ErrorKind::Io(err) => err.to_string(),
            ErrorKind::IncorrectQsvVersion => String::from("incorrect qsv version"),
            ErrorKind::IncorrectQsvFormat => String::from("incorrect qsv format"),
            ErrorKind::QsvTagsIsEmpty => String::from("qsv has 0 tags"),
            ErrorKind::MediaDurationIsTooShort => String::from("media duration is too short"),
        }
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(err: std::io::Error) -> Error {
        Error::new(ErrorKind::Io(err))
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Error {
        Error::new(kind)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind() {
            ErrorKind::Io(err) => write!(f, "{}", err),
            _ => write!(f, "{}", &self.0.to_string()),
        }
    }
}
