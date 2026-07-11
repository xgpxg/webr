//! Marker traits that abstract over per-database `FromRow` / `Decode + Type` bounds.
//!
//! `#[cfg]` cannot be placed on individual trait bounds, so we use a `macro_rules!`
//! helper to generate one trait definition + blanket impl per active feature combination.

/// Internal macro: emit a trait definition + blanket impl.
///
/// ```ignore
/// __define!(TraitName, #[cfg(...)], { bound1 + bound2 });
/// ```
macro_rules! __define {
    ($name:ident, #[$cfg:meta], { $($bounds:tt)+ }) => {
        #[$cfg]
        pub trait $name: Send + Unpin + 'static + $($bounds)+ {}

        #[$cfg]
        #[automatically_derived]
        impl<T> $name for T where T: Send + Unpin + 'static + $($bounds)+ {}
    };
}

// ─── Row ───────────────────────────────────────────────────────────
// Bound per DB: `for<'r> FromRow<'r, XxxRow>`

macro_rules! __row {
    (@pg_ms_sq) => {
        __define!(Row, #[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))],
            { for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
            + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> });
    };
    (@pg_ms) => {
        __define!(Row, #[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))],
            { for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> });
    };
    (@pg_sq) => {
        __define!(Row, #[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))],
            { for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
            + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> });
    };
    (@ms_sq) => {
        __define!(Row, #[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))],
            { for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>
            + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> });
    };
    (@pg) => {
        __define!(Row, #[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))],
            { for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> });
    };
    (@ms) => {
        __define!(Row, #[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))],
            { for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> });
    };
    (@sq) => {
        __define!(Row, #[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))],
            { for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> });
    };
}

__row! { @pg_ms_sq }
__row! { @pg_ms }
__row! { @pg_sq }
__row! { @ms_sq }
__row! { @pg }
__row! { @ms }
__row! { @sq }

// ─── Scalar ────────────────────────────────────────────────────────
// Bound per DB: `for<'r> Decode<'r, XxxDB> + Type<XxxDB>`

macro_rules! __scalar {
    (@pg_ms_sq) => {
        __define!(Scalar, #[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))],
            { for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
            + for<'r> sqlx::Decode<'r, sqlx::MySql>    + sqlx::Type<sqlx::MySql>
            + for<'r> sqlx::Decode<'r, sqlx::Sqlite>   + sqlx::Type<sqlx::Sqlite> });
    };
    (@pg_ms) => {
        __define!(Scalar, #[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))],
            { for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
            + for<'r> sqlx::Decode<'r, sqlx::MySql>    + sqlx::Type<sqlx::MySql> });
    };
    (@pg_sq) => {
        __define!(Scalar, #[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))],
            { for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>
            + for<'r> sqlx::Decode<'r, sqlx::Sqlite>   + sqlx::Type<sqlx::Sqlite> });
    };
    (@ms_sq) => {
        __define!(Scalar, #[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))],
            { for<'r> sqlx::Decode<'r, sqlx::MySql>  + sqlx::Type<sqlx::MySql>
            + for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> });
    };
    (@pg) => {
        __define!(Scalar, #[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))],
            { for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> });
    };
    (@ms) => {
        __define!(Scalar, #[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))],
            { for<'r> sqlx::Decode<'r, sqlx::MySql> + sqlx::Type<sqlx::MySql> });
    };
    (@sq) => {
        __define!(Scalar, #[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))],
            { for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> });
    };
}

__scalar! { @pg_ms_sq }
__scalar! { @pg_ms }
__scalar! { @pg_sq }
__scalar! { @ms_sq }
__scalar! { @pg }
__scalar! { @ms }
__scalar! { @sq }
