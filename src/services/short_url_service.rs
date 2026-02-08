use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, QueryOrder, QuerySelect, PaginatorTrait,
};
use sea_orm::prelude::{DateTimeWithTimeZone, Expr};
use chrono::Utc;

use crate::{
    common::error::AppError,
    models::short_url::{ActiveModel, Column, Entity, Model},
    utils::base62,
};

const STATUS_ACTIVE: i16 = 1;
const STATUS_DISABLED: i16 = 0;
const NOT_DELETED: i16 = 0;

#[derive(Clone)]
pub struct ShortUrlService {
    db: DatabaseConnection,
}

impl ShortUrlService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
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

        Ok(saved)
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Model, AppError> {
        let model = Entity::find()
            .filter(Column::ShortCode.eq(code))
            .filter(Column::IsDeleted.eq(NOT_DELETED))
            .filter(Column::Status.eq(STATUS_ACTIVE))
            .one(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to query short url: {err}")))?;

        model.ok_or_else(|| AppError::not_found("short url not found"))
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

    pub async fn update_short_url(
        &self,
        id: i64,
        original_url: Option<String>,
        status: Option<i16>,
        is_deleted: Option<i16>,
        expires_at: Option<Option<DateTimeWithTimeZone>>,
    ) -> Result<Model, AppError> {
        let model = self.find_by_id(id).await?;
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

        active
            .update(&self.db)
            .await
            .map_err(|err| AppError::internal(format!("failed to update short url: {err}")))
    }

    pub async fn soft_delete(&self, id: i64) -> Result<(), AppError> {
        let model = self.find_by_id(id).await?;
        let mut active: ActiveModel = model.into();
        active.is_deleted = Set(1);
        active.status = Set(STATUS_DISABLED);
        active.deleted_at = Set(Some(Utc::now().fixed_offset()));
        active.updated_at = Set(Utc::now().fixed_offset());

        active
            .update(&self.db)
            .await
            .map(|_| ())
            .map_err(|err| AppError::internal(format!("failed to delete short url: {err}")))
    }
}
