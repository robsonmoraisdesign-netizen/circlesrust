// SPDX-License-Identifier: Apache-2.0
//! Bounded transform commands, with one history entry per pointer gesture.
use crate::integrator::Scene;
use super::transform::ObjectTransform;

#[derive(Clone, Copy)]
struct Change {
    object: usize,
    before: ObjectTransform,
    after: ObjectTransform,
}

#[derive(Default)]
pub struct TransformHistory {
    undo: Vec<Change>,
    redo: Vec<Change>,
    pending: Option<Change>,
}

impl TransformHistory {
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() || self.pending.is_some() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() && self.pending.is_none() }
    pub fn is_editing(&self) -> bool { self.pending.is_some() }

    /// Invalid/no-op updates never enter history. The caller finishes the gesture.
    pub fn apply(&mut self, scene: &mut Scene, object: usize, value: ObjectTransform)
        -> Result<bool, &'static str>
    {
        let before = *scene.object_transforms.get(object).ok_or("Object index out of range")?;
        if !value.valid() { return Err("Invalid object transform values"); }
        if before == value { return Ok(false); }
        scene.apply_object_transform(object, value)?;
        if self.pending.is_some_and(|change| change.object != object) { self.finish(); }
        match &mut self.pending {
            Some(change) => change.after = value,
            None => self.pending = Some(Change { object, before, after: value }),
        }
        Ok(true)
    }

    pub fn finish(&mut self) {
        if let Some(change) = self.pending.take() {
            if change.before != change.after {
                self.redo.clear();
                if self.undo.len() == 128 { self.undo.remove(0); }
                self.undo.push(change);
            }
        }
    }

    /// Cancel restores geometry without destroying the existing redo branch.
    pub fn cancel(&mut self, scene: &mut Scene) -> Result<Option<usize>, &'static str> {
        let Some(change) = self.pending else { return Ok(None); };
        scene.apply_object_transform(change.object, change.before)?;
        self.pending = None;
        Ok(Some(change.object))
    }

    pub fn undo(&mut self, scene: &mut Scene) -> Result<Option<usize>, &'static str> {
        self.finish();
        let Some(change) = self.undo.last().copied() else { return Ok(None); };
        scene.apply_object_transform(change.object, change.before)?;
        self.undo.pop();
        self.redo.push(change);
        Ok(Some(change.object))
    }

    pub fn redo(&mut self, scene: &mut Scene) -> Result<Option<usize>, &'static str> {
        self.finish();
        let Some(change) = self.redo.last().copied() else { return Ok(None); };
        scene.apply_object_transform(change.object, change.after)?;
        self.redo.pop();
        self.undo.push(change);
        Ok(Some(change.object))
    }
}
