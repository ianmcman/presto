//! Queue operations (play next, add to queue) as SetQueue edits (D-08).
use presto_core::mirror::QueueMirror;

pub enum QueueOp {
    PlayFrom(u32),
    Remove(u32),
    PlayNext(String),
    Add(String),
}

#[derive(Debug, PartialEq)]
pub struct Edit {
    pub ids: Vec<String>,
    pub start: u32,
    pub keep_position: bool,
}

#[derive(Debug, PartialEq)]
pub enum Plan {
    Edit(Edit),
    Clear,
    Noop,
}

pub fn apply(q: &QueueMirror, op: &QueueOp) -> Plan {
    let mut ids = q.ids();
    let cur = q.index.unwrap_or(0) as usize;
    let edit = |ids, start: usize, keep_position| Plan::Edit(Edit { ids, start: start as u32, keep_position });
    if ids.is_empty() {
        return match op {
            QueueOp::PlayNext(id) | QueueOp::Add(id) => edit(vec![id.clone()], 0, false),
            _ => Plan::Noop,
        };
    }
    match op {
        QueueOp::PlayFrom(i) if (*i as usize) < ids.len() => edit(ids, *i as usize, false),
        QueueOp::Remove(i) => {
            let i = *i as usize;
            if i >= ids.len() {
                return Plan::Noop;
            }
            if ids.len() == 1 {
                return Plan::Clear;
            }
            ids.remove(i);
            match i.cmp(&cur) {
                std::cmp::Ordering::Less => edit(ids, cur - 1, true),
                std::cmp::Ordering::Greater => edit(ids, cur, true),
                // current removed: the next track takes its slot, or the new last one
                std::cmp::Ordering::Equal => {
                    let start = cur.min(ids.len() - 1);
                    edit(ids, start, false)
                }
            }
        }
        QueueOp::PlayNext(id) => {
            ids.insert((cur + 1).min(ids.len()), id.clone());
            edit(ids, cur, true)
        }
        QueueOp::Add(id) => {
            ids.push(id.clone());
            edit(ids, cur, true)
        }
        QueueOp::PlayFrom(_) => Plan::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use presto_ipc::QueueItem;

    fn q(ids: &[&str], index: Option<u32>) -> QueueMirror {
        let items = ids
            .iter()
            .map(|id| QueueItem { id: id.to_string(), title: String::new(), artist: String::new(), album: String::new(), duration_ms: 0, artwork_url: None, playable: true })
            .collect();
        QueueMirror { items, index, ..Default::default() }
    }

    fn e(ids: &[&str], start: u32, keep: bool) -> Plan {
        Plan::Edit(Edit { ids: ids.iter().map(|s| s.to_string()).collect(), start, keep_position: keep })
    }

    fn abcd() -> QueueMirror {
        q(&["a", "b", "c", "d"], Some(1))
    }

    #[test]
    fn play_from() {
        assert_eq!(apply(&abcd(), &QueueOp::PlayFrom(3)), e(&["a", "b", "c", "d"], 3, false));
        assert_eq!(apply(&abcd(), &QueueOp::PlayFrom(4)), Plan::Noop);
    }

    #[test]
    fn remove_after_current() {
        assert_eq!(apply(&abcd(), &QueueOp::Remove(2)), e(&["a", "b", "d"], 1, true));
    }

    #[test]
    fn remove_before_current() {
        assert_eq!(apply(&abcd(), &QueueOp::Remove(0)), e(&["b", "c", "d"], 0, true));
    }

    #[test]
    fn remove_current() {
        assert_eq!(apply(&abcd(), &QueueOp::Remove(1)), e(&["a", "c", "d"], 1, false));
    }

    #[test]
    fn remove_current_last() {
        assert_eq!(apply(&q(&["a", "b"], Some(1)), &QueueOp::Remove(1)), e(&["a"], 0, false));
    }

    #[test]
    fn remove_single_and_out_of_range() {
        assert_eq!(apply(&q(&["a"], Some(0)), &QueueOp::Remove(0)), Plan::Clear);
        assert_eq!(apply(&abcd(), &QueueOp::Remove(9)), Plan::Noop);
    }

    #[test]
    fn play_next_and_add() {
        assert_eq!(apply(&abcd(), &QueueOp::PlayNext("x".into())), e(&["a", "b", "x", "c", "d"], 1, true));
        assert_eq!(apply(&abcd(), &QueueOp::Add("x".into())), e(&["a", "b", "c", "d", "x"], 1, true));
    }

    #[test]
    fn empty_queue() {
        let empty = q(&[], None);
        assert_eq!(apply(&empty, &QueueOp::PlayNext("x".into())), e(&["x"], 0, false));
        assert_eq!(apply(&empty, &QueueOp::Add("x".into())), e(&["x"], 0, false));
        assert_eq!(apply(&empty, &QueueOp::PlayFrom(0)), Plan::Noop);
        assert_eq!(apply(&empty, &QueueOp::Remove(0)), Plan::Noop);
    }

    #[test]
    fn missing_index_counts_as_zero() {
        assert_eq!(apply(&q(&["a", "b"], None), &QueueOp::PlayNext("x".into())), e(&["a", "x", "b"], 0, true));
    }

    #[test]
    fn remove_current_middle_of_three_end() {
        assert_eq!(apply(&q(&["a", "b", "c"], Some(0)), &QueueOp::Remove(0)), e(&["b", "c"], 0, false));
    }
}
