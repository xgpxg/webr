use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Fields, ItemStruct, Type};

/// 实体元数据，从 `#[entity(table = "...")]` 和结构体字段解析而来。
struct EntityInfo {
    /// 数据库表名
    table: String,
    /// Rust 结构体名称
    struct_name: syn::Ident,
    /// 主键字段名
    primary_key: syn::Ident,
    /// 主键类型
    primary_key_type: Type,
    /// 主键映射后的列名（尊重 #[column(name = "...")]）
    pk_col: String,
    /// 非主键列：(字段名, 列名, 类型)
    columns: Vec<(syn::Ident, String, Type)>,
    /// 所有列（含主键）：(字段名, 列名, 类型)
    all_columns: Vec<(syn::Ident, String, Type)>,
}

/// 扩展 `#[entity]` 宏：生成 CRUD 方法、Iden 枚举和 FromRow 实现。
pub fn expand_entity(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut item_struct: ItemStruct =
        syn::parse2(item).expect("#[entity] can only be applied to a struct");

    let entity_attr: EntityAttr = syn::parse2(attr)
        .expect("#[entity] requires attributes, e.g. #[entity(table = \"users\")]");
    let info = parse_entity(&item_struct, entity_attr.table)?;

    // 剥离字段上的 #[column(...)] 属性，避免 rustc 报错
    if let syn::Fields::Named(ref mut named) = item_struct.fields {
        for field in named.named.iter_mut() {
            field.attrs.retain(|a| !a.path().is_ident("column"));
        }
    }

    // 手动生成 FromRow 实现，用户无需手动导入 sqlx
    let from_row_impl = generate_from_row_impl(&info);

    // 生成 sea-query Iden 枚举
    let iden_enum = generate_iden_enum(&info);
    // 生成 CRUD 方法
    let crud_impl = generate_crud_methods(&info);

    Ok(quote! {
        #item_struct
        #iden_enum
        #from_row_impl
        #crud_impl
    })
}

/// `#[entity(table = "...")]` 属性的结构化解析器。
///
/// 使用 `syn` 解析而非字符串操作，保证健壮性并易于扩展未来属性（如 `schema`）。
struct EntityAttr {
    table: String,
}

impl Parse for EntityAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut table: Option<String> = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            if key == "table" {
                let _eq: syn::Token![=] = input.parse()?;
                let value: syn::LitStr = input.parse()?;
                table = Some(value.value());
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    format!("unknown attribute `{key}`, expected `table`"),
                ));
            }

            // 允许多个属性之间的尾随逗号或分隔符
            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        let table =
            table.ok_or_else(|| input.error("missing required attribute: `table = \"...\"`"))?;
        Ok(EntityAttr { table })
    }
}

/// 从结构体定义中提取实体元数据。
fn parse_entity(item_struct: &ItemStruct, table: String) -> syn::Result<EntityInfo> {
    let struct_name = item_struct.ident.clone();
    let fields = match &item_struct.fields {
        Fields::Named(named) => &named.named,
        _ => panic!("#[entity] requires named fields"),
    };

    let mut primary_key: Option<syn::Ident> = None;
    let mut primary_key_type: Option<Type> = None;
    let mut pk_col: Option<String> = None;
    let mut columns = Vec::new();
    let mut all_columns = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap().clone();
        let col_attr = parse_column_attr(field)?;
        let col_name = col_attr.name.unwrap_or_else(|| field_name.to_string());
        let field_type = field.ty.clone();
        all_columns.push((field_name.clone(), col_name.clone(), field_type.clone()));

        if col_attr.is_pk {
            primary_key = Some(field_name.clone());
            primary_key_type = Some(field.ty.clone());
            pk_col = Some(col_name);
        } else {
            columns.push((field_name, col_name, field_type));
        }
    }

    let primary_key = primary_key.expect("#[entity] struct must have one field with #[column(pk)]");
    let primary_key_type = primary_key_type.unwrap();
    let pk_col = pk_col.unwrap();

    Ok(EntityInfo {
        table,
        struct_name,
        primary_key,
        primary_key_type,
        pk_col,
        columns,
        all_columns,
    })
}

/// 生成 sea-query Iden 枚举及其手动 trait 实现，
/// 用户无需将 `sea-query` 作为直接依赖。
fn generate_iden_enum(info: &EntityInfo) -> TokenStream {
    let enum_name = syn::Ident::new(
        &format!("{}Iden", info.struct_name),
        info.struct_name.span(),
    );

    let table_str = &info.table;

    // 构建 unquoted() 匹配分支：variant -> 列名字符串（尊重 #[column(name = "...")]）
    let mut field_variants = Vec::with_capacity(info.all_columns.len());
    let mut arm_variants = vec![quote! { Self::Table => write!(s, "{}", #table_str).unwrap() }];
    for (name, col_name, _) in &info.all_columns {
        let variant = syn::Ident::new(&to_pascal_case(&name.to_string()), name.span());
        arm_variants.push(quote! { Self::#variant => write!(s, "{}", #col_name).unwrap() });
        field_variants.push(variant);
    }

    quote! {
        #[derive(Copy, Clone, Debug)]
        pub enum #enum_name {
            Table,
            #(#field_variants,)*
        }

        impl webr::sea_query::Iden for #enum_name {
            fn prepare(&self, s: &mut dyn ::std::fmt::Write, q: webr::sea_query::Quote) {
                write!(s, "{}", q.left()).unwrap();
                self.unquoted(s);
                write!(s, "{}", q.right()).unwrap();
            }

            fn unquoted(&self, s: &mut dyn ::std::fmt::Write) {
                match self {
                    #(#arm_variants,)*
                }
            }
        }
    }
}

/// 手动生成 `sqlx::FromRow` 实现，用户无需手动 derive 且不需要 `sqlx` 作为直接依赖。
fn generate_from_row_impl(info: &EntityInfo) -> TokenStream {
    let struct_name = &info.struct_name;

    let field_names: Vec<_> = info.all_columns.iter().map(|(f, _, _)| f).collect();
    let col_strs: Vec<_> = info
        .all_columns
        .iter()
        .map(|(_, c, _)| c.as_str())
        .collect();
    let field_types: Vec<_> = info.all_columns.iter().map(|(_, _, t)| t).collect();

    quote! {
        #[automatically_derived]
        impl<'__r, __R: webr::sqlx::Row> webr::sqlx::FromRow<'__r, __R> for #struct_name
        where
            &'__r str: webr::sqlx::ColumnIndex<__R>,
            #( #field_types: webr::sqlx::Decode<'__r, __R::Database> + webr::sqlx::Type<__R::Database>, )*
        {
            fn from_row(__row: &'__r __R) -> ::std::result::Result<Self, webr::sqlx::Error> {
                Ok(Self {
                    #( #field_names: webr::sqlx::Row::try_get(__row, #col_strs)?, )*
                })
            }
        }
    }
}

/// 生成错误转换代码：将 DbError 映射为 webr::Error::Database
fn db_err_map() -> TokenStream {
    quote! { .map_err(|e| webr::Error::Database(Box::new(e)))? }
}

/// 在实体结构体上生成 CRUD 方法。
///
/// 方法内部使用 `webr::db::get_pool()` 获取连接池，调用者无需传递 pool 引用。
/// 通过 `webr::db::try_get_txn()` 保持事务感知能力。
/// 所有方法返回 `webr::Result<T>`，自动将 `DbError` 转换为 `webr::Error::Database`。
fn generate_crud_methods(info: &EntityInfo) -> TokenStream {
    let struct_name = &info.struct_name;
    let table = &info.table;
    let pk = &info.primary_key;
    let pk_type = &info.primary_key_type;
    let pk_col = info.pk_col.as_str();

    let all_col_names: Vec<&str> = info
        .all_columns
        .iter()
        .map(|(_, c, _)| c.as_str())
        .collect();
    let select_cols = all_col_names.join(", ");
    let non_pk_col_names: Vec<&str> = info.columns.iter().map(|(_, c, _)| c.as_str()).collect();
    let non_pk_count = non_pk_col_names.len();
    let insert_cols = non_pk_col_names.join(", ");
    let non_pk_field_names: Vec<&syn::Ident> = info.columns.iter().map(|(f, _, _)| f).collect();

    let err_map = db_err_map();

    quote! {
        #[allow(unreachable_patterns, unreachable_code, unexpected_cfgs)]
        impl #struct_name {
            /// 按主键查找实体。
            pub async fn find_by_id(
                id: &#pk_type,
            ) -> webr::Result<Option<Self>> {
                let __pool = webr::db::get_pool();
                let sql = format!(
                    "SELECT {} FROM {} WHERE {} = {}",
                    #select_cols, #table, #pk_col,
                    __pool.placeholder(1),
                );
                webr::tracing::debug!(target: "webr::sql", "==> {} | params: [{}]", sql, id);
                let result = if let Some(__t) = webr::db::try_get_txn() {
                    __t.fetch_optional(&sql, |q| q.bind(id)).await
                } else {
                    __pool.fetch_optional(&sql, |q| q.bind(id)).await
                };
                Ok(result #err_map)
            }

            /// 获取所有实体。
            pub async fn find_all() -> webr::Result<Vec<Self>> {
                let __pool = webr::db::get_pool();
                let sql = format!("SELECT {} FROM {}", #select_cols, #table);
                webr::tracing::debug!(target: "webr::sql", "==> {}", sql);
                let result = if let Some(__t) = webr::db::try_get_txn() {
                    __t.fetch_all(&sql, |q| q).await
                } else {
                    __pool.fetch_all(&sql, |q| q).await
                };
                Ok(result #err_map)
            }

            /// 插入实体并返回创建的记录。
            pub async fn save(
                &self,
            ) -> webr::Result<Self> {
                let __pool = webr::db::get_pool();
                let mut placeholders = Vec::new();
                for i in 1..=#non_pk_count {
                    placeholders.push(__pool.placeholder(i));
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
                    __pool.placeholder(1),
                );
                webr::tracing::debug!(target: "webr::sql", "==> {} (insert)", insert_sql);
                let result = if let Some(__t) = webr::db::try_get_txn() {
                    __t.insert_fetch(
                        &insert_sql, &fetch_sql, #pk_col,
                        |q| q #( .bind(&self.#non_pk_field_names) )*,
                    ).await
                } else {
                    __pool.insert_fetch(
                        &insert_sql, &fetch_sql, #pk_col,
                        |q| q #( .bind(&self.#non_pk_field_names) )*,
                    ).await
                };
                Ok(result #err_map)
            }

            /// 按主键更新实体。如果更新了行则返回 true。
            pub async fn update(
                &self,
            ) -> webr::Result<bool> {
                let __pool = webr::db::get_pool();
                let non_pk_cols: &[&str] = &[#(#non_pk_col_names),*];
                let mut set_parts = Vec::with_capacity(non_pk_cols.len());
                for (i, col) in non_pk_cols.iter().enumerate() {
                    set_parts.push(format!("{col} = {}", __pool.placeholder(i + 1)));
                }
                let pk_ph = __pool.placeholder(non_pk_cols.len() + 1);
                let sql = format!(
                    "UPDATE {} SET {} WHERE {} = {pk_ph}",
                    #table, set_parts.join(", "), #pk_col,
                );
                webr::tracing::debug!(target: "webr::sql", "==> {}", sql);
                let rows: u64 = if let Some(__t) = webr::db::try_get_txn() {
                    __t.execute(&sql, |q| q
                        #( .bind(&self.#non_pk_field_names) )*
                        .bind(&self.#pk)
                    ).await #err_map
                } else {
                    __pool.execute(&sql, |q| q
                        #( .bind(&self.#non_pk_field_names) )*
                        .bind(&self.#pk)
                    ).await #err_map
                };
                Ok(rows > 0)
            }

            /// 按主键删除实体。如果删除了行则返回 true。
            pub async fn delete(
                &self,
            ) -> webr::Result<bool> {
                let __pool = webr::db::get_pool();
                let sql = format!(
                    "DELETE FROM {} WHERE {} = {}",
                    #table, #pk_col,
                    __pool.placeholder(1),
                );
                webr::tracing::debug!(target: "webr::sql", "==> {} | params: [{}]", sql, &self.#pk);
                let rows: u64 = if let Some(__t) = webr::db::try_get_txn() {
                    __t.execute(&sql, |q| q.bind(&self.#pk)).await #err_map
                } else {
                    __pool.execute(&sql, |q| q.bind(&self.#pk)).await #err_map
                };
                Ok(rows > 0)
            }

            /// 统计所有实体数量。
            pub async fn count() -> webr::Result<i64> {
                let __pool = webr::db::get_pool();
                let sql = format!("SELECT COUNT(*) FROM {}", #table);
                webr::tracing::debug!(target: "webr::sql", "==> {}", sql);
                let result = if let Some(__t) = webr::db::try_get_txn() {
                    __t.fetch_scalar::<i64>(&sql, |q| q).await
                } else {
                    __pool.fetch_scalar::<i64>(&sql, |q| q).await
                };
                Ok(result #err_map)
            }

            /// 分页查询实体。
            pub async fn find_page(
                pager: webr::db::Pagination,
            ) -> webr::Result<webr::db::Page<Self>> {
                let __pool = webr::db::get_pool();
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
                    let __total = __t.fetch_scalar::<i64>(&count_sql, |q| q).await #err_map;
                    let __items = __t.fetch_all::<Self>(&data_sql, |q| q).await #err_map;
                    if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
                        webr::tracing::debug!(target: "webr::sql", "<== total={}, items={}", __total, __items.len());
                    }
                    Ok(webr::db::Page::new(__items, __total, pager.page, pager.page_size))
                } else {
                    let __total = __pool.fetch_scalar::<i64>(&count_sql, |q| q).await #err_map;
                    let __items = __pool.fetch_all::<Self>(&data_sql, |q| q).await #err_map;
                    if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
                        webr::tracing::debug!(target: "webr::sql", "<== total={}, items={}", __total, __items.len());
                    }
                    Ok(webr::db::Page::new(__items, __total, pager.page, pager.page_size))
                }
            }
        }
    }
}

/// `#[column(...)]` 属性解析结果。
struct ColumnAttr {
    /// 是否为主键
    is_pk: bool,
    /// 自定义列名
    name: Option<String>,
}

/// 解析字段的 `#[column(...)]` 属性。
///
/// 支持：
/// - `#[column(pk)]` — 标记主键
/// - `#[column(name = "col_name")]` — 自定义列名
/// - `#[column(pk, name = "col_name")]` — 两者组合
fn parse_column_attr(field: &syn::Field) -> syn::Result<ColumnAttr> {
    let mut attr = ColumnAttr {
        is_pk: false,
        name: None,
    };
    for a in &field.attrs {
        if a.path().is_ident("column") {
            a.parse_nested_meta(|meta| {
                if meta.path.is_ident("pk") {
                    attr.is_pk = true;
                    Ok(())
                } else if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    attr.name = Some(lit.value());
                    Ok(())
                } else {
                    Err(meta.error("expected `pk` or `name`"))
                }
            })?;
            break;
        }
    }
    Ok(attr)
}

/// 将 snake_case 转换为 PascalCase，用于枚举变体名。
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
