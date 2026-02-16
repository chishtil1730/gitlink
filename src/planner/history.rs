use super::task::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Add { task: Task },
    Delete { task: Task, index: usize },
    Toggle { id: String },
    UpdateTitle { id: String, old_title: String, new_title: String },
    UpdateDescription { id: String, old_desc: Option<String>, new_desc: Option<String> },
    UpdateTags { id: String, old_tags: Vec<String>, new_tags: Vec<String> },
}

pub struct History {
    undo_stack: Vec<Action>,
    redo_stack: Vec<Action>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, action: Action) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, tasks: &mut Vec<Task>) -> bool {
        if let Some(action) = self.undo_stack.pop() {
            match &action {
                Action::Add { task } => {
                    tasks.retain(|t| t.id != task.id);
                }
                Action::Delete { task, index } => {
                    tasks.insert(*index, task.clone());
                }
                Action::Toggle { id } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.toggle();
                    }
                }
                Action::UpdateTitle { id, old_title, .. } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.update_title(old_title.clone());
                    }
                }
                Action::UpdateDescription { id, old_desc, .. } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.update_description(old_desc.clone());
                    }
                }
                Action::UpdateTags { id, old_tags, .. } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.set_tags(old_tags.clone());
                    }
                }
            }
            self.redo_stack.push(action);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, tasks: &mut Vec<Task>) -> bool {
        if let Some(action) = self.redo_stack.pop() {
            match &action {
                Action::Add { task } => {
                    tasks.push(task.clone());
                }
                Action::Delete { task, .. } => {
                    tasks.retain(|t| t.id != task.id);
                }
                Action::Toggle { id } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.toggle();
                    }
                }
                Action::UpdateTitle { id, new_title, .. } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.update_title(new_title.clone());
                    }
                }
                Action::UpdateDescription { id, new_desc, .. } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.update_description(new_desc.clone());
                    }
                }
                Action::UpdateTags { id, new_tags, .. } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        task.set_tags(new_tags.clone());
                    }
                }
            }
            self.undo_stack.push(action);
            true
        } else {
            false
        }
    }
}