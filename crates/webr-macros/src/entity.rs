use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, Type};

/// Parsed entity metadata from #[entity(table = "...")] and struct fields.
struct EntityInfo {
    table: String,
    struct_name: syn::Ident,
    primary_key: syn::Ident,
    primary_key_type: Type,
    /// (field_name, column_name) pairs — excluding primary key
    columns: Vec<(syn::Ident, String)>,
    /// All columns including primary key
    all_columns: Vec<(syn::Ident, String)>,
}

pub fn expand_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_struct: ItemStruct =
        syn::parse2(item).expect("#[entity] can only be applied to a struct");

    let table = parse_table_attr(attr);
    let info = parse_entity(&item_struct, table);

    // Strip #[primary_key] attributes from fields so rustc doesn't see them
    if let syn::Fields::Named(ref mut named) = item_struct.fields {
        for field in named.named.iter_mut() {
            field.attrs.retain(|a| !a.path().is_ident("primary_key"));
        }
    }

    let iden_enum = generate_iden_enum(&info);
    let crud_impl = generate_crud_methods(&info);

    quote! {
        #item_struct
        #iden_enum
        #crud_impl
    }
}

/// Parse `table = "..."` from the attribute.
fn parse_table_attr(attr: TokenStream) -> String {
    let attr_str = attr.to_string();
    for part in attr_str.split(',') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("table") {
            let val = val.trim().strip_prefix('=').unwrap_or("").trim();
            return val.trim_matches('"').to_string();
        }
    }
    panic!("#[entity] requires table attribute, e.g. #[entity(table = \"users\")]")
}

/// Extract entity metadata from the struct definition.
fn parse_entity(item_struct: &ItemStruct, table: String) -> EntityInfo {
    let struct_name = item_struct.ident.clone();
    let fields = match &item_struct.fields {
        Fields::Named(named) => &named.named,
        _ => panic!("#[entity] requires named fields"),
    };

    let mut primary_key: Option<syn::Ident> = None;
    let mut primary_key_type: Option<Type> = None;
    let mut columns = Vec::new();
    let mut all_columns = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap().clone();
        let col_name = field_name.to_string();
        all_columns.push((field_name.clone(), col_name.clone()));

        let is_pk = field.attrs.iter().any(|a| a.path().is_ident("primary_key"));
        if is_pk {
            primary_key = Some(field_name.clone());
            primary_key_type = Some(field.ty.clone());
        } else {
            columns.push((field_name, col_name));
        }
    }

    let primary_key = primary_key.expect("#[entity] struct must have one #[primary_key] field");
    let primary_key_type = primary_key_type.unwrap();

    EntityInfo {
        table,
        struct_name,
        primary_key,
        primary_key_type,
        columns,
        all_columns,
    }
}

/// Generate the sea-query Iden enum.
fn generate_iden_enum(info: &EntityInfo) -> TokenStream {
    let enum_name = syn::Ident::new(
        &format!("{}Iden", info.struct_name),
        info.struct_name.span(),
    );

    let field_variants: Vec<_> = info
        .all_columns
        .iter()
        .map(|(name, _)| {
            let variant_name = to_pascal_case(&name.to_string());
            syn::Ident::new(&variant_name, name.span())
        })
        .collect();

    let table_str = &info.table;

    quote! {
        #[derive(Copy, Clone, Debug, sea_query::Iden)]
        pub enum #enum_name {
            #[iden = #table_str]
            Table,
            #(#field_variants,)*
        }
    }
}

/// Generate CRUD methods on the entity struct.
fn generate_crud_methods(info: &EntityInfo) -> TokenStream {
    let struct_name = &info.struct_name;
    let table = &info.table;
    let pk = &info.primary_key;
    let pk_type = &info.primary_key_type;
    let pk_col = pk.to_string();

    let all_col_names: Vec<&str> = info.all_columns.iter().map(|(_, c)| c.as_str()).collect();
    let select_cols = all_col_names.join(", ");
    let non_pk_col_names: Vec<&str> = info.columns.iter().map(|(_, c)| c.as_str()).collect();
    let non_pk_count = non_pk_col_names.len();
    let insert_cols = non_pk_col_names.join(", ");
    let non_pk_field_names: Vec<&syn::Ident> = info.columns.iter().map(|(f, _)| f).collect();

    quote! {
        #[allow(unreachable_patterns, unreachable_code, unexpected_cfgs)]
        impl #struct_name {
            /// Find entity by primary key.
            pub async fn find_by_id(
                pool: &webr::db::DbPool,
                id: &#pk_type,
            ) -> Result<Option<Self>, webr::db::DbError> {
                let sql = format!(
                    "SELECT {} FROM {} WHERE {} = {}",
                    #select_cols, #table, #pk_col,
                    pool.placeholder(1),
                );
                webr::tracing::debug!(target: "webr::sql", "==> {} | params: [{}]", sql, id);
                if let Some(__t) = webr::db::try_get_txn() {
                    __t.fetch_optional(&sql, |q| q.bind(id)).await
                } else {
                    pool.fetch_optional(&sql, |q| q.bind(id)).await
                }
            }

            /// Fetch all entities.
            pub async fn find_all(
                pool: &webr::db::DbPool,
            ) -> Result<Vec<Self>, webr::db::DbError> {
                let sql = format!("SELECT {} FROM {}", #select_cols, #table);
                webr::tracing::debug!(target: "webr::sql", "==> {}", sql);
                if let Some(__t) = webr::db::try_get_txn() {
                    __t.fetch_all(&sql, |q| q).await
                } else {
                    pool.fetch_all(&sql, |q| q).await
                }
            }

            /// Insert entity and return the created record.
            pub async fn save(
                &self,
                pool: &webr::db::DbPool,
            ) -> Result<Self, webr::db::DbError> {
                let mut placeholders = Vec::new();
                for i in 1..=#non_pk_count {
                    placeholders.push(pool.placeholder(i));
                }
                let insert_sql = format!(
                    "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
                    #table, #insert_cols,
                    placeholders.join(", "),
                    #select_cols,
                );
                let fetch_sql = format!(
                    "SELECT {} FROM {} WHERE {} = {}",
                    #select_cols, #table, #pk_col,
                    pool.placeholder(1),
                );
                webr::tracing::debug!(target: "webr::sql", "==> {} (insert)", insert_sql);
                if let Some(__t) = webr::db::try_get_txn() {
                    __t.insert_fetch(
                        &insert_sql, &fetch_sql, #pk_col,
                        |q| q #( .bind(&self.#non_pk_field_names) )*,
                    ).await
                } else {
                    pool.insert_fetch(
                        &insert_sql, &fetch_sql, #pk_col,
                        |q| q #( .bind(&self.#non_pk_field_names) )*,
                    ).await
                }
            }

            /// Update entity by primary key. Returns true if a row was updated.
            pub async fn update(
                &self,
                pool: &webr::db::DbPool,
            ) -> Result<bool, webr::db::DbError> {
                let non_pk_cols: &[&str] = &[#(#non_pk_col_names),*];
                let mut set_parts = Vec::with_capacity(non_pk_cols.len());
                for (i, col) in non_pk_cols.iter().enumerate() {
                    set_parts.push(format!("{col} = {}", pool.placeholder(i + 1)));
                }
                let pk_ph = pool.placeholder(non_pk_cols.len() + 1);
                let sql = format!(
                    "UPDATE {} SET {} WHERE {} = {pk_ph}",
                    #table, set_parts.join(", "), #pk_col,
                );
                webr::tracing::debug!(target: "webr::sql", "==> {}", sql);
                let rows: u64 = if let Some(__t) = webr::db::try_get_txn() {
                    __t.execute(&sql, |q| q
                        #( .bind(&self.#non_pk_field_names) )*
                        .bind(&self.#pk)
                    ).await?
                } else {
                    pool.execute(&sql, |q| q
                        #( .bind(&self.#non_pk_field_names) )*
                        .bind(&self.#pk)
                    ).await?
                };
                Ok(rows > 0)
            }

            /// Delete entity by primary key. Returns true if a row was deleted.
            pub async fn delete(
                &self,
                pool: &webr::db::DbPool,
            ) -> Result<bool, webr::db::DbError> {
                let sql = format!(
                    "DELETE FROM {} WHERE {} = {}",
                    #table, #pk_col,
                    pool.placeholder(1),
                );
                webr::tracing::debug!(target: "webr::sql", "==> {} | params: [{}]", sql, &self.#pk);
                let rows: u64 = if let Some(__t) = webr::db::try_get_txn() {
                    __t.execute(&sql, |q| q.bind(&self.#pk)).await?
                } else {
                    pool.execute(&sql, |q| q.bind(&self.#pk)).await?
                };
                Ok(rows > 0)
            }

            /// Count all entities.
            pub async fn count(
                pool: &webr::db::DbPool,
            ) -> Result<i64, webr::db::DbError> {
                let sql = format!("SELECT COUNT(*) FROM {}", #table);
                webr::tracing::debug!(target: "webr::sql", "==> {}", sql);
                if let Some(__t) = webr::db::try_get_txn() {
                    __t.fetch_scalar::<i64>(&sql, |q| q).await
                } else {
                    pool.fetch_scalar::<i64>(&sql, |q| q).await
                }
            }

            /// Find entities with pagination.
            pub async fn find_page(
                pool: &webr::db::DbPool,
                pager: webr::db::Pagination,
            ) -> Result<webr::db::Page<Self>, webr::db::DbError> {
                let __offset = pager.offset();
                let __limit = pager.limit();
                let count_sql = format!("SELECT COUNT(*) FROM {}", #table);
                let data_sql = format!(
                    "SELECT {} FROM {} LIMIT {} OFFSET {}",
                    #select_cols, #table, __limit, __offset,
                );
                webr::tracing::debug!(target: "webr::sql", "==> [count] {}", count_sql);
                webr::tracing::debug!(target: "webr::sql", "==> [data]  {}", data_sql);
                if let Some(__t) = webr::db::try_get_txn() {
                    let __total = __t.fetch_scalar::<i64>(&count_sql, |q| q).await?;
                    let __items = __t.fetch_all::<Self>(&data_sql, |q| q).await?;
                    if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
                        webr::tracing::debug!(target: "webr::sql", "<== total={}, items={}", __total, __items.len());
                    }
                    Ok(webr::db::Page::new(__items, __total, pager.page, pager.page_size))
                } else {
                    let __total = pool.fetch_scalar::<i64>(&count_sql, |q| q).await?;
                    let __items = pool.fetch_all::<Self>(&data_sql, |q| q).await?;
                    if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
                        webr::tracing::debug!(target: "webr::sql", "<== total={}, items={}", __total, __items.len());
                    }
                    Ok(webr::db::Page::new(__items, __total, pager.page, pager.page_size))
                }
            }
        }
    }
}

/// Convert snake_case to PascalCase for enum variants.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}
