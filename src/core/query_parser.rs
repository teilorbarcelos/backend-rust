use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use chrono::{NaiveDate, TimeZone, Utc};

#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct FilterParams {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub searchWord: Option<String>,
    pub searchFields: Option<String>,
    pub orderBy: Option<String>,
    pub orderDirection: Option<String>,
    pub createdAt_start: Option<String>,
    pub createdAt_end: Option<String>,
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
        params: &FilterParams,
        allowed_search_fields: &[&str],
    ) -> Result<ParsedFilters, AppError> {
        // 1. If searchWord is provided, searchFields must be provided
        if params.searchWord.is_some() && params.searchFields.is_none() {
            return Err(AppError::BadRequest(
                "O parâmetro 'searchFields' é obrigatório quando 'searchWord' é fornecido.".to_string(),
            ));
        }

        let mut parsed_search_fields = Vec::new();

        // 2. Validate all provided search fields
        if let Some(ref fields_str) = params.searchFields {
            for field in fields_str.split(',') {
                let trimmed = field.trim();
                if !trimmed.is_empty() {
                    if !allowed_search_fields.contains(&trimmed) {
                        return Err(AppError::BadRequest(
                            format!("Busca não permitida no campo '{}'.", trimmed),
                        ));
                    }
                    parsed_search_fields.push(trimmed.to_string());
                }
            }
        }

        // 3. Parse date ranges
        let mut start_date = None;
        let mut end_date = None;

        if let Some(ref start_str) = params.createdAt_start {
            let parsed = NaiveDate::parse_from_str(start_str, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest("Formato de 'createdAt_start' inválido. Use YYYY-MM-DD.".to_string()))?;
            // Set to beginning of the day (00:00:00)
            let dt = Utc.from_utc_datetime(&parsed.and_hms_opt(0, 0, 0).unwrap());
            start_date = Some(dt);
        }

        if let Some(ref end_str) = params.createdAt_end {
            let parsed = NaiveDate::parse_from_str(end_str, "%Y-%m-%d")
                .map_err(|_| AppError::BadRequest("Formato de 'createdAt_end' inválido. Use YYYY-MM-DD.".to_string()))?;
            // Set to end of the day (23:59:59)
            let dt = Utc.from_utc_datetime(&parsed.and_hms_opt(23, 59, 59).unwrap());
            end_date = Some(dt);
        }

        let page = params.page.unwrap_or(0);
        let size = params.size.unwrap_or(25);
        let order_by = params.orderBy.clone();
        let order_direction = params.orderDirection.clone().unwrap_or_else(|| "asc".to_string());

        Ok(ParsedFilters {
            page,
            size,
            search_word: params.searchWord.clone(),
            search_fields: parsed_search_fields,
            order_by,
            order_direction,
            start_date,
            end_date,
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
    pub start_date: Option<chrono::DateTime<Utc>>,
    pub end_date: Option<chrono::DateTime<Utc>>,
}
