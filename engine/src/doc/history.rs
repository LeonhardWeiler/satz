use super::*;
use loro::{Frontiers, ID, IdSpan};

const RESTORED: &str = "restore";

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub peer: String,
    pub counter: i32,
    pub time: f64,
    pub restored: bool,
    pub edits: u32,
    pub page: Option<String>,
}

impl Doc {
    #[allow(clippy::disallowed_methods)]
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
        changes.sort_by_key(|m| m.lamport);
        let sheets: Vec<TreeID> = (self.tree.roots().into_iter())
            .filter(|&r| matches!(self.kind(r), Some(NodeKind::Page | NodeKind::Master)))
            .collect();
        let replay = LoroDoc::new();
        let start = self.doc.shallow_since_frontiers();
        if let Some(Ok(bytes)) =
            (!start.is_empty()).then(|| self.doc.export(ExportMode::state_only(Some(&start))))
        {
            _ = replay.import(&bytes);
        }
        let tree = replay.get_tree("nodes");
        let nodes = Arc::new(Mutex::new(HashSet::new()));
        let n = nodes.clone();
        let _sub = replay.subscribe_root(Arc::new(move |e| {
            let mut n = n.lock().unwrap();
            for d in &e.events {
                touched(d, &mut n);
            }
        }));
        let mut out: Vec<Version> = (changes.into_iter())
            .map(|m| {
                let span = IdSpan::new(m.id.peer, m.id.counter, m.id.counter + m.len as i32);
                if let Ok(bytes) = self.doc.export(ExportMode::updates_in_range(vec![span])) {
                    _ = replay.import(&bytes);
                }
                let touched: HashSet<TreeID> = (nodes.lock().unwrap().drain())
                    .map(|mut r| {
                        while let Some(TreeParentId::Node(p)) = tree.parent(r) {
                            r = p;
                        }
                        r
                    })
                    .collect();
                Version {
                    peer: m.id.peer.to_string(),
                    counter: m.id.counter + m.len as i32 - 1,
                    time: m.timestamp as f64,
                    restored: m.message.as_deref() == Some(RESTORED),
                    edits: m.len as u32,
                    page: (sheets.iter().find(|s| touched.contains(s))).map(|s| s.to_string()),
                }
            })
            .collect();
        out.reverse();
        out
    }

    fn at(&self, peer: &str, counter: i32) -> Res<Frontiers> {
        let id = ID::new(peer.parse().map_err(err)?, counter);
        if !self.doc.oplog_vv().includes_id(id)
            || self
                .doc
                .shallow_since_vv()
                .get(&id.peer)
                .is_some_and(|&s| id.counter < s)
        {
            return Err(Error::NoVersion);
        }
        Ok(Frontiers::from(id))
    }

    pub fn preview(&mut self, version: Option<(&str, i32)>) -> Res<()> {
        match version {
            Some((peer, counter)) => {
                let at = self.at(peer, counter)?;
                self.undo.pause();
                self.doc.checkout(&at).map_err(err)?;
            }
            None if self.doc.is_detached() => {
                self.doc.try_checkout_to_latest().map_err(err)?;
                self.undo.resume();
            }
            None => return Ok(()),
        }
        self.changed.lock().unwrap().take();
        self.invalidate();
        self.retain_images()
    }

    pub(super) fn restore(&self, peer: String, counter: i32) -> Res<Vec<String>> {
        self.doc.revert_to(&self.at(&peer, counter)?).map_err(err)?;
        self.retain_images()?;
        self.message.set(Some(RESTORED));
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn a_version_is_restored_in_a_change_that_undo_takes_back_and_a_saved_document_drops_them() {
        let (mut d, p) = empty();
        create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let one = d.versions()[0].clone();
        d.doc.set_next_commit_message("apart");
        create(&mut d, &p, NewKind::Ellipse, [0.0, 0.0, 10.0, 10.0]);
        assert_eq!(page(&d).children.len(), 2);
        let saved = Doc::load(&d.save()).unwrap();
        assert!(!saved.versions().contains(&one));
        assert_eq!(page(&saved).children.len(), 2);
        let restore = |peer: &str, counter| Command::Restore {
            peer: peer.into(),
            counter,
        };
        assert_eq!(
            d.apply(restore(&one.peer, one.counter + 1000)).err(),
            Some(Error::NoVersion)
        );
        assert_eq!(d.apply(restore("1", 0)).err(), Some(Error::NoVersion));
        let n = d.versions().len();
        d.apply(restore(&one.peer, one.counter)).unwrap();
        assert_eq!(page(&d).children.len(), 1);
        let v = d.versions();
        assert!(v.len() > n && v[0].restored && v[0].time > 0.0);
        d.apply(Command::Undo).unwrap();
        assert_eq!(page(&d).children.len(), 2);
    }

    #[test]
    fn a_version_names_its_page_and_is_previewed_without_touching_the_document_or_undo() {
        let (mut d, p) = empty();
        create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let one = d.versions()[0].clone();
        assert_eq!(one.page.as_deref(), Some(p.as_str()));
        d.doc.set_next_commit_message("apart");
        create(&mut d, &p, NewKind::Ellipse, [0.0, 0.0, 10.0, 10.0]);
        let (version, n) = (d.version(), d.versions().len());
        assert_eq!(
            d.preview(Some((&one.peer, one.counter + 1000))).err(),
            Some(Error::NoVersion)
        );
        d.preview(Some((&one.peer, one.counter))).unwrap();
        assert_eq!(page(&d).children.len(), 1);
        assert_eq!(d.apply(Command::Undo).err(), Some(Error::Previewing));
        d.preview(None).unwrap();
        assert_eq!(page(&d).children.len(), 2);
        assert_eq!((d.version(), d.versions().len()), (version, n));
        d.apply(Command::Undo).unwrap();
        assert_eq!(page(&d).children.len(), 1);
        let mut saved = Doc::load(&d.save()).unwrap();
        let ids = vec![page(&saved).children[0].id.clone()];
        saved.apply(Command::Delete { ids }).unwrap();
        assert_eq!(saved.versions()[0].page.as_deref(), Some(p.as_str()));
    }

    #[test]
    fn previewing_a_version_before_a_linked_image_keeps_the_image_read() {
        let (mut d, p) = empty();
        create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let one = d.versions()[0].clone();
        let png = image::tests::png(30, 20);
        let hash = d.add_image(&png, Some("a.png".into())).unwrap().hash;
        d.preview(Some((&one.peer, one.counter))).unwrap();
        d.preview(None).unwrap();
        assert!(image::info(&hash).is_some());
    }
}
