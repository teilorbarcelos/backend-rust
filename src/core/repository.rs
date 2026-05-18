use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct BaseRepository<E: EntityTrait, A: ActiveModelTrait<Entity = E>> {
    pub db: DatabaseConnection,
    _marker: PhantomData<(E, A)>,
}

impl<E, A> BaseRepository<E, A>
where
    E: EntityTrait,
    A: ActiveModelTrait<Entity = E> + ActiveModelBehavior + Send + Sync,
    E::Model: IntoActiveModel<A> + Send + Sync,
{
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            _marker: PhantomData,
        }
    }

    /// Fetches a single record by its primary key
    pub async fn find_by_id<V>(&self, id: V) -> Result<Option<E::Model>, DbErr>
    where
        V: Into<<<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType> + Send,
    {
        E::find_by_id(id).one(&self.db).await
    }

    /// Saves a record to the database (inserts if new, updates if existing)
    pub async fn persist(&self, active_model: A) -> Result<E::Model, DbErr> {
        active_model.insert(&self.db).await
    }

    /// Updates an existing record
    pub async fn update(&self, active_model: A) -> Result<E::Model, DbErr> {
        active_model.update(&self.db).await
    }
}
