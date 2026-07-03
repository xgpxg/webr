/// Marker trait for types deserializable from database rows.
///
/// Automatically satisfied by any type that derives `sqlx::FromRow`,
/// since `#[derive(sqlx::FromRow)]` generates impls for all enabled databases.
///
/// This trait encapsulates per-database `FromRow` bounds so that
/// proc-macro-generated code and executor methods only need `R: Row`.

// ── All three databases ─────────────────────────────────────────────
#[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub trait Row:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
    + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
    + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>
{
}
#[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
impl<T> Row for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
        + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
        + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
{
}

// ── postgres + mysql ────────────────────────────────────────────────
#[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))]
pub trait Row:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
    + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
{
}
#[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))]
impl<T> Row for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
        + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>,
{
}

// ── postgres + sqlite ───────────────────────────────────────────────
#[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))]
pub trait Row:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
    + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>
{
}
#[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))]
impl<T> Row for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
        + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
{
}

// ── mysql + sqlite ──────────────────────────────────────────────────
#[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))]
pub trait Row:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
    + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>
{
}
#[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))]
impl<T> Row for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
        + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
{
}

// ── postgres only ────────────────────────────────────────────────
#[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))]
pub trait Row:
    Send + Unpin + 'static + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
{
}
#[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))]
impl<T> Row for T where
    T: Send + Unpin + 'static + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
{
}

// ── mysql only ─────────────────────────────────────────────────────
#[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))]
pub trait Row:
    Send + Unpin + 'static + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
{
}
#[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))]
impl<T> Row for T where
    T: Send + Unpin + 'static + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>,
{
}

// ── sqlite only ─────────────────────────────────────────────────────
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
pub trait Row:
    Send + Unpin + 'static + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>
{
}
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
impl<T> Row for T where
    T: Send + Unpin + 'static + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
{
}

// ─── Scalar marker trait ────────────────────────────────────────────
// Same pattern as `Row`, but for scalar query results (COUNT, SUM, etc.).
// `sqlx::query_scalar` requires `(T,): FromRow<'r, DB::Row>`, which
// reduces to `T: Decode + Type` for single-column tuples.

// ── All three databases ─────────────────────────────────────────────
#[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
pub trait Scalar:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::Decode<'r, sqlx::Postgres>
    + sqlx::Type<sqlx::Postgres>
    + for<'r> sqlx::Decode<'r, sqlx::MySql>
    + sqlx::Type<sqlx::MySql>
    + for<'r> sqlx::Decode<'r, sqlx::Sqlite>
    + sqlx::Type<sqlx::Sqlite>
{
}
#[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
impl<T> Scalar for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::Decode<'r, sqlx::Postgres>
        + sqlx::Type<sqlx::Postgres>
        + for<'r> sqlx::Decode<'r, sqlx::MySql>
        + sqlx::Type<sqlx::MySql>
        + for<'r> sqlx::Decode<'r, sqlx::Sqlite>
        + sqlx::Type<sqlx::Sqlite>,
{
}

// ── postgres + mysql ────────────────────────────────────────────────
#[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))]
pub trait Scalar:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::Decode<'r, sqlx::Postgres>
    + sqlx::Type<sqlx::Postgres>
    + for<'r> sqlx::Decode<'r, sqlx::MySql>
    + sqlx::Type<sqlx::MySql>
{
}
#[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))]
impl<T> Scalar for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::Decode<'r, sqlx::Postgres>
        + sqlx::Type<sqlx::Postgres>
        + for<'r> sqlx::Decode<'r, sqlx::MySql>
        + sqlx::Type<sqlx::MySql>,
{
}

// ── postgres + sqlite ───────────────────────────────────────────────
#[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))]
pub trait Scalar:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::Decode<'r, sqlx::Postgres>
    + sqlx::Type<sqlx::Postgres>
    + for<'r> sqlx::Decode<'r, sqlx::Sqlite>
    + sqlx::Type<sqlx::Sqlite>
{
}
#[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))]
impl<T> Scalar for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::Decode<'r, sqlx::Postgres>
        + sqlx::Type<sqlx::Postgres>
        + for<'r> sqlx::Decode<'r, sqlx::Sqlite>
        + sqlx::Type<sqlx::Sqlite>,
{
}

// ── mysql + sqlite ──────────────────────────────────────────────────
#[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))]
pub trait Scalar:
    Send
    + Unpin
    + 'static
    + for<'r> sqlx::Decode<'r, sqlx::MySql>
    + sqlx::Type<sqlx::MySql>
    + for<'r> sqlx::Decode<'r, sqlx::Sqlite>
    + sqlx::Type<sqlx::Sqlite>
{
}
#[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))]
impl<T> Scalar for T where
    T: Send
        + Unpin
        + 'static
        + for<'r> sqlx::Decode<'r, sqlx::MySql>
        + sqlx::Type<sqlx::MySql>
        + for<'r> sqlx::Decode<'r, sqlx::Sqlite>
        + sqlx::Type<sqlx::Sqlite>,
{
}

// ── postgres only ───────────────────────────────────────────────────
#[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))]
pub trait Scalar:
    Send + Unpin + 'static + for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
{
}
#[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))]
impl<T> Scalar for T where
    T: Send + Unpin + 'static + for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
}

// ── mysql only ──────────────────────────────────────────────────────
#[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))]
pub trait Scalar:
    Send + Unpin + 'static + for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>
{
}
#[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))]
impl<T> Scalar for T where
    T: Send + Unpin + 'static + for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql>,
{
}

// ── sqlite only ──────────────────────────────────────────────────────
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
pub trait Scalar:
    Send + Unpin + 'static + for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>
{
}
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
impl<T> Scalar for T where
    T: Send + Unpin + 'static + for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
}
