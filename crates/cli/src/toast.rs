#![cfg(feature = "gui")]

use crate::gui::{MainWindow, SnackbarItem};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

// Reusable generic toast item retained by the queue.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SnackbarNotice {
    pub text: String,
    pub level: i32,
    pub closing: bool,
}

// Compatibility state retained for the original single-snackbar tests.
#[derive(Default)]
pub struct SnackbarQueue {
    pending: VecDeque<SnackbarNotice>,
    active: Option<SnackbarNotice>,
    // Visible toast stack, oldest first and newest last.
    stack: VecDeque<SnackbarNotice>,
    // Parallel toast IDs, kept strictly in lockstep with `stack`.
    ids: VecDeque<u32>,
    next_id: u32,
}

#[allow(dead_code)]
impl SnackbarQueue {
    pub fn enqueue(&mut self, notice: SnackbarNotice) -> bool {
        let was_idle = self.active.is_none();
        self.pending.push_back(notice.clone());
        was_idle
    }

    pub fn push_toast(&mut self, notice: SnackbarNotice) -> u32 {
        const MAX_VISIBLE_TOASTS: usize = 3;

        // A new toast first displaces the oldest active toast by marking it as
        // closing, instead of deleting it immediately. The actual final removal
        // is scheduled by the UI path after the fade-out/collapse animation.
        if self.active_toast_count() >= MAX_VISIBLE_TOASTS
            && let Some(evicted_id) = self.oldest_active_toast_id()
        {
            let _started = self.begin_toast_closing(evicted_id);
        }

        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.stack.push_back(notice);
        self.ids.push_back(id);
        id
    }

    pub fn take_next(&mut self) -> Option<SnackbarNotice> {
        let next = self.pending.pop_front();
        self.active = next.clone();
        next
    }

    pub fn dismiss(&mut self) -> Option<SnackbarNotice> {
        self.active.as_ref()?;
        self.active = None;
        let next = self.pending.pop_front();
        self.active = next.clone();
        next
    }

    pub fn remove_toast(&mut self, id: u32) -> bool {
        if let Some(index) = self.ids.iter().position(|value| *value == id) {
            self.stack.remove(index);
            self.ids.remove(index);
            true
        } else {
            false
        }
    }

    pub fn begin_toast_closing(&mut self, id: u32) -> bool {
        if let Some(index) = self.ids.iter().position(|value| *value == id) {
            if self.stack[index].closing {
                false
            } else {
                self.stack[index].closing = true;
                true
            }
        } else {
            false
        }
    }

    pub fn finalize_toast_removal(&mut self, id: u32) -> bool {
        if let Some(index) = self.ids.iter().position(|value| *value == id) {
            if self.stack[index].closing {
                self.stack.remove(index);
                self.ids.remove(index);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn active_toast_count(&self) -> usize {
        self.stack.iter().filter(|notice| !notice.closing).count()
    }

    pub fn oldest_active_toast_id(&self) -> Option<u32> {
        self.ids
            .iter()
            .copied()
            .zip(self.stack.iter())
            .find_map(|(id, notice)| (!notice.closing).then_some(id))
    }

    pub fn visible_toasts(&self) -> Vec<SnackbarNotice> {
        self.stack.iter().cloned().collect()
    }
}

pub fn enqueue_snackbar(
    ui: &MainWindow,
    state: &Rc<RefCell<SnackbarQueue>>,
    text: String,
    level: i32,
) {
    const MAX_VISIBLE_TOASTS: usize = 3;
    const TOAST_DISMISS_ANIMATION_DURATION: Duration = Duration::from_millis(350);

    let active_count = state.borrow().active_toast_count();
    let oldest_active_id = state.borrow().oldest_active_toast_id();
    let evicted_id = if active_count >= MAX_VISIBLE_TOASTS {
        oldest_active_id
    } else {
        None
    };

    // Mark the evicted toast before appending the new one, so push_toast can
    // observe the reduced active count and does not schedule a second eviction.
    let eviction_started = if let Some(evicted_id) = evicted_id {
        state.borrow_mut().begin_toast_closing(evicted_id)
    } else {
        false
    };

    let notice = SnackbarNotice {
        text,
        level,
        closing: false,
    };
    let id = state.borrow_mut().push_toast(notice);

    sync_snackbar_stack(ui, state);

    schedule_auto_dismiss(ui.as_weak(), state.clone(), id);

    if eviction_started && let Some(evicted_id) = evicted_id {
        let ui = ui.as_weak();
        let state = state.clone();
        slint::Timer::single_shot(TOAST_DISMISS_ANIMATION_DURATION, move || {
            if let Some(ui) = ui.upgrade() {
                let removed = state.borrow_mut().finalize_toast_removal(evicted_id);
                if removed {
                    sync_snackbar_stack(&ui, &state);
                }
            }
        });
    }
}

pub fn sync_snackbar_stack(ui: &MainWindow, state: &Rc<RefCell<SnackbarQueue>>) {
    let snapshot = state.borrow();
    let items: Vec<SnackbarItem> = snapshot
        .ids
        .iter()
        .zip(snapshot.stack.iter())
        .map(|(id, notice)| SnackbarItem {
            text: notice.text.clone().into(),
            level: notice.level,
            id: *id as i32,
            closing: notice.closing,
        })
        .collect();
    ui.set_snackbar_items(slint::ModelRc::new(slint::VecModel::from(items)));
}

pub fn dismiss_snackbar(ui: &MainWindow, state: &Rc<RefCell<SnackbarQueue>>, toast_id: i32) {
    const TOAST_DISMISS_ANIMATION_DURATION: Duration = Duration::from_millis(350);

    let id = toast_id as u32;
    let started = state.borrow_mut().begin_toast_closing(id);
    if !started {
        return;
    }

    sync_snackbar_stack(ui, state);

    let ui = ui.as_weak();
    let state = state.clone();
    slint::Timer::single_shot(TOAST_DISMISS_ANIMATION_DURATION, move || {
        if let Some(ui) = ui.upgrade() {
            let removed = state.borrow_mut().finalize_toast_removal(id);
            if removed {
                sync_snackbar_stack(&ui, &state);
            }
        }
    });
}

pub fn schedule_auto_dismiss(
    ui: slint::Weak<MainWindow>,
    state: Rc<RefCell<SnackbarQueue>>,
    id: u32,
) {
    slint::Timer::single_shot(Duration::from_secs(4), move || {
        if let Some(ui) = ui.upgrade() {
            let still_present = state.borrow().ids.contains(&id);
            if !still_present {
                return;
            }
            dismiss_snackbar(&ui, &state, id as i32);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{SnackbarNotice, SnackbarQueue};

    #[test]
    fn queue_stores_notice_and_reports_idle() {
        let mut queue = SnackbarQueue::default();
        assert!(queue.enqueue(SnackbarNotice {
            text: "alpha".to_owned(),
            level: 0,
            closing: false,
        }));
        assert_eq!(queue.pending.len(), 1);
        assert!(queue.active.is_none());
    }

    #[test]
    fn take_next_activates_notice() {
        let mut queue = SnackbarQueue::default();
        queue.enqueue(SnackbarNotice {
            text: "alpha".to_owned(),
            level: 1,
            closing: false,
        });
        let notice = queue.take_next().unwrap();
        assert_eq!(notice.text, "alpha");
        assert_eq!(queue.active.as_ref().unwrap().text, "alpha");
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn dismiss_returns_next_notice() {
        let mut queue = SnackbarQueue::default();
        queue.enqueue(SnackbarNotice {
            text: "first".to_owned(),
            level: 0,
            closing: false,
        });
        queue.enqueue(SnackbarNotice {
            text: "second".to_owned(),
            level: 2,
            closing: false,
        });
        queue.take_next();
        let next = queue.dismiss().unwrap();
        assert_eq!(next.text, "second");
        assert_eq!(queue.active.as_ref().unwrap().text, "second");
    }

    #[test]
    fn dismiss_on_idle_returns_none() {
        let mut queue = SnackbarQueue::default();
        assert!(queue.dismiss().is_none());
    }

    #[test]
    fn toast_stack_stores_oldest_to_newest_notices_and_ids() {
        let mut queue = SnackbarQueue::default();
        let first = queue.push_toast(SnackbarNotice {
            text: "first".to_owned(),
            level: 0,
            closing: false,
        });
        let second = queue.push_toast(SnackbarNotice {
            text: "second".to_owned(),
            level: 1,
            closing: false,
        });

        assert_eq!(queue.stack.len(), 2);
        assert_eq!(queue.ids.len(), 2);
        assert_eq!(
            queue
                .visible_toasts()
                .iter()
                .map(|item| item.text.clone())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(
            queue.ids.iter().copied().collect::<Vec<_>>(),
            vec![first, second]
        );
        assert_ne!(first, second);
    }

    #[test]
    fn toast_stack_begins_animated_eviction_at_three() {
        let mut queue = SnackbarQueue::default();
        let first = queue.push_toast(SnackbarNotice {
            text: "first".to_owned(),
            level: 0,
            closing: false,
        });
        let second = queue.push_toast(SnackbarNotice {
            text: "second".to_owned(),
            level: 0,
            closing: false,
        });
        let third = queue.push_toast(SnackbarNotice {
            text: "third".to_owned(),
            level: 0,
            closing: false,
        });
        let fourth = queue.push_toast(SnackbarNotice {
            text: "fourth".to_owned(),
            level: 0,
            closing: false,
        });

        assert_eq!(queue.stack.len(), 4);
        assert_eq!(queue.ids.len(), 4);
        assert_eq!(
            queue
                .visible_toasts()
                .iter()
                .map(|item| item.text.clone())
                .collect::<Vec<_>>(),
            vec!["first", "second", "third", "fourth"]
        );
        assert!(queue.stack[0].closing);
        assert!(!queue.stack[1].closing);
        assert!(!queue.stack[2].closing);
        assert!(!queue.stack[3].closing);
        assert_eq!(
            queue.ids.iter().copied().collect::<Vec<_>>(),
            vec![first, second, third, fourth]
        );
        assert_eq!(third, 2);
        assert_eq!(fourth, 3);
        assert!(!queue.begin_toast_closing(first));
        assert_eq!(queue.stack.len(), 4);
        assert!(queue.finalize_toast_removal(first));
        assert_eq!(queue.stack.len(), 3);
        assert_eq!(
            queue
                .visible_toasts()
                .iter()
                .map(|item| item.text.clone())
                .collect::<Vec<_>>(),
            vec!["second", "third", "fourth"]
        );
        assert_eq!(
            queue.ids.iter().copied().collect::<Vec<_>>(),
            vec![second, third, fourth]
        );
    }

    #[test]
    fn toast_close_is_two_phase() {
        let mut queue = SnackbarQueue::default();
        let id = queue.push_toast(SnackbarNotice {
            text: "first".to_owned(),
            level: 2,
            closing: false,
        });

        assert!(queue.begin_toast_closing(id));
        assert_eq!(queue.stack.len(), 1);
        assert!(queue.stack[0].closing);
        assert!(!queue.begin_toast_closing(id));
        assert!(!queue.finalize_toast_removal(id.wrapping_add(1)));
        assert!(queue.finalize_toast_removal(id));
        assert!(queue.stack.is_empty());
        assert!(queue.ids.is_empty());
    }

    #[test]
    fn toast_stack_removes_only_requested_id() {
        let mut queue = SnackbarQueue::default();
        let first = queue.push_toast(SnackbarNotice {
            text: "first".to_owned(),
            level: 0,
            closing: false,
        });
        let second = queue.push_toast(SnackbarNotice {
            text: "second".to_owned(),
            level: 0,
            closing: false,
        });

        assert!(queue.remove_toast(first));
        assert_eq!(queue.stack.len(), 1);
        assert_eq!(queue.ids.len(), 1);
        assert_eq!(queue.stack[0].text, "second");
        assert_eq!(queue.ids[0], second);
    }

    #[test]
    fn toast_stack_ignores_unknown_ids() {
        let mut queue = SnackbarQueue::default();
        let id = queue.push_toast(SnackbarNotice {
            text: "first".to_owned(),
            level: 2,
            closing: false,
        });
        let expected_stack = queue.visible_toasts();
        let expected_ids = queue.ids.clone();

        assert!(!queue.remove_toast(id.wrapping_add(1)));
        assert!(!queue.remove_toast(u32::MAX));
        assert_eq!(queue.visible_toasts(), expected_stack);
        assert_eq!(queue.ids, expected_ids);
    }
}
