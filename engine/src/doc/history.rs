use super::*;
use loro::{Frontiers, ID};

/// The message of the change that `Restore` makes.
const RESTORED: &str = "restore";

/// The document as one change of its history left it: the change's peer and last
/// counter, when it started in Unix seconds and whether it restored an earlier one.
#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub peer: String,
    pub counter: i32,
    pub time: f64,
    pub restored: bool,
}

impl Doc {
    /// The versions of the history, the newest first.
    pub fn versions(&self) -> Vec<Version> {
        let since = self.doc.shallow_since_vv();
        let mut changes = Vec::new();
        for (&peer, &end) in self.doc.oplog_vv().iter() {
            let mut c = since.get(&peer).copied().unwrap_or(0);
            while c < end {
                let Some(m) = self.doc.get_change(ID::new(peer, c)) else {
                    break;
                };
                c = m.id.counter + m.len as i32;
                changes.push(m);
            }
        }
        changes.sort_by_key(|m| std::cmp::Reverse(m.lamport));
        changes
            .into_iter()
            .map(|m| Version {
                peer: m.id.peer.to_string(),
                counter: m.id.counter + m.len as i32 - 1,
                time: m.timestamp as f64,
                restored: m.message.as_deref() == Some(RESTORED),
            })
            .collect()
    }

    /// Makes the document what it was at the version `peer`, `counter` in a change of
    /// its own, which undo takes back.
    pub(super) fn restore(&self, peer: String, counter: i32) -> Res<Vec<String>> {
        let id = ID::new(peer.parse().map_err(err)?, counter);
        if !self.doc.oplog_vv().includes_id(id)
            || self
                .doc
                .shallow_since_vv()
                .get(&id.peer)
                .is_some_and(|&s| id.counter < s)
        {
            return Err("no such version".into());
        }
        self.doc.revert_to(&Frontiers::from(id)).map_err(err)?;
        self.register_images()?;
        self.message.set(Some(RESTORED));
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn a_version_is_restored_in_a_change_that_undo_takes_back_and_a_saved_document_keeps_them() {
        let (mut d, p) = empty();
        create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let one = d.versions()[0].clone();
        d.doc.set_next_commit_message("apart");
        create(&mut d, &p, NewKind::Ellipse, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(page(&d).children.len(), 2);
        let mut d = Doc::load(&d.save()).unwrap();
        assert!(d.versions().contains(&one));
        let restore = |peer: &str, counter| Command::Restore {
            peer: peer.into(),
            counter,
        };
        assert!(d.apply(restore(&one.peer, one.counter + 1000)).is_err());
        assert!(d.apply(restore("x", 0)).is_err());
        let n = d.versions().len();
        d.apply(restore(&one.peer, one.counter)).unwrap();
        assert_eq!(page(&d).children.len(), 1);
        let v = d.versions();
        assert!(v.len() > n && v[0].restored && v[0].time > 0.0);
        d.apply(Command::Undo).unwrap();
        assert_eq!(page(&d).children.len(), 2);
    }
}
