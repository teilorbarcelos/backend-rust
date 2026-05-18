use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use chrono::{NaiveDate, TimeZone, Utc};
use std::collections::HashMap;

#[macro_export]
macro_rules! apply_common_filters {
    ($query:expr, $filters:expr, $active_col:expr, $created_col:expr, $updated_col:expr) => {{
        use sea_orm::{QueryFilter, ColumnTrait};
        let mut q = $query;
        if let Some(act) = $filters.active {
            q = q.filter($active_col.eq(act));
        }
        if let Some(start) = $filters.created_at_start {
            q = q.filter($created_col.gte(start));
        }
        if let Some(end) = $filters.created_at_end {
            q = q.filter($created_col.lte(end));
        }
        if let Some(start) = $filters.updated_at_start {
            q = q.filter($updated_col.gte(start));
        }
        if let Some(end) = $filters.updated_at_end {
            q = q.filter($updated_col.lte(end));
        }
        q
    }};
    ($query:expr, $filters:expr, $created_col:expr) => {{
        use sea_orm::{QueryFilter, ColumnTrait};
        let mut q = $query;
        if let Some(start) = $filters.created_at_start {
            q = q.filter($created_col.gte(start));
        }
        if let Some(end) = $filters.created_at_end {
            q = q.filter($created_col.lte(end));
        }
        q
    }};
}


#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct FilterParams {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub searchWord: Option<String>,
    pub searchFields: Option<String>,
    pub orderBy: Option<String>,
    pub orderDirection: Option<String>,
    pub ignoreDefaultFilters: Option<String>,
    pub active: Option<String>,
    pub createdAt_start: Option<String>,
    pub createdAt_end: Option<String>,
    pub updatedAt_start: Option<String>,
    pub updatedAt_end: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "Role.name")]
    pub role_name: Option<String>,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

pub struct QueryValidator;

impl QueryValidator {
    pub fn validate_and_parse(
        params: &HashMap<String, String>,
        allowed_search_fields: &[&str],
        allowed_filterable_fields: &[&str],
    ) -> Result<ParsedFilters, AppError> {
        let reserved_keys = ["page", "size", "searchWord", "searchFields", "orderBy", "orderDirection", "ignoreDefaultFilters"];

        // 1. If searchWord is provided, searchFields must be provided
        let search_word = params.get("searchWord").cloned().filter(|s| !s.is_empty());
        let search_fields_str = params.get("searchFields").cloned().filter(|s| !s.is_empty());
        
        if search_word.is_some() && search_fields_str.is_none() {
            return Err(AppError::BadRequest(
                "O parâmetro \"searchFields\" é obrigatório quando \"searchWord\" é fornecido.".to_string(),
            ));
        }

        let mut parsed_search_fields = Vec::new();

        // 2. Validate all provided search fields
        if let Some(ref fields_str) = search_fields_str {
            for field in fields_str.split(',') {
                let trimmed = field.trim();
                if !trimmed.is_empty() {
                    if !allowed_search_fields.contains(&trimmed) {
                        return Err(AppError::BadRequest(
                            format!("O campo '{}' não está disponível para pesquisa global.", trimmed),
                        ));
                    }
                    parsed_search_fields.push(trimmed.to_string());
                }
            }
        }

        // 3. Validate that all query keys passed are allowed filterable fields (or reserved keys)
        for (key, val) in params.iter() {
            if val.is_empty() {
                continue;
            }
            if reserved_keys.contains(&key.as_str()) {
                continue;
            }

            // Normalize: strip _start or _end suffix
            let mut field_key = key.as_str();
            if key.ends_with("_start") {
                field_key = &key[..key.len() - 6];
            } else if key.ends_with("_end") {
                field_key = &key[..key.len() - 4];
            }

            // Map frontend key names to resource filter keys
            let mapped_key = match field_key {
                "createdAt" => "createdAt",
                "updatedAt" => "updatedAt",
                "Role.name" => "Role.name",
                other => other,
            };

            if !allowed_filterable_fields.contains(&mapped_key) {
                return Err(AppError::BadRequest(
                    format!("O filtro '{}' não é permitido para este recurso.", field_key),
                ));
            }
        }

        // 4. Parse order validation
        let order_by = params.get("orderBy").cloned().filter(|s| !s.is_empty());
        if let Some(ref field) = order_by {
            let mapped_field = match field.as_str() {
                "createdAt" => "createdAt",
                "updatedAt" => "updatedAt",
                "Role.name" => "Role.name",
                other => other,
            };
            if !allowed_filterable_fields.contains(&mapped_field) && field != "created_at" && field != "updated_at" {
                return Err(AppError::BadRequest(
                    format!("A ordenação pelo campo '{}' não é permitida.", field),
                ));
            }
        }

        // 5. Parse pagination
        let page = params.get("page")
            .and_then(|p| p.parse::<u64>().ok())
            .unwrap_or(0);
        let size = params.get("size")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(25);

        if size > 100 {
            return Err(AppError::BadRequest(
                "O tamanho máximo da página é 100 itens.".to_string(),
            ));
        }

        let order_direction = params.get("orderDirection")
            .cloned()
            .unwrap_or_else(|| "asc".to_string());

        // 6. Parse ignoreDefaultFilters
        let ignore_default_filters = params.get("ignoreDefaultFilters")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        // 7. Parse active filter (defaults to true if ignore_default_filters is false and not explicitly passed)
        let active = if let Some(active_str) = params.get("active") {
            if active_str.is_empty() {
                None
            } else {
                Some(active_str == "true" || active_str == "1")
            }
        } else if !ignore_default_filters {
            Some(true) // Default to active users/products/roles!
        } else {
            None
        };

        // 8. Parse date filters
        let parse_date = |val: &str, name: &str, end_of_day: bool| -> Result<chrono::DateTime<Utc>, AppError> {
            let parsed = NaiveDate::parse_from_str(val, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest(format!("Formato de '{}' inválido. Use YYYY-MM-DD.", name)))?;
            let hms = if end_of_day { (23, 59, 59) } else { (0, 0, 0) };
            let dt = Utc.from_utc_datetime(&parsed.and_hms_opt(hms.0, hms.1, hms.2).unwrap());
            Ok(dt)
        };

        let created_at_start = params.get("createdAt_start")
            .map(|v| parse_date(v, "createdAt_start", false))
            .transpose()?;
        let created_at_end = params.get("createdAt_end")
            .map(|v| parse_date(v, "createdAt_end", true))
            .transpose()?;
        let updated_at_start = params.get("updatedAt_start")
            .map(|v| parse_date(v, "updatedAt_start", false))
            .transpose()?;
        let updated_at_end = params.get("updatedAt_end")
            .map(|v| parse_date(v, "updatedAt_end", true))
            .transpose()?;

        // 9. Parse other filters
        let name = params.get("name").cloned().filter(|s| !s.is_empty());
        let email = params.get("email").cloned().filter(|s| !s.is_empty());
        let role_name = params.get("Role.name").cloned().filter(|s| !s.is_empty());
        let sku = params.get("sku").cloned().filter(|s| !s.is_empty());
        let category = params.get("category").cloned().filter(|s| !s.is_empty());
        let description = params.get("description").cloned().filter(|s| !s.is_empty());

        Ok(ParsedFilters {
            page,
            size,
            search_word,
            search_fields: parsed_search_fields,
            order_by,
            order_direction,
            active,
            created_at_start,
            created_at_end,
            updated_at_start,
            updated_at_end,
            name,
            email,
            role_name,
            sku,
            category,
            description,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParsedFilters {
    pub page: u64,
    pub size: u64,
    pub search_word: Option<String>,
    pub search_fields: Vec<String>,
    pub order_by: Option<String>,
    pub order_direction: String,
    pub active: Option<bool>,
    pub created_at_start: Option<chrono::DateTime<Utc>>,
    pub created_at_end: Option<chrono::DateTime<Utc>>,
    pub updated_at_start: Option<chrono::DateTime<Utc>>,
    pub updated_at_end: Option<chrono::DateTime<Utc>>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role_name: Option<String>,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
}
