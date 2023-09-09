use super::{ObjectComponent, ObjectId};
use crate::ContextHandle;
use specs::Entity;

#[derive(Clone)]
pub struct ObjectHandle {
    pub ctx: ContextHandle,
    pub entity: Entity,
    pub object_id: ObjectId,
}

impl ObjectHandle {
    pub fn new(ctx: ContextHandle, entity: Entity, object_id: ObjectId) -> Self {
        Self {
            ctx,
            entity,
            object_id,
        }
    }

    pub fn component<T: ObjectComponent>(&self) -> T {
        T::new(self.clone())
    }

    pub fn name(&self) -> Option<String> {
        self.ctx
            .object_mgr()
            .object_name_registry()
            .name(self.object_id)
            .cloned()
    }

    pub fn parent(&self) -> Option<Self> {
        self.ctx
            .object_mgr()
            .object_hierarchy()
            .parent(self.object_id)
            .map(|parent_id| self.ctx.object_mgr().object_handle(parent_id))
    }

    pub fn set_name(&self, name: impl Into<Option<String>>) {
        self.ctx
            .object_mgr_mut()
            .object_name_registry_mut()
            .set_name(self.object_id, name.into());
    }

    pub fn set_parent(&self, parent: impl Into<Option<Self>>) {
        self.ctx
            .object_mgr_mut()
            .object_hierarchy_mut()
            .set_parent(self.object_id, parent.into().map(|h| h.object_id));
    }

    pub fn remove(&self) {
        self.ctx.object_mgr_mut().remove_object(self);
    }
}

impl PartialEq for ObjectHandle {
    fn eq(&self, other: &Self) -> bool {
        self.object_id == other.object_id
    }
}

impl Eq for ObjectHandle {}
