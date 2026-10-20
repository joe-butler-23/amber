use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use injector_derive::ScopeInjectable;

use crate::common::repository_error::RepositoryError;
use crate::elements::repositories::meta_repository::MetaRepository;
use crate::elements::services::element_index_service::ElementIndexService;
use crate::elements::services::priority_service::PriorityService;
use crate::elements::value_objects::element_id::ElementId;
use crate::trash::repositories::trash_repository::TrashRepository;
use crate::trash::services::trash_service::{TrashService, TrashServiceError};
use crate::trash::value_objects::trashed_element::TrashedElement;

#[derive(ScopeInjectable)]
pub struct DefaultTrashService {
    trash_repository: Arc<dyn TrashRepository>,
    meta_repository: Arc<dyn MetaRepository>,
    element_index_service: Arc<dyn ElementIndexService>,
    priority_service: Arc<dyn PriorityService>,
}

#[async_trait]
impl TrashService for DefaultTrashService {
    async fn trash_element(&self, id: ElementId) -> Result<(), TrashServiceError> {
        self.trash_repository.trash(id, Utc::now()).await?;
        Ok(())
    }

    async fn trash_many(&self, ids: Vec<ElementId>) -> Result<(), TrashServiceError> {
        for id in ids {
            self.trash_repository.trash(id, Utc::now()).await?;
        }
        Ok(())
    }

    async fn restore_element(&self, id: ElementId) -> Result<(), TrashServiceError> {
        let restored_ids = self.trash_repository.restore(id).await?;
        self.reassign_priorities(restored_ids).await?;

        // An ancestor may have been permanently deleted in the meantime, or
        // still be trashed itself. Either way the element would come back
        // invisible, so it is put back at the root instead.
        if !self.trash_repository.has_live_ancestry(id).await? {
            let position = self.element_index_service.get_new_last_index(None).await?;
            self.meta_repository.move_to(id, None, position).await?;
        }

        Ok(())
    }

    async fn list_trash(&self) -> Result<Vec<TrashedElement>, TrashServiceError> {
        Ok(self.trash_repository.get_trashed_roots().await?)
    }

    async fn delete_permanently(&self, id: ElementId) -> Result<(), TrashServiceError> {
        if !self.trash_repository.is_trashed(id).await? {
            return Err(TrashServiceError::NotInTrash);
        }
        self.meta_repository.delete(id).await?;
        Ok(())
    }

    // Deleting a root cascades to its subtree, so a root nested inside another
    // one may already be gone by the time its turn comes; the delete is then a
    // no-op rather than an error.
    async fn empty_trash(&self) -> Result<(), TrashServiceError> {
        for root in self.trash_repository.get_trashed_roots().await? {
            self.meta_repository.delete(root.element_id).await?;
        }
        Ok(())
    }

    async fn purge_expired(&self, retention_days: u32) -> Result<usize, RepositoryError> {
        let cutoff = Utc::now() - Duration::days(retention_days.into());
        let expired = self
            .trash_repository
            .get_roots_trashed_before(cutoff)
            .await?;

        for id in &expired {
            log::info!("Purging trashed element {id:?} past the retention threshold.");
            self.meta_repository.delete(*id).await?;
        }

        Ok(expired.len())
    }
}

impl DefaultTrashService {
    /// A restored element's old priority may have been reclaimed by a live
    /// element while it was trashed, so it isn't safe to reuse as-is.
    async fn reassign_priorities(&self, ids: Vec<ElementId>) -> Result<(), TrashServiceError> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut metas = Vec::with_capacity(ids.len());
        for id in ids {
            metas.push(self.meta_repository.get_by_id(id.id()).await?);
        }
        metas.sort_by(|a, b| a.priority.cmp(&b.priority));

        let old_priorities: Vec<_> = metas.iter().map(|meta| meta.priority.clone()).collect();
        let new_priorities = self
            .priority_service
            .get_priorities_for_restore(&old_priorities)
            .await?;

        for (meta, priority) in metas.into_iter().zip(new_priorities) {
            self.meta_repository
                .set_priority(meta.element_id, priority)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use fractional_index::FractionalIndex;
    use injector::{injector::Injector, register_scope};
    use uuid::Uuid;

    use crate::{
        elements::{
            entities::{
                folder::Folder,
                learning_asset::{LearningAsset, LearningAssetContent},
            },
            repositories::{
                folder_repository::FolderRepository,
                learning_asset_repository::LearningAssetRepository,
            },
            services::implementations::{
                default_element_index_service::DefaultElementIndexService,
                default_priority_service::DefaultPriorityService,
            },
            services::priority_service::PriorityService,
            value_objects::{meta::Meta, read_point::ReadPoint},
        },
        infrastructure::repositories::sqlite::{
            sqlite_folder_repository::SqliteFolderRepository,
            sqlite_learning_asset_repository::SqliteLearningAssetRepository,
            sqlite_meta_repository::SqliteMetaRepository,
            sqlite_trash_repository::SqliteTrashRepository,
        },
        test_utils::create_test_injector,
    };

    use super::*;

    async fn initialize_test_injector() -> Injector {
        let mut injector = create_test_injector().await;

        register_scope!(injector, dyn FolderRepository, SqliteFolderRepository);
        register_scope!(
            injector,
            dyn LearningAssetRepository,
            SqliteLearningAssetRepository
        );
        register_scope!(injector, dyn MetaRepository, SqliteMetaRepository);
        register_scope!(injector, dyn TrashRepository, SqliteTrashRepository);
        register_scope!(
            injector,
            dyn ElementIndexService,
            DefaultElementIndexService
        );
        register_scope!(injector, dyn PriorityService, DefaultPriorityService);
        register_scope!(injector, dyn TrashService, DefaultTrashService);

        injector
    }

    fn make_meta(element_id: ElementId) -> Meta {
        Meta {
            element_id,
            name: "test".into(),
            parent: None,
            position: FractionalIndex::default(),
            priority: FractionalIndex::default(),
            study_profile_id: None,
            bibliographical_source_id: None,
            derived_from: None,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        }
    }

    fn make_folder(name: &str, parent: Option<ElementId>) -> Folder {
        Folder {
            meta: Meta {
                name: name.into(),
                parent,
                ..make_meta(ElementId::Folder(Uuid::new_v4()))
            },
        }
    }

    fn make_learning_asset(name: &str, parent: Option<ElementId>) -> LearningAsset {
        LearningAsset {
            r#type: Default::default(),
            interval_multiplier: 1.2,
            read_point: ReadPoint::default(),
            meta: Meta {
                name: name.into(),
                parent,
                ..make_meta(ElementId::LearningAsset(Uuid::new_v4()))
            },
        }
    }

    #[tokio::test]
    async fn restore_element_descendant_is_a_trash_root_leaves_that_descendant_in_the_trash() {
        // Arrange

        let injector = initialize_test_injector().await;
        let scope = injector.start_scope();
        let service = scope.resolve::<dyn TrashService>().await;
        let trash_repository = scope.resolve::<dyn TrashRepository>().await;

        let folder = make_folder("Science", None);
        let folder_id = folder.meta.element_id;
        let learning_asset = make_learning_asset("Photosynthesis", Some(folder_id));
        let learning_asset_id = learning_asset.meta.element_id;
        scope
            .resolve::<dyn FolderRepository>()
            .await
            .create(folder)
            .await
            .unwrap();
        scope
            .resolve::<dyn LearningAssetRepository>()
            .await
            .create(learning_asset, LearningAssetContent::Extracted(Vec::new()))
            .await
            .unwrap();

        service.trash_element(folder_id).await.unwrap();
        // Makes the learning asset a trash root of its own inside the trashed folder,
        // the way an incoming sync of a trash state can.
        trash_repository
            .trash(learning_asset_id, Utc::now())
            .await
            .unwrap();

        // Act

        service.restore_element(folder_id).await.unwrap();

        // Assert

        let trashed = service.list_trash().await.unwrap();
        assert_eq!(1, trashed.len());
        assert_eq!(learning_asset_id, trashed[0].element_id);
    }

    #[tokio::test]
    async fn restore_element_old_priority_was_reclaimed_gets_a_priority_that_does_not_collide() {
        // Arrange

        let injector = initialize_test_injector().await;
        let scope = injector.start_scope();
        let service = scope.resolve::<dyn TrashService>().await;
        let folder_repository = scope.resolve::<dyn FolderRepository>().await;
        let meta_repository = scope.resolve::<dyn MetaRepository>().await;

        let folder = make_folder("Science", None);
        let folder_id = folder.meta.element_id;
        let old_priority = folder.meta.priority.clone();
        folder_repository.create(folder).await.unwrap();
        service.trash_element(folder_id).await.unwrap();

        // While the folder is trashed, another element claims the exact
        // priority it left behind — the way an unrelated insertion would once
        // the trashed row stops counting as an obstacle.
        let other = make_folder("Reclaimed", None);
        let other_id = other.meta.element_id;
        folder_repository.create(other).await.unwrap();
        meta_repository
            .set_priority(other_id, old_priority)
            .await
            .unwrap();

        // Act

        service.restore_element(folder_id).await.unwrap();

        // Assert

        let restored = meta_repository.get_by_id(folder_id.id()).await.unwrap();
        let other_meta = meta_repository.get_by_id(other_id.id()).await.unwrap();
        assert_ne!(restored.priority, other_meta.priority);
    }

    #[tokio::test]
    async fn purge_expired_element_trashed_past_retention_deletes_element() {
        // Arrange

        let injector = initialize_test_injector().await;
        let scope = injector.start_scope();
        let service = scope.resolve::<dyn TrashService>().await;
        let trash_repository = scope.resolve::<dyn TrashRepository>().await;
        let meta_repository = scope.resolve::<dyn MetaRepository>().await;

        let folder = make_folder("Science", None);
        let folder_id = folder.meta.element_id;
        scope
            .resolve::<dyn FolderRepository>()
            .await
            .create(folder)
            .await
            .unwrap();
        trash_repository
            .trash(folder_id, Utc::now() - Duration::days(31))
            .await
            .unwrap();

        // Act

        let actual = service.purge_expired(30).await.unwrap();

        // Assert

        assert_eq!(1, actual);
        assert!(!meta_repository.exists(folder_id).await.unwrap());
    }
}
