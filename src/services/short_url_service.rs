use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, QueryOrder, QuerySelect, PaginatorTrait,
};
use sea_orm::prelude::{DateTimeWithTimeZone, Expr};
use chrono::Utc;

use crate::{
    cache::{CACHE_EXPIRED_VALUE, CACHE_NULL_VALUE, RedisCache},
    common::error::AppError,
    models::short_url::{ActiveModel, Column, Entity, Model},
    utils::base62,
};

const STATUS_ACTIVE: i16 = 1;
const STATUS_DISABLED: i16 = 0;
const NOT_DELETED: i16 = 0;

#[derive(Debug, Clone)]
pub struct ShortUrlStats {
    pub total: u64,
    pub active_count: u64,
    pub disabled_count: u64,
    pub expired_count: u64,
}

#[derive(Clone)]
pub struct ShortUrlService {
    db: DatabaseConnection,
    cache: Option<RedisCache>,
}

impl ShortUrlService {
    pub fn new(db: DatabaseConnection, cache: Option<RedisCache>) -> Self {
        Self { db, cache }
    }

    pub async fn create_short_url(
        &self,
        original_url: String,
        expires_at: Option<DateTimeWithTimeZone>,
    ) -> Result<Model, AppError> {
        let active = ActiveModel {
            original_url: Set(original_url),
            status: Set(STATUS_ACTIVE),
            is_deleted: Set(NOT_DELETED),
            visit_count: Set(0),
            expires_at: Set(expires_at),
            ..Default::default()
        };

        let inserted = active
            .insert(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to create short url: {err}")))?;

        let code = base62::encode(inserted.id);

        let updated = ActiveModel {
            id: Set(inserted.id),
            short_code: Set(Some(code)),
            ..Default::default()
        };

        let saved = updated
            .update(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to update short url: {err}")))?;

        if let Some(cache) = &self.cache {
            let cached = CachedShortUrl::from_model(&saved);
            let _ = cache.set_json(&cache_key(saved.short_code.as_deref().unwrap_or_default()), &cached).await;
            let _ = cache.set_string(&cache_key_by_id(saved.id), saved.short_code.as_deref().unwrap_or_default()).await;
        }

        Ok(saved)
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Model, AppError> {
        if let Some(cache) = &self.cache {
            let key = cache_key(code);
            if let Ok(Some(value)) = cache.get_string(&key).await {
                if value == CACHE_NULL_VALUE {
                    return Err(AppError::not_found("short url not found"));
                }
                if value == CACHE_EXPIRED_VALUE {
                    return Err(AppError::gone("short url expired"));
                }
                if let Ok(Some(cached)) = cache.get_json::<CachedShortUrl>(&key).await {
                    let model = cached.into_model();
                    if let Some(expires_at) = model.expires_at {
                        if expires_at <= Utc::now().fixed_offset() {
                            return Err(AppError::gone("short url expired"));
                        }
                    }
                    return Ok(model);
                }
            }
        }

        let key = cache_key(code);
        let lock_key = lock_key(code);
        let mut locked = false;
        if let Some(cache) = &self.cache {
            if let Ok(acquired) = cache.try_lock(&lock_key, 2000).await {
                locked = acquired;
            }
            if !locked {
                for _ in 0..3 {
                    RedisCache::sleep_backoff(50).await;
                    if let Ok(Some(value)) = cache.get_string(&key).await {
                        if value == CACHE_NULL_VALUE {
                            return Err(AppError::not_found("short url not found"));
                        }
                        if value == CACHE_EXPIRED_VALUE {
                            return Err(AppError::gone("short url expired"));
                        }
                        if let Ok(Some(cached)) = cache.get_json::<CachedShortUrl>(&key).await {
                            return Ok(cached.into_model());
                        }
                    }
                }
            }
        }

        let model = Entity::find()
            .filter(Column::ShortCode.eq(code))
            .filter(Column::IsDeleted.eq(NOT_DELETED))
            .filter(Column::Status.eq(STATUS_ACTIVE))
            .one(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to query short url: {err}")))?;

        let model = match model {
            Some(model) => model,
            None => {
                if let Some(cache) = &self.cache {
                    let _ = cache.set_null(&key).await;
                    if locked {
                        let _ = cache.unlock(&lock_key).await;
                    }
                }
                return Err(AppError::not_found("short url not found"));
            }
        };
        if let Some(expires_at) = model.expires_at {
            if expires_at <= Utc::now().fixed_offset() {
                if let Some(cache) = &self.cache {
                    let _ = cache.set_expired(&key).await;
                    if locked {
                        let _ = cache.unlock(&lock_key).await;
                    }
                }
                return Err(AppError::gone("short url expired"));
            }
        }
        if let Some(cache) = &self.cache {
            let cached = CachedShortUrl::from_model(&model);
            let _ = cache.set_json(&key, &cached).await;
            if let Some(code) = model.short_code.as_deref() {
                let _ = cache.set_string(&cache_key_by_id(model.id), code).await;
            }
            if locked {
                let _ = cache.unlock(&lock_key).await;
            }
        }
        Ok(model)
    }

    pub async fn increment_visit_count(&self, id: i64) -> Result<(), AppError> {
        let result = Entity::update_many()
            .col_expr(Column::VisitCount, Expr::col(Column::VisitCount).add(1))
            .col_expr(Column::UpdatedAt, Expr::value(Utc::now().fixed_offset()))
            .filter(Column::Id.eq(id))
            .exec(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to update visit count: {err}")))?;

        if result.rows_affected == 0 {
            return Err(AppError::not_found("short url not found"));
        }

        if let Some(cache) = &self.cache {
            if let Ok(Some(code)) = cache.get_string(&cache_key_by_id(id)).await {
                let _ = cache.del(&cache_key(&code)).await;
            }
            let _ = cache.del(&cache_key_by_id(id)).await;
        }

        Ok(())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Model, AppError> {
        let model = Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to query short url: {err}")))?;

        model.ok_or_else(|| AppError::not_found("short url not found"))
    }

    pub async fn list_short_urls(
        &self,
        limit: u64,
        offset: u64,
        status: Option<i16>,
        is_deleted: Option<i16>,
    ) -> Result<(u64, Vec<Model>), AppError> {
        let mut query = Entity::find();

        if let Some(status) = status {
            query = query.filter(Column::Status.eq(status));
        }
        if let Some(is_deleted) = is_deleted {
            query = query.filter(Column::IsDeleted.eq(is_deleted));
        }

        let total = query
            .clone()
            .count(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to count short urls: {err}")))?;

        let models = query
            .order_by_desc(Column::Id)
            .offset(offset)
            .limit(limit)
            .all(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to list short urls: {err}")))?;

        Ok((total, models))
    }

    pub async fn short_url_stats(&self) -> Result<ShortUrlStats, AppError> {
        let now = Utc::now().fixed_offset();

        let total = Entity::find()
            .count(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to count total short urls: {err}")))?;

        let active_count = Entity::find()
            .filter(Column::IsDeleted.eq(NOT_DELETED))
            .filter(Column::Status.eq(STATUS_ACTIVE))
            .count(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to count active short urls: {err}")))?;

        let disabled_count = Entity::find()
            .filter(Column::IsDeleted.eq(NOT_DELETED))
            .filter(Column::Status.eq(STATUS_DISABLED))
            .count(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to count disabled short urls: {err}")))?;

        let expired_count = Entity::find()
            .filter(Column::IsDeleted.eq(NOT_DELETED))
            .filter(Column::ExpiresAt.is_not_null())
            .filter(Column::ExpiresAt.lte(now))
            .count(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to count expired short urls: {err}")))?;

        Ok(ShortUrlStats {
            total,
            active_count,
            disabled_count,
            expired_count,
        })
    }

    pub async fn update_short_url(
        &self,
        id: i64,
        original_url: Option<String>,
        status: Option<i16>,
        is_deleted: Option<i16>,
        expires_at: Option<Option<DateTimeWithTimeZone>>,
    ) -> Result<Model, AppError> {
        let model = self.find_by_id(id).await?;
        let code = model.short_code.clone();
        let mut active: ActiveModel = model.into();
        active.updated_at = Set(Utc::now().fixed_offset());

        if let Some(url) = original_url {
            active.original_url = Set(url);
        }
        if let Some(status) = status {
            active.status = Set(status);
        }
        if let Some(is_deleted) = is_deleted {
            active.is_deleted = Set(is_deleted);
            active.deleted_at = if is_deleted == 1 {
                Set(Some(Utc::now().fixed_offset()))
            } else {
                Set(None)
            };
        }
        if let Some(expires_at) = expires_at {
            active.expires_at = Set(expires_at);
        }

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to update short url: {err}")))?;

        if let Some(cache) = &self.cache {
            if let Some(code) = code.as_deref() {
                let _ = cache.del(&cache_key(code)).await;
            }
            let _ = cache.del(&cache_key_by_id(id)).await;
        }
        Ok(updated)
    }

    pub async fn soft_delete(&self, id: i64) -> Result<(), AppError> {
        let model = self.find_by_id(id).await?;
        let code = model.short_code.clone();
        let mut active: ActiveModel = model.into();
        active.is_deleted = Set(1);
        active.status = Set(STATUS_DISABLED);
        active.deleted_at = Set(Some(Utc::now().fixed_offset()));
        active.updated_at = Set(Utc::now().fixed_offset());

        active
            .update(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to delete short url: {err}")))?;

        if let Some(cache) = &self.cache {
            if let Some(code) = code.as_deref() {
                let _ = cache.del(&cache_key(code)).await;
            }
            let _ = cache.del(&cache_key_by_id(id)).await;
        }

        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedShortUrl {
    id: i64,
    short_code: Option<String>,
    original_url: String,
    visit_count: i64,
    status: i16,
    is_deleted: i16,
    created_at: DateTimeWithTimeZone,
    updated_at: DateTimeWithTimeZone,
    deleted_at: Option<DateTimeWithTimeZone>,
    expires_at: Option<DateTimeWithTimeZone>,
}

impl CachedShortUrl {
    fn from_model(model: &Model) -> Self {
        Self {
            id: model.id,
            short_code: model.short_code.clone(),
            original_url: model.original_url.clone(),
            visit_count: model.visit_count,
            status: model.status,
            is_deleted: model.is_deleted,
            created_at: model.created_at,
            updated_at: model.updated_at,
            deleted_at: model.deleted_at,
            expires_at: model.expires_at,
        }
    }

    fn into_model(self) -> Model {
        Model {
            id: self.id,
            short_code: self.short_code,
            original_url: self.original_url,
            visit_count: self.visit_count,
            status: self.status,
            is_deleted: self.is_deleted,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
            expires_at: self.expires_at,
        }
    }
}

fn cache_key(code: &str) -> String {
    format!("short_url:code:{code}")
}

fn cache_key_by_id(id: i64) -> String {
    format!("short_url:id:{id}")
}

fn lock_key(code: &str) -> String {
    format!("short_url:lock:{code}")
}
