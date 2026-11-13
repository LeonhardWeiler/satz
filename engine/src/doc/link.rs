use super::*;

/// The key of a linked image's file name in its entry of the `images` map; an
/// embedded image's entry is its file.
const LINK: &str = "link";

impl Doc {
    /// Adds a PNG or JPEG file that `image::register` takes to the images of the document,
    /// for `PlaceImage`, as a link to the file `link` when given; undo does not take it
    /// out again.
    pub fn add_image(&mut self, bytes: &[u8], link: Option<String>) -> Res<ImageInfo> {
        let bytes = LoroBinaryValue::from(image::upright(bytes).unwrap_or_else(|| bytes.to_vec()));
        let info = image::register(bytes.clone())?;
        let v = match link {
            Some(name) => linked(name),
            None => LoroValue::Binary(bytes),
        };
        self.keep_image(&info.hash, v)?;
        Ok(info)
    }

    /// Adds the image `hash` as `v` unless the document has it.
    pub(super) fn keep_image(&mut self, hash: &str, v: LoroValue) -> Res<()> {
        let images = self.doc.get_map("images");
        if images.get(hash).is_none() {
            self.doc.commit();
            self.doc.set_next_commit_origin(IMAGE_ORIGIN);
            images.insert(hash, v).map_err(err)?;
            self.doc.commit();
            self.version = format!("{:?}", self.doc.oplog_frontiers());
        }
        Ok(())
    }

    /// Reads the file of a linked image for this session, which shows the layers it
    /// fills; `Command::Relink` fills those of another image with it.
    pub fn register_image(&mut self, bytes: &[u8]) -> Res<ImageInfo> {
        let bytes = image::upright(bytes).unwrap_or_else(|| bytes.to_vec());
        let info = image::register(bytes.into())?;
        self.sheets.take();
        self.invalidate();
        self.finish(vec![], false)?;
        Ok(info)
    }

    /// The file name of each linked image by its hash.
    pub(super) fn image_links(&self) -> BTreeMap<String, String> {
        let images = self.doc.get_map("images").get_value().into_map();
        images
            .unwrap_or_default()
            .iter()
            .filter_map(|(h, v)| match v {
                LoroValue::Map(m) => match m.get(LINK) {
                    Some(LoroValue::String(name)) => Some((h.clone(), name.to_string())),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    /// Fills the layers filled with the linked image `image` with the read image `to` in
    /// its place, linked to the file `name` unless the document embeds it.
    pub(super) fn relink_image(&self, image: String, to: String, name: String) -> Res<Vec<String>> {
        let images = self.doc.get_map("images");
        if !self.image_links().contains_key(&image) {
            return Err(format!("{image} is not a linked image"));
        }
        image::info(&to).ok_or(format!("image {to} is not read"))?;
        if name.is_empty() {
            return Err("a link needs a file name".into());
        }
        if !matches!(value(&images, &to), Some(LoroValue::Binary(_))) {
            images.insert(&to, linked(name)).map_err(err)?;
        }
        if image != to {
            images.delete(&image).map_err(err)?;
        }
        let mut ids = Vec::new();
        for r in self.tree.roots() {
            self.walk(r, &mut ids);
        }
        for id in ids {
            let m = self.meta(id);
            let Some(LoroValue::List(fills)) = value(&m, "fills") else {
                continue;
            };
            let mut changed = false;
            let fills: Vec<LoroValue> = fills
                .iter()
                .map(|f| match f {
                    LoroValue::Map(f) if f.get("image") == Some(&image.as_str().into()) => {
                        changed = true;
                        let mut f = (**f).clone();
                        f.insert("image".into(), to.as_str().into());
                        f.into()
                    }
                    f => f.clone(),
                })
                .collect();
            if changed {
                m.insert("fills", fills).map_err(err)?;
            }
        }
        Ok(vec![])
    }

    /// Keeps the files of the read linked images `hashes` in the document.
    pub(super) fn embed(&self, hashes: Vec<String>) -> Res<Vec<String>> {
        let links = self.image_links();
        let mut files = Vec::new();
        for h in &hashes {
            if !links.contains_key(h) {
                return Err(format!("{h} is not a linked image"));
            }
            let id = image::id(h).ok_or(format!("the file of {} is missing", links[h]))?;
            files.push(image::bytes(id));
        }
        let images = self.doc.get_map("images");
        for (h, f) in hashes.iter().zip(files) {
            images.insert(h, LoroValue::Binary(f.into())).map_err(err)?;
        }
        Ok(vec![])
    }
}

/// The entry of an image linked to the file `name`.
fn linked(name: String) -> LoroValue {
    LoroValue::from(HashMap::from([(LINK.to_string(), LoroValue::from(name))]))
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn a_linked_image_stays_out_of_the_file_and_shows_once_read_relinked_or_embedded() {
        let (mut d, p1) = empty();
        let png = image::tests::png(30, 20);
        let hash = d.add_image(&png, Some("a.png".into())).unwrap().hash;
        let id = d
            .apply(Command::PlaceImage {
                parent: p1,
                image: hash.clone(),
                name: "a.png".into(),
                x: 20.0,
                y: 20.0,
                w: 30.0,
                h: 20.0,
            })
            .unwrap()
            .remove(0);
        assert_eq!(
            d.image_links(),
            BTreeMap::from([(hash.clone(), "a.png".into())])
        );

        let saved = d.save();
        image::retain(&[]);
        let mut d = Doc::load(&saved).unwrap();
        assert_eq!(
            problems(&d),
            [(
                id.clone(),
                Problem::MissingImage {
                    image: hash.clone()
                }
            )]
        );
        assert_eq!(d.register_image(&png).unwrap().hash, hash);
        assert!(matches!(problems(&d)[..], [(_, Problem::LowPpi { .. })]));

        let other = image::tests::png(40, 20);
        let to = d.register_image(&other).unwrap().hash;
        d.apply(Command::Relink {
            image: hash.clone(),
            to: to.clone(),
            name: "b.png".into(),
        })
        .unwrap();
        let fill = |d: &Doc| page(d).children[0].style.fills[0].image.clone();
        assert_eq!(fill(&d), Some(to.clone()));
        assert_eq!(
            d.image_links(),
            BTreeMap::from([(to.clone(), "b.png".into())])
        );
        d.apply(Command::Embed {
            images: vec![to.clone()],
        })
        .unwrap();
        assert!(!d.image_links().contains_key(&to));
        let embedded = value(&d.doc.get_map("images"), &to);
        assert!(matches!(embedded, Some(LoroValue::Binary(_))));
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(fill(&d), Some(hash.clone()));
        assert_eq!(d.image_links()[&hash], "a.png");
        assert!(d.apply(Command::Embed { images: vec![hash] }).is_ok());
        assert!(
            d.apply(Command::Embed {
                images: vec!["x".into()]
            })
            .is_err()
        );
    }
}
