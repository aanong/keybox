use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("密码错误")]
    WrongPassword,
    #[error("需要密码才能解密此文件")]
    NeedPassword,
    #[error("条目不存在")]
    NotFound,
    #[error("{0}")]
    Invalid(String),
    #[error("加密错误")]
    Crypto,
    #[error("数据库错误：{0}")]
    Db(#[from] rusqlite::Error),
    #[error("文件读写错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 格式错误：{0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::WrongPassword => "WRONG_PASSWORD",
            AppError::NeedPassword => "NEED_PASSWORD",
            AppError::NotFound => "NOT_FOUND",
            AppError::Invalid(_) => "INVALID",
            AppError::Crypto => "CRYPTO",
            AppError::Db(_) => "DB",
            AppError::Io(_) => "IO",
            AppError::Json(_) => "JSON",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
