use std::{borrow::Cow, fmt::Display, str::FromStr};

use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
    Decode, Encode, Sqlite, Type,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct AdId(uuid::Uuid);

impl AdId {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl AsRef<uuid::Uuid> for AdId {
    fn as_ref(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl From<uuid::Uuid> for AdId {
    fn from(id: uuid::Uuid) -> Self {
        Self(id)
    }
}

impl FromStr for AdId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = uuid::Uuid::from_str(s).map_err(Self::Err::from)?;
        Ok(Self(id))
    }
}

impl Display for AdId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

impl<'r> Encode<'r, Sqlite> for AdId {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'r>>) -> sqlx::encode::IsNull {
        args.push(SqliteArgumentValue::Text(Cow::Owned(self.0.to_string())));

        IsNull::No
    }
}

impl<'r> Decode<'r, Sqlite> for AdId {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <&str as Decode<Sqlite>>::decode(value)?;
        Self::from_str(s).map_err(Into::into)
    }
}

impl Type<Sqlite> for AdId {
    fn type_info() -> SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}
