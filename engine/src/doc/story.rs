use super::*;

/// Where the story of a thread, from the frame `head`, flows through a frame: the
/// bytes `start..end`, which are `content`, as `placed` there relative to them, its
/// neighbours, and text left over at the end of the thread.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Flow {
    pub(super) head: TreeID,
    pub(super) story: Rc<Story>,
    pub(super) content: Rc<Story>,
    pub(super) start: usize,
    pub(super) end: usize,
    /// `start` and `end` in UTF-16.
    pub(super) units: [usize; 2],
    /// Where the text left over for the next frame starts.
    pub(super) rest: Option<usize>,
    pub(super) prev: Option<TreeID>,
    pub(super) next: Option<TreeID>,
    pub(super) overset: bool,
    pub(super) placed: Rc<text::Placed>,
}

impl Flow {
    /// Whether the layer shows the same as with `b`.
    pub(super) fn shows(&self, b: &Flow) -> bool {
        Rc::ptr_eq(&self.content, &b.content)
            && Rc::ptr_eq(&self.placed, &b.placed)
            && self.lead() == b.lead()
            && (self.head, self.prev, self.next, self.overset)
                == (b.head, b.prev, b.next, b.overset)
    }

    /// The start of the text from this frame on, as much as its name shows.
    pub(super) fn lead(&self) -> &str {
        let t = &self.story.text[self.start..];
        &t[..t.char_indices().nth(41).map_or(t.len(), |(i, _)| i)]
    }
}

/// What a change reaches.
pub(super) struct Reach {
    /// The pages and masters to lay out again.
    pub(super) roots: HashSet<TreeID>,
    /// The text layers whose flows go, and the stories to set again.
    texts: HashSet<TreeID>,
    heads: HashSet<TreeID>,
}

/// A story set through the frames of its thread, with the bounds and settings of
/// each frame and the number of fonts it was set with.
pub(super) struct Set {
    story: Rc<Story>,
    frames: Vec<(TreeID, [f32; 4], TextFrame)>,
    fonts: usize,
    out: Vec<Placing>,
}

/// The size a hugging text layer measured for its story, frame, width and start.
pub(super) struct Measured {
    story: Rc<Story>,
    frame: TextFrame,
    w: Option<f32>,
    from: usize,
    size: [f32; 2],
}

/// What of the last setting `old` of a story still holds after its text changed
/// between the bytes `p` and `old.len() - q`, which moved the text after it by
/// `bytes`; frames from `frames` on are as they were, and the first `same`. `head` holds the byte of the newline before `p` and where the
/// last `keep_next` before it ends, if the spans up to it are as they were.
struct Tail<'a> {
    old: &'a Set,
    head: Option<(usize, usize)>,
    same: usize,
    q: usize,
    bytes: isize,
    frames: usize,
}

impl<'a> Tail<'a> {
    fn of(
        old: &'a Set,
        text: &str,
        spans: &[Span],
        frames: &[(TreeID, [f32; 4], TextFrame)],
        fonts: usize,
    ) -> Option<Tail<'a>> {
        let o = &old.story;
        if old.fonts != fonts
            || old.frames.len() != frames.len()
            || spans
                .iter()
                .chain(&o.spans)
                .any(|s| s.attrs.list == text::List::Number)
        {
            return None;
        }
        let (a, b) = (o.text.as_bytes(), text.as_bytes());
        let p = a.iter().zip(b).take_while(|(x, y)| x == y).count();
        let most = a.len().min(b.len()) - p;
        let q = (a.iter().rev().zip(b.iter().rev()))
            .take(most)
            .take_while(|(x, y)| x == y)
            .count();
        let head = o.text[..p].rfind('\n').and_then(|h| {
            let h16 = utf16_of(text, h);
            let (mut end, mut kept) = (0, 0);
            for (x, y) in o.spans.iter().zip(spans) {
                if end >= h16 {
                    break;
                }
                if x.attrs != y.attrs || (x.len != y.len && end + x.len.min(y.len) < h16) {
                    return None;
                }
                if x.attrs.keep_next {
                    kept = (end + x.len).min(h16);
                }
                end += x.len;
            }
            (end >= h16).then_some(())?;
            Some((h, byte_of(&o.text, kept).ok()?))
        });
        let same = frames.iter().rev().zip(old.frames.iter().rev());
        Some(Tail {
            old,
            head,
            same: frames
                .iter()
                .zip(&old.frames)
                .take_while(|(x, y)| x == y)
                .count(),
            q,
            bytes: b.len() as isize - a.len() as isize,
            frames: frames.len() - same.take_while(|(x, y)| x == y).count(),
        })
    }

    /// The frame `j` as it was if it starts where it did and its paragraphs, with
    /// those a `keep_next` holds to them, end before the change.
    fn head(&self, j: usize, start: usize) -> Option<&Placing> {
        let (h, kept) = self.head?;
        let p = &self.old.out[j];
        let n = p.2?;
        let t = &self.old.story.text;
        (j < self.same && p.1 == start && n <= h && t[..n].rfind('\n').map_or(0, |b| b + 1) >= kept)
            .then_some(p)
    }

    /// Whether the frame `j`, which now starts at the byte `start`, sets what it set
    /// before: its paragraph and all after it are as they were.
    fn keeps(&self, j: usize, start: usize, text: &str, spans: &[Span]) -> bool {
        let (o, at) = (&self.old.story, self.old.out[j].1);
        let Some(nl) = o.text[..at].rfind('\n') else {
            return false;
        };
        let moved = |v: usize| v.wrapping_add_signed(self.bytes);
        fn tail(s: &[Span], from: usize) -> impl Iterator<Item = (usize, &Attrs)> {
            let mut end = 0;
            s.iter().filter_map(move |s| {
                end += s.len;
                (end > from).then(|| (end - from, &s.attrs))
            })
        }
        j >= self.frames
            && start == moved(at)
            && nl >= o.text.len() - self.q
            && tail(&o.spans, utf16_of(&o.text, nl)).eq(tail(spans, utf16_of(text, moved(nl))))
    }

    /// The frames from `j` on as they were, moved on with the text.
    fn from(&self, j: usize) -> impl Iterator<Item = Placing> {
        let moved = |v: usize| v.wrapping_add_signed(self.bytes);
        self.old.out[j..]
            .iter()
            .map(move |(f, start, next, placed)| {
                (*f, moved(*start), next.map(moved), placed.clone())
            })
    }
}

/// A frame of a thread, the byte where its text starts, where the text left over for
/// the next begins, and how its text is placed relative to its start.
pub(super) type Placing = (TreeID, usize, Option<usize>, Rc<text::Placed>);

/// The index of the span that holds the UTF-16 index `at`.
fn span_at(spans: &[Span], at: usize) -> usize {
    let mut end = 0;
    spans
        .iter()
        .take_while(|s| {
            end += s.len;
            end <= at
        })
        .count()
}

/// `text::set` from the byte `start`, placed relative to it and its span.
pub(super) fn set_from(
    text: &str,
    spans: &[Span],
    frame: [f32; 4],
    tf: &TextFrame,
    start: usize,
) -> (text::Placed, Option<usize>) {
    let (placed, rest) = text::set(text, spans, frame, tf, start);
    let k = span_at(spans, utf16_of(text, start));
    (placed.shifted(-(start as isize), -(k as isize)), rest)
}

/// The bytes `b` of `story`, which are the UTF-16 units `u`.
fn part(story: &Story, b: Range<usize>, u: [usize; 2]) -> Story {
    let k = span_at(&story.spans, u[0]);
    let mut start: usize = story.spans[..k].iter().map(|s| s.len).sum();
    let mut spans = Vec::new();
    for (i, s) in story.spans.iter().enumerate().skip(k) {
        if i > k && start >= u[1] {
            break;
        }
        let len = (start + s.len).min(u[1]).saturating_sub(start.max(u[0]));
        spans.push(Span {
            len,
            attrs: s.attrs.clone(),
        });
        start += s.len;
    }
    Story {
        text: story.text[b].into(),
        spans,
    }
}

/// The UTF-16 range of the paragraphs that `r` touches in `text`.
pub(super) fn paragraphs(text: &str, r: &Range<usize>) -> Range<usize> {
    let breaks: Vec<usize> = text
        .encode_utf16()
        .enumerate()
        .filter(|&(_, c)| c == '\n' as u16)
        .map(|(i, _)| i + 1)
        .collect();
    let last = if r.end > r.start { r.end - 1 } else { r.end };
    let start = breaks
        .iter()
        .rev()
        .find(|&&b| b <= r.start)
        .copied()
        .unwrap_or(0);
    let end = breaks
        .iter()
        .find(|&&b| b > last)
        .copied()
        .unwrap_or(text.encode_utf16().count());
    start..end
}

/// The byte offset in `s` of the UTF-16 `index`.
pub(super) fn byte_of(s: &str, index: usize) -> Res<usize> {
    let mut units = 0;
    for (b, c) in s.char_indices() {
        if units >= index {
            return Ok(b);
        }
        units += c.len_utf16();
    }
    if units >= index {
        Ok(s.len())
    } else {
        Err("index outside the text".into())
    }
}

/// The UTF-16 index of the byte offset `byte` in `s`.
pub(super) fn utf16_of(s: &str, byte: usize) -> usize {
    s[..byte.min(s.len())].encode_utf16().count()
}

impl Doc {
    /// Adds a font to those text can be set in and sets the text again.
    pub fn add_font(&mut self, bytes: &[u8]) -> Res<Typeface> {
        let face = text::add_font(bytes)?;
        self.changed.lock().unwrap().take();
        self.invalidate();
        self.doc.set_next_commit_origin(LAYOUT_ORIGIN);
        self.finish(vec![], false)?;
        Ok(face)
    }

    /// Removes the added font `hash`; text set in it falls back as a missing font.
    pub fn remove_font(&mut self, hash: &str) -> Res<()> {
        text::remove_font(hash);
        self.sets.take();
        self.changed.lock().unwrap().take();
        self.invalidate();
        self.doc.set_next_commit_origin(LAYOUT_ORIGIN);
        self.finish(vec![], false).map(drop)
    }

    /// The story of the text layer `n` and the lines of it set in its frame.
    pub(super) fn frame_lines(&self, n: TreeID) -> Res<(Rc<Story>, Vec<text::Line>)> {
        let flows = self.flows();
        let f = flows.get(&n).ok_or("not a text node")?;
        Ok((
            f.story.clone(),
            f.placed.shifted(f.start as isize, 0).geometry,
        ))
    }

    /// The story of the thread starting at `head` set through its frames. A story set
    /// before from the same inputs is taken as it was.
    pub(super) fn flow(&self, head: TreeID) -> Res<(Rc<Story>, Vec<Placing>)> {
        let text = self.own_text(head)?.to_string();
        let v = serde_json::to_value(self.meta(head).get_deep_value()).map_err(err)?;
        let palette = self.palette();
        let modes = self.active_modes(head);
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let spans = self.spans(head, &v, &s);
        let frames: Vec<_> = self
            .thread(head)
            .into_iter()
            .map(|f| (f, self.bounds(f).map(|v| v as f32), self.text_frame_of(f)))
            .collect();
        let fonts = text::fonts_added();
        let set = |t: &str, spans: &[Span], old: Option<&Set>| {
            let tail = old.and_then(|o| Tail::of(o, t, spans, &frames, fonts));
            let mut out = Vec::new();
            let mut from = Some(0);
            for (j, (f, frame, tf)) in frames.iter().enumerate() {
                if let Some(p) = tail.as_ref().zip(from).and_then(|(t, s)| t.head(j, s)) {
                    out.push(p.clone());
                    from = p.2;
                    continue;
                }
                if let (Some(tail), Some(start)) = (&tail, from)
                    && tail.keeps(j, start, t, spans)
                {
                    out.extend(tail.from(j));
                    break;
                }
                let start = from.unwrap_or(t.len());
                let (placed, rest) = set_from(t, spans, *frame, tf, start);
                let next = from.and(rest);
                out.push((*f, start, next, Rc::new(placed)));
                from = next;
            }
            out
        };
        if let Some(s) = self.sets.borrow().get(&head).filter(|s| {
            s.story.text == text && s.story.spans == spans && s.frames == frames && s.fonts == fonts
        }) {
            debug_assert_eq!(
                s.out,
                set(&text, &spans, None),
                "a story set from the same inputs changed"
            );
            return Ok((s.story.clone(), s.out.clone()));
        }
        let out = set(&text, &spans, self.sets.borrow().get(&head));
        debug_assert!(
            out == set(&text, &spans, None),
            "a story set on from its last setting differs from one set anew"
        );
        let story = Rc::new(Story { text, spans });
        let s = Set {
            story: story.clone(),
            frames,
            fonts,
            out: out.clone(),
        };
        self.sets.borrow_mut().insert(head, s);
        Ok((story, out))
    }

    /// The UTF-16 range of its story that the text layer `id` sets.
    pub fn frame_range(&self, id: &str) -> Res<[usize; 2]> {
        let flows = self.flows();
        Ok(flows.get(&self.node(id)?).ok_or("not a text node")?.units)
    }

    /// The frame of the thread of `n` that holds the byte `at`: the last one that
    /// gets text and starts at or before it.
    pub(super) fn holding(&self, n: TreeID, at: usize) -> Res<TreeID> {
        let flows = self.flows();
        let mut f = flows.get(&n).ok_or("not a text node")?.head;
        let mut holder = f;
        while let Some(flow) = flows.get(&f) {
            let Some(next) = flow.next.filter(|_| flow.rest.is_some()) else {
                break;
            };
            if flows[&next].start <= at {
                holder = next;
            }
            f = next;
        }
        Ok(holder)
    }

    /// The frame of the thread of the text `id` that holds the UTF-16 `index`.
    pub fn text_frame(&self, id: &str, index: usize) -> Res<String> {
        let n = self.node(id)?;
        let t = self.text(n)?.to_string();
        Ok(self.holding(n, byte_of(&t, index)?)?.to_string())
    }

    /// [x, top, bottom] in pt of a caret before the UTF-16 `index` of the text `id`,
    /// in the frame of its thread that holds it.
    pub fn caret(&self, id: &str, index: usize) -> Res<[f64; 3]> {
        let n = self.node(id)?;
        let t = self.text(n)?.to_string();
        let at = byte_of(&t, index)?;
        let (story, lines) = self.frame_lines(self.holding(n, at)?)?;
        Ok(text::caret(&story.text, &lines, at).map(f64::from))
    }

    /// The UTF-16 index of the character boundary in the frame `id` nearest (x, y).
    pub fn text_index(&self, id: &str, x: f64, y: f64) -> Res<usize> {
        let n = self.node(id)?;
        let (story, lines) = self.frame_lines(n)?;
        let at = match lines.is_empty() {
            true => self.flows()[&n].start,
            false => text::index_at(&lines, x as f32, y as f32),
        };
        Ok(utf16_of(&story.text, at))
    }

    /// UTF-16 start and end of the line of the text `id` that holds `index`.
    pub fn text_line(&self, id: &str, index: usize) -> Res<[usize; 2]> {
        let n = self.node(id)?;
        let t = self.text(n)?.to_string();
        let at = byte_of(&t, index)?;
        let (story, lines) = self.frame_lines(self.holding(n, at)?)?;
        Ok(match lines.get(text::line_of(&lines, at)) {
            Some(l) => [l.start, l.end].map(|b| utf16_of(&story.text, b)),
            None => [index; 2],
        })
    }

    /// The selection from `anchor` to `focus` in the text `id` as highlighted boxes in
    /// the frames of its thread on `page`, or a caret `caret_width` pt wide where they
    /// meet.
    pub fn text_overlay(
        &self,
        id: &str,
        anchor: usize,
        focus: usize,
        caret_width: f32,
        page: &str,
    ) -> Res<Vec<Op>> {
        let n = self.node(id)?;
        let page = self.sheet(page)?;
        let t = self.text(n)?.to_string();
        let [a, b] = [anchor.min(focus), anchor.max(focus)].map(|i| byte_of(&t, i));
        let (a, b) = (a?, b?);
        let solid = |color| Paint::Solid {
            color,
            ink: Ink::Rgb,
            overprint: false,
        };
        if a == b {
            let f = self.holding(n, a)?;
            if self.root(f) != page {
                return Ok(Vec::new());
            }
            let (story, lines) = self.frame_lines(f)?;
            let [x, top, bottom] = text::caret(&story.text, &lines, a);
            return Ok(self.turned(
                f,
                vec![Op::FillPath {
                    paint: solid([0.0, 0.0, 0.0, 1.0]),
                    path: rect(x - caret_width / 2.0, top, caret_width, bottom - top),
                }],
            ));
        }
        let mut ops = Vec::new();
        for f in self.thread(self.story(n)) {
            if self.root(f) != page {
                continue;
            }
            let (story, lines) = self.frame_lines(f)?;
            let t = &story.text;
            ops.extend(
                self.turned(
                    f,
                    lines
                        .iter()
                        .filter(|l| a <= l.end && b > l.start || a == l.start && b >= l.start)
                        .map(|l| {
                            let x0 = text::x_at(t, l, a.max(l.start));
                            let mut x1 = text::x_at(t, l, b.min(l.end));
                            if b > l.end {
                                x1 = x1.max(x0 + (l.bottom - l.top) / 4.0);
                            }
                            Op::FillPath {
                                paint: solid(SELECTION),
                                path: rect(x0, l.top, x1 - x0, l.bottom - l.top),
                            }
                        })
                        .collect(),
                ),
            );
        }
        Ok(ops)
    }

    /// `ops` drawn turned as the layer `id` is.
    fn turned(&self, id: TreeID, ops: Vec<Op>) -> Vec<Op> {
        let transform = self.turn(id).map(|v| v as f32);
        if transform == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
            return ops;
        }
        [
            vec![Op::PushTransform { transform }],
            ops,
            vec![Op::PopTransform],
        ]
        .concat()
    }

    /// The layer's own text, which is its story's unless it follows another frame.
    pub(super) fn own_text(&self, id: TreeID) -> Res<LoroText> {
        container(&self.meta(id), "text")
            .and_then(|c| c.into_text().ok())
            .ok_or_else(|| "not a text node".into())
    }

    /// The text layer that follows `id` in its thread.
    pub(super) fn next_of(&self, id: TreeID) -> Option<TreeID> {
        let v = value(&self.meta(id), NEXT)?.into_string().ok()?;
        let n = self.node(&v).ok()?;
        (self.kind(n) == Some(NodeKind::Text)).then_some(n)
    }

    /// The text layer that `id` follows in its thread.
    pub(super) fn prev_of(&self, id: TreeID) -> Option<TreeID> {
        if let Some(prevs) = &*self.prevs.borrow() {
            return prevs.get(&id).copied();
        }
        let prevs = self.links();
        let prev = prevs.get(&id).copied();
        *self.prevs.borrow_mut() = Some(Rc::new(prevs));
        prev
    }

    /// The text layer each text layer follows in its thread.
    pub(super) fn links(&self) -> HashMap<TreeID, TreeID> {
        let mut prevs = HashMap::new();
        for n in self.texts(&self.tree.roots()) {
            if let Some(x) = self.next_of(n).filter(|&x| x != n) {
                prevs.entry(x).or_insert(n);
            }
        }
        prevs
    }

    /// Forgets the links back of the threads unless the changes since the last
    /// snapshot leave every link that they touch as it was.
    pub(super) fn check_links(&self) {
        let Some(prevs) = self.prevs.borrow().clone() else {
            return;
        };
        let nodes = self.changed.lock().unwrap().clone();
        let kept = nodes.is_some_and(|nodes| {
            nodes.iter().all(|&n| {
                let shown = self.is_sheet(self.root(n));
                let next = self.next_of(n).filter(|&x| shown && x != n);
                let was: Vec<_> = prevs.iter().filter(|e| *e.1 == n).map(|e| *e.0).collect();
                was == next.into_iter().collect::<Vec<_>>()
                    && prevs
                        .get(&n)
                        .is_none_or(|&p| shown && self.next_of(p) == Some(n))
            })
        });
        if !kept {
            self.prevs.take();
        }
    }

    /// Makes the text layer `to` follow `id` in its thread, or ends the thread at `id`.
    pub(super) fn set_next(&self, id: TreeID, to: Option<TreeID>) -> Res<()> {
        self.prevs.take();
        match to {
            Some(t) => self.meta(id).insert(NEXT, t.to_string()),
            None => self.meta(id).delete(NEXT),
        }
        .map_err(err)
    }

    /// The first frame of the thread of `id`, which holds the story.
    pub(super) fn story(&self, id: TreeID) -> TreeID {
        let mut head = id;
        let mut seen = vec![id];
        while let Some(p) = self.prev_of(head).filter(|p| !seen.contains(p)) {
            seen.push(p);
            head = p;
        }
        head
    }

    /// The frames of the thread from `head` on.
    pub(super) fn thread(&self, head: TreeID) -> Vec<TreeID> {
        let mut out = vec![head];
        while let Some(n) = self
            .next_of(*out.last().unwrap())
            .filter(|n| !out.contains(n))
        {
            out.push(n);
        }
        out
    }

    /// Takes the text layers in `id`'s subtree out of their threads, which close up;
    /// a story whose first frame goes moves on to the next frame.
    pub(super) fn unlink_all(&self, id: TreeID) -> Res<()> {
        let mut all = Vec::new();
        self.walk(id, &mut all);
        for n in all
            .into_iter()
            .filter(|&n| self.kind(n) == Some(NodeKind::Text))
        {
            let next = self.next_of(n);
            match (self.prev_of(n), next) {
                (Some(p), x) => self.set_next(p, x)?,
                (None, Some(x)) => {
                    let delta = self.own_text(n)?.to_delta();
                    let t = self
                        .meta(x)
                        .insert_container("text", LoroText::new())
                        .map_err(err)?;
                    t.apply_delta(&delta).map_err(err)?;
                    let (from, to) = (self.meta(n), self.meta(x));
                    for k in STORY {
                        match value(&from, k) {
                            Some(v) => to.insert(k, v).map_err(err)?,
                            None => to.delete(k).map_err(err)?,
                        }
                    }
                    let mut b = self.bindings(x);
                    let own = b.clone();
                    b.extend(
                        self.bindings(n)
                            .into_iter()
                            .filter(|(k, _)| k == "size" || k == "font"),
                    );
                    if b != own {
                        to.insert(BINDINGS, loro(b)?).map_err(err)?;
                    }
                }
                (None, None) => {}
            }
            if next.is_some() {
                self.set_next(n, None)?;
            }
        }
        Ok(())
    }

    /// The story of the text layer `id`, held by the first frame of its thread.
    pub(super) fn text(&self, id: TreeID) -> Res<LoroText> {
        self.own_text(self.story(id))
    }

    /// Replaces the text styles for which `only` holds in `range` of `id`, or in all
    /// of it, by the values they stand for.
    pub(super) fn detach(
        &self,
        id: TreeID,
        range: Option<Range<usize>>,
        only: impl Fn(&str) -> bool,
    ) -> Res<()> {
        let t = self.text(id)?;
        let m = self.meta(id);
        let palette = self.palette();
        let modes = self.active_modes(id);
        let s = Scope {
            palette: &palette,
            modes: &modes,
        };
        let find = |id: &str| palette.text_styles.iter().find(|t| t.id == id && only(id));
        let own = value(&m, "textStyle")
            .and_then(|v| v.into_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if range.is_none()
            && let Some(st) = find(&own)
        {
            for (k, v) in st.values(&s) {
                m.insert(&k, loro(v)?).map_err(err)?;
            }
            m.insert("textStyle", "").map_err(err)?;
        }
        let r = range.clone().unwrap_or(0..t.len_utf16());
        let mut at = 0;
        for d in t.to_delta() {
            let TextDelta::Insert { insert, attributes } = d else {
                continue;
            };
            let (a, b) = (at, at + insert.encode_utf16().count());
            at = b;
            let (lo, hi) = (a.max(r.start), b.min(r.end));
            if lo >= hi {
                continue;
            }
            let marks = attributes.unwrap_or_default();
            let mine = match marks.get("textStyle") {
                Some(LoroValue::String(s)) => s.to_string(),
                _ if range.is_some() => own.clone(),
                _ => continue,
            };
            let Some(st) = find(&mine) else { continue };
            for (k, v) in st.values(&s) {
                if !marks.contains_key(&k) {
                    t.mark_utf16(lo..hi, &k, loro(v)?).map_err(err)?;
                }
            }
            t.mark_utf16(lo..hi, "textStyle", "").map_err(err)?;
        }
        Ok(())
    }

    pub(super) fn put_text_style(&self, i: Option<usize>, s: TextStyle) -> Res<()> {
        let taken = self
            .list::<TextStyle>("textStyles")
            .iter()
            .any(|o| o.id != s.id && o.name == s.name);
        if taken {
            return Err(format!("a text style named {} exists", s.name));
        }
        self.put("textStyles", i, s)
    }

    /// Sets the hugging sides of a text layer to what its text needs, from where its
    /// thread's story reaches it.
    pub(super) fn fit(&self, id: TreeID) -> Res<()> {
        let Sizing {
            horizontal,
            vertical,
        } = self.layout(id).sizing;
        if vertical != Size::Hug || self.next_of(id).is_some() {
            return Ok(());
        }
        let (story, flows) = self.flow(self.story(id))?;
        let from = flows.iter().find(|f| f.0 == id).map_or(0, |f| f.1);
        let tf = self.text_frame_of(id);
        let old = self.bounds(id);
        let [x, y, w, _] = old;
        let auto_width = horizontal == Size::Hug;
        let fw = (!auto_width).then_some(w as f32);
        let kept = self.fits.borrow().get(&id).and_then(|f| {
            (Rc::ptr_eq(&f.story, &story) && f.frame == tf && f.w == fw && f.from == from)
                .then_some(f.size)
        });
        let measure = || text::measure(&story.text, &story.spans, &tf, fw, from);
        let [nw, nh] = match kept {
            Some(size) => {
                debug_assert_eq!(
                    size,
                    measure(),
                    "a kept fit differs from one measured again"
                );
                size
            }
            None => {
                let size = measure();
                let fit = Measured {
                    story: story.clone(),
                    frame: tf.clone(),
                    w: fw,
                    from,
                    size,
                };
                self.fits.borrow_mut().insert(id, fit);
                size
            }
        };
        let h = f64::from(nh).max(tf.min_height);
        let new = [x, y, if auto_width { nw.into() } else { w }, h];
        if new != old {
            self.set_frame(id, new)?;
        }
        Ok(())
    }

    /// The flows through all text layers, kept since the last command.
    pub(super) fn flows(&self) -> Rc<HashMap<TreeID, Flow>> {
        match &*self.flows.borrow() {
            Some(f) => f.clone(),
            None => Rc::new(self.flow_all(None)),
        }
    }

    /// What the changes since the last snapshot reach, `None` for everything.
    pub(super) fn reach(&self) -> Option<Reach> {
        let nodes = self.changed.lock().unwrap().clone()?;
        let (old, _) = &*self.sheets.borrow();
        let mut roots: HashSet<TreeID> = nodes.iter().map(|&n| self.root(n)).collect();
        let mut todo = self.texts(&roots.iter().copied().collect::<Vec<_>>());
        todo.retain(|&t| self.reflows(t, &nodes, old));
        todo.extend(nodes.iter().filter(|n| old.contains_key(n)));
        let mut texts = HashSet::new();
        let mut heads = HashSet::new();
        while let Some(n) = todo.pop() {
            if !texts.insert(n) {
                continue;
            }
            let mut f = old.get(&n).map(|f| f.head);
            while let Some(x) = f {
                todo.push(x);
                f = old.get(&x).and_then(|f| f.next);
            }
            let head = self.story(n);
            if self.tree.contains(head) && self.is_sheet(self.root(head)) && heads.insert(head) {
                let thread = self.thread(head);
                roots.insert(self.root(*thread.last().unwrap()));
                todo.extend(thread);
            }
        }
        Some(Reach {
            roots,
            texts,
            heads,
        })
    }

    /// Whether the text layer `n` or a layer it is in changed, or its bounds or text
    /// frame differ from those its story was set through last.
    fn reflows(&self, n: TreeID, nodes: &HashSet<TreeID>, old: &HashMap<TreeID, Flow>) -> bool {
        let mut a = n;
        while !nodes.contains(&a) {
            let Some(TreeParentId::Node(p)) = self.tree.parent(a) else {
                let sets = self.sets.borrow();
                let was = old.get(&n).and_then(|f| sets.get(&f.head));
                return was
                    .and_then(|s| s.frames.iter().find(|f| f.0 == n))
                    .is_none_or(|(_, b, f)| {
                        *b != self.bounds(n).map(|v| v as f32) || *f != self.text_frame_of(n)
                    });
            };
            a = p;
        }
        true
    }

    pub(super) fn is_sheet(&self, r: TreeID) -> bool {
        matches!(self.kind(r), Some(NodeKind::Page | NodeKind::Master))
    }

    /// The text layers on the pages and masters `roots`.
    fn texts(&self, roots: &[TreeID]) -> Vec<TreeID> {
        let mut all = Vec::new();
        for &r in roots.iter().filter(|&&r| self.is_sheet(r)) {
            self.walk(r, &mut all);
        }
        all.retain(|&n| self.kind(n) == Some(NodeKind::Text));
        all
    }

    /// The flows through all text layers; with `reach`, those of the last snapshot
    /// that it does not reach as they were, and `reflowed` the layers it changed.
    pub(super) fn flow_all(&self, reach: Option<&Reach>) -> HashMap<TreeID, Flow> {
        let mut out = HashMap::new();
        let mut gone = HashMap::new();
        let heads = match reach {
            Some(r) => {
                self.flows.take();
                let old = std::mem::take(&mut self.sheets.borrow_mut().0);
                out = Rc::try_unwrap(old).unwrap_or_else(|o| (*o).clone());
                gone = (r.texts.iter())
                    .filter_map(|t| Some((*t, out.remove(t)?)))
                    .collect();
                self.sets
                    .borrow_mut()
                    .retain(|h, _| r.heads.contains(h) || !r.texts.contains(h));
                r.heads.clone()
            }
            None => {
                let heads: HashSet<_> = self
                    .texts(&self.tree.roots())
                    .into_iter()
                    .filter(|&n| self.prev_of(n).is_none())
                    .collect();
                self.sets.borrow_mut().retain(|h, _| heads.contains(h));
                heads
            }
        };
        let sheets = self.sheets.borrow();
        let mut reflowed = HashSet::new();
        for head in heads {
            let Ok((story, frames)) = self.flow(head) else {
                continue;
            };
            let mut at = (0, 0);
            let mut units = |b: usize| {
                at = (b, at.1 + story.text[at.0..b].encode_utf16().count());
                at.1
            };
            for (i, (f, start, next, placed)) in frames.iter().enumerate() {
                let end = next.unwrap_or(story.text.len());
                let units = [units(*start), units(end)];
                let was = gone.get(f).or_else(|| sheets.0.get(f));
                let content = match was {
                    Some(o) if Rc::ptr_eq(&o.placed, placed) => o.content.clone(),
                    _ => Rc::new(part(&story, *start..end, units)),
                };
                debug_assert_eq!(*content, part(&story, *start..end, units));
                let flow = Flow {
                    head,
                    story: story.clone(),
                    content,
                    start: *start,
                    end,
                    units,
                    rest: *next,
                    prev: i.checked_sub(1).map(|j| frames[j].0),
                    next: frames.get(i + 1).map(|n| n.0),
                    overset: i + 1 == frames.len() && next.is_some(),
                    placed: placed.clone(),
                };
                if !was.is_some_and(|w| w.shows(&flow)) {
                    reflowed.insert(*f);
                }
                out.insert(*f, flow);
            }
        }
        if reach.is_some() {
            self.reflowed.replace(Some(reflowed));
        }
        out
    }

    pub(super) fn set_text(&self, id: String, text: String) -> Res<Vec<String>> {
        self.text(self.node(&id)?)?
            .update(&text, UpdateOptions::default())
            .map_err(|e| format!("{e:?}"))?;
        Ok(vec![])
    }

    pub(super) fn edit_text(
        &self,
        id: String,
        range: [usize; 2],
        text: String,
    ) -> Res<Vec<String>> {
        let [a, b] = range;
        let t = self.text(self.node(&id)?)?;
        if !(a <= b && b <= t.len_utf16()) {
            return Err("range outside the text".into());
        }
        if b > a {
            t.delete_utf16(a, b - a).map_err(err)?;
        }
        if !text.is_empty() {
            t.insert_utf16(a, &text).map_err(err)?;
        }
        Ok(vec![])
    }

    /// The sizes and spacing of the story of `n` by `by`, for `Format` of each span.
    fn scaled(&self, n: TreeID, by: f64) -> Res<Vec<(Option<[usize; 2]>, TextProps)>> {
        let head = self.story(n);
        self.text(head)?;
        if !(by > 0.0 && by.is_finite()) {
            return Err("the scale must be positive".into());
        }
        let snap = self.snapshot();
        let story = snap.stories.get(&head.to_string()).ok_or("no story")?;
        let mut at = 0;
        let spans = story.spans.iter().map(|s| {
            at += s.len;
            let props = TextProps {
                size: Some(s.attrs.size * by),
                line_height: Some(s.attrs.line_height * by),
                paragraph_spacing: Some(s.attrs.paragraph_spacing * by),
                paragraph_indent: Some(s.attrs.paragraph_indent * by),
                indent_left: Some(s.attrs.indent_left * by),
                indent_right: Some(s.attrs.indent_right * by),
                space_before: Some(s.attrs.space_before * by),
                baseline_shift: Some(s.attrs.baseline_shift * by),
                ..TextProps::default()
            };
            props.check()?;
            Ok(((!story.text.is_empty()).then_some([at - s.len, at]), props))
        });
        spans.collect()
    }

    pub(super) fn check_scale(&self, n: TreeID, by: f64) -> Res<()> {
        self.scaled(n, by).map(|_| ())
    }

    /// Scales the sizes and spacing in pt of the story of `n` by `by`.
    pub(super) fn scale_text(&self, n: TreeID, by: f64) -> Res<()> {
        for (range, props) in self.scaled(n, by)? {
            self.format(self.story(n).to_string(), range, props)?;
        }
        Ok(())
    }

    pub(super) fn format(
        &self,
        id: String,
        range: Option<[usize; 2]>,
        props: TextProps,
    ) -> Res<Vec<String>> {
        let n = self.story(self.node(&id)?);
        let t = self.text(n)?;
        props.check()?;
        let len = t.len_utf16();
        let range = match range {
            Some([a, b]) if a <= b && b <= len => Some(a..b),
            Some(_) => return Err("range outside the text".into()),
            None => None,
        };
        if let Some(s) = props.text_style.as_deref().filter(|s| !s.is_empty()) {
            self.find::<TextStyle>("textStyles", s)?;
        }
        let serde_json::Value::Object(set) = serde_json::to_value(&props).map_err(err)? else {
            return Err("props are not a map".into());
        };
        let set: Vec<_> = set
            .into_iter()
            .filter(|(k, v)| !v.is_null() || k == "shading")
            .collect();
        let text = t.to_string();
        let m = self.meta(n);
        let styled = value(&m, "textStyle").is_some_and(|v| v.as_string().is_some_and(|s| !s.is_empty()))
            || t.to_delta().iter().any(|d| {
                matches!(d, TextDelta::Insert { attributes: Some(a), .. } if a.get("textStyle").is_some_and(|v| v.as_string().is_some_and(|s| !s.is_empty())))
            });
        for (k, v) in &set {
            let cleared: &[&str] = if k == "textStyle" { &STYLED } else { &[] };
            let r = match &range {
                Some(r) if PARAGRAPH.contains(&k.as_str()) => Some(paragraphs(&text, r)),
                r => r.clone(),
            };
            match r {
                Some(r) if r.is_empty() => {}
                Some(r) if v.is_null() => {
                    if let Some(own) = value(&m, k).filter(|v| !v.is_null()) {
                        let mut at = 0;
                        for d in t.to_delta() {
                            let TextDelta::Insert { insert, attributes } = d else {
                                continue;
                            };
                            let end = at + insert.encode_utf16().count();
                            if !attributes.is_some_and(|a| a.contains_key(k)) {
                                t.mark_utf16(at..end, k, own.clone()).map_err(err)?;
                            }
                            at = end;
                        }
                        m.insert(k, LoroValue::Null).map_err(err)?;
                    }
                    t.unmark_utf16(r, k).map_err(err)?;
                }
                Some(r) => {
                    t.mark_utf16(r.clone(), k, loro(v)?).map_err(err)?;
                    for c in cleared {
                        t.unmark_utf16(r.clone(), c).map_err(err)?;
                    }
                }
                None => {
                    if k == "fill" {
                        let c = props.fill.clone().unwrap();
                        m.insert("fills", loro(vec![Fill::solid(c)])?)
                            .map_err(err)?;
                    } else {
                        m.insert(k, loro(v)?).map_err(err)?;
                    }
                    if len > 0 && styled && STYLED.contains(&k.as_str()) {
                        t.mark_utf16(0..len, k, loro(v)?).map_err(err)?;
                        continue;
                    }
                    for c in [k.as_str()].iter().chain(cleared) {
                        if len > 0 {
                            t.unmark_utf16(0..len, c).map_err(err)?;
                        }
                    }
                }
            }
        }
        if range.is_none() {
            self.unbind(n, |p| set.iter().any(|(k, _)| k == p))?;
        }
        Ok(vec![])
    }

    pub(super) fn add_text_style(&self, name: String, props: TextProps) -> Res<Vec<String>> {
        let id = self.new_id();
        let mut s = TextStyle {
            id: id.clone(),
            name,
            attrs: Attrs::default(),
            bindings: BTreeMap::new(),
        };
        restyle(&mut s, props)?;
        self.put_text_style(None, s)?;
        Ok(vec![id])
    }

    pub(super) fn set_text_style(
        &self,
        id: String,
        name: Option<String>,
        props: TextProps,
    ) -> Res<Vec<String>> {
        let (i, mut s) = self.find::<TextStyle>("textStyles", &id)?;
        s.name = name.unwrap_or(s.name);
        restyle(&mut s, props)?;
        self.put_text_style(Some(i), s)?;
        Ok(vec![])
    }

    pub(super) fn delete_text_style(&self, id: String) -> Res<Vec<String>> {
        let (i, _) = self.find::<TextStyle>("textStyles", &id)?;
        self.each(|n, _| match self.kind(n) {
            Some(NodeKind::Text) if self.story(n) == n => self.detach(n, None, |s| s == id),
            _ => Ok(()),
        })?;
        self.doc.get_list("textStyles").delete(i, 1).map_err(err)?;
        Ok(vec![])
    }

    /// The text frames `from` and `to` if `to` can join the thread after `from`.
    pub fn threadable(&self, from: &str, to: &str) -> Res<(TreeID, TreeID)> {
        let (a, b) = (self.node(from)?, self.node(to)?);
        if self.kind(a) != Some(NodeKind::Text) || self.kind(b) != Some(NodeKind::Text) {
            return Err("only text frames thread".into());
        }
        if a == b || self.next_of(b).is_some() || self.prev_of(b).is_some() {
            return Err("the frame is threaded already".into());
        }
        let (ra, rb) = (self.root(a), self.root(b));
        if ra != rb
            && (self.kind(ra) != Some(NodeKind::Page) || self.kind(rb) != Some(NodeKind::Page))
        {
            return Err("frames thread within the pages or within one master".into());
        }
        Ok((a, b))
    }

    pub(super) fn thread_after(&self, from: String, to: String) -> Res<Vec<String>> {
        let (a, b) = self.threadable(&from, &to)?;
        let own = self.own_text(b)?;
        if own.len_utf16() > 0 {
            let story = self.text(a)?;
            let at = story.len_utf16();
            story.insert_utf16(at, "\n").map_err(err)?;
            let mut delta = vec![TextDelta::Retain {
                retain: at + 1,
                attributes: None,
            }];
            delta.extend(own.to_delta());
            story.apply_delta(&delta).map_err(err)?;
            own.delete_utf16(0, own.len_utf16()).map_err(err)?;
        }
        if let Some(c) = self.next_of(a) {
            self.set_next(b, Some(c))?;
        }
        self.set_next(a, Some(b))?;
        let fills = value(&self.meta(self.story(a)), "fills");
        if let Some(f) = fills {
            self.meta(b).insert("fills", f).map_err(err)?;
        }
        for f in [a, b] {
            let old = self.layout(f).sizing;
            let last = self.next_of(f).is_none();
            let sizing = Sizing {
                horizontal: match old.horizontal {
                    Size::Hug => Size::Fixed,
                    s => s,
                },
                vertical: match old.vertical {
                    Size::Hug if !last || old.horizontal == Size::Hug => Size::Fixed,
                    s => s,
                },
            };
            if sizing != old {
                self.meta(f).insert(SIZING, loro(sizing)?).map_err(err)?;
            }
        }
        Ok(vec![])
    }

    pub(super) fn unthread(&self, id: String) -> Res<Vec<String>> {
        let a = self.node(&id)?;
        if self.next_of(a).is_some() {
            self.set_next(a, None)?;
        }
        Ok(vec![])
    }
}

/// Sets the `STYLED` attributes of `props` on `s` and unbinds them.
fn restyle(s: &mut TextStyle, props: TextProps) -> Res<()> {
    props.check()?;
    let mut attrs = serde_json::to_value(&s.attrs).map_err(err)?;
    let serde_json::Value::Object(set) = serde_json::to_value(props).map_err(err)? else {
        return Err("props are not a map".into());
    };
    for (k, v) in set.into_iter().filter(|(_, v)| !v.is_null()) {
        if !STYLED.contains(&k.as_str()) {
            return Err(format!("a text style does not set {k}"));
        }
        s.bindings.remove(&k);
        attrs[&k] = v;
    }
    s.attrs = serde_json::from_value(attrs).map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::tests::*;

    #[test]
    fn the_kept_flows_are_those_set_afresh() {
        random_commands(2, 300, |d| {
            let kept = d.sets.take();
            let fresh = d.flow_all(None);
            d.sets.replace(kept);
            assert_eq!(*d.flows(), fresh);
        });
    }

    #[test]
    fn set_text_changes_text() {
        let mut d = Doc::sample();
        let id = page(&d).children[1].id.clone();
        d.apply(Command::SetText {
            id: id.clone(),
            text: "Hallo".into(),
        })
        .unwrap();
        assert!(
            matches!(&page(&d).children[1].kind, Kind::Text { content, .. } if content.text == "Hallo")
        );
        assert!(
            d.apply(Command::SetText {
                id: page(&d).children[0].id.clone(),
                text: String::new()
            })
            .is_err()
        );
    }

    #[test]
    fn format_marks_a_range_and_formatting_the_whole_text_replaces_the_marks() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        format(&mut d, &t, Some([0, 5]), sized(20.0)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(5, 20.0), (6, 12.0)]);
        d.apply(Command::SetText {
            id: t.clone(),
            text: "Hello, world".into(),
        })
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(6, 20.0), (6, 12.0)]);
        format(&mut d, &t, None, sized(9.0)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(12, 9.0)]);
        d.apply(Command::Undo).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(6, 20.0), (6, 12.0)]);
        assert!(format(&mut d, &t, None, sized(0.0)).is_err());
        assert!(format(&mut d, &t, Some([3, 99]), sized(9.0)).is_err());
    }

    #[test]
    fn paragraph_attributes_cover_the_whole_paragraphs_of_a_range() {
        let (mut d, _) = empty();
        let t = text(&mut d, "ab\ncd\nef");
        let centred = TextProps {
            text_align: Some(TextAlign::Center),
            paragraph_spacing: Some(6.0),
            hyphenate: Some(true),
            lang: Some(Lang::De),
            tabs: Some(vec![text::Tab {
                at: 40.0,
                align: text::TabAlign::Decimal,
                leader: ".".into(),
            }]),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([4, 4]), centred).unwrap();
        let s = spans(&d);
        let aligns: Vec<_> = s.iter().map(|s| (s.len, s.attrs.text_align)).collect();
        assert_eq!(
            aligns,
            [
                (3, TextAlign::Left),
                (3, TextAlign::Center),
                (2, TextAlign::Left)
            ]
        );
        assert_eq!(s[1].attrs.paragraph_spacing, 6.0);
        assert_eq!((s[1].attrs.hyphenate, s[1].attrs.lang), (true, Lang::De));
        assert_eq!((s[2].attrs.hyphenate, s[2].attrs.lang), (false, Lang::En));
        assert_eq!(s[1].attrs.tabs[0].leader, ".");
    }

    #[test]
    fn shading_set_on_all_text_comes_off_one_paragraph() {
        let (mut d, _) = empty();
        let t = text(&mut d, "ab\ncd");
        let shade = |c| TextProps {
            shading: Some(c),
            ..TextProps::default()
        };
        let red = Color::Rgb(0xff0000ff);
        format(&mut d, &t, None, shade(Some(red.clone()))).unwrap();
        format(&mut d, &t, Some([4, 4]), shade(None)).unwrap();
        let s: Vec<_> = spans(&d)
            .into_iter()
            .map(|s| (s.len, s.attrs.shading))
            .collect();
        assert_eq!(s, [(3, Some(red)), (2, None)]);
        format(&mut d, &t, None, shade(None)).unwrap();
        assert_eq!(spans(&d)[0].attrs.shading, None);
    }

    #[test]
    fn a_range_color_draws_its_glyphs_in_that_color_and_the_rest_in_the_fills() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi there");
        let red = TextProps {
            fill: Some(Color::Rgb(0xff0000ff)),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 2]), red).unwrap();
        let colors: Vec<_> = page_ops(&d)
            .into_iter()
            .filter_map(|op| match op {
                Op::GlyphRun {
                    paint: Paint::Solid { color, .. },
                    glyphs,
                    ..
                } => Some((glyphs.len(), color)),
                _ => None,
            })
            .collect();
        assert_eq!(
            colors,
            [(2, [1.0, 0.0, 0.0, 1.0]), (5, [0.0, 0.0, 0.0, 1.0])]
        );
    }

    #[test]
    fn a_text_style_sets_its_values_where_applied_and_follows_changes_and_variables() {
        let (mut d, p) = empty();
        let t = text(&mut d, "Hello world");
        let body = style(&mut d, "Body", 10.0);
        let styled = |id: &str| TextProps {
            text_style: Some(id.into()),
            ..TextProps::default()
        };
        format(&mut d, &t, Some([0, 5]), styled(&body)).unwrap();
        let s = spans(&d);
        assert_eq!(
            (s[0].len, s[0].attrs.size, s[0].attrs.line_height),
            (5, 10.0, 14.0)
        );
        assert_eq!(s[0].attrs.text_style, body);
        assert_eq!(s[1].attrs.text_style, "");

        d.apply(Command::SetTextStyle {
            id: body.clone(),
            name: None,
            props: TextProps {
                size: Some(11.0),
                ..TextProps::default()
            },
        })
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(5, 11.0), (6, 12.0)]);

        let (c, _) = collection(&mut d, "Type");
        let big = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Big".into(),
            })
            .unwrap()
            .remove(0);
        let v = variable(&mut d, &c, "Body size", Value::Number(8.0)).unwrap();
        set_value(&mut d, &v, &big, Value::Number(16.0)).unwrap();
        bind(&mut d, &body, "size", Some(&v)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(5, 8.0), (6, 12.0)]);
        use_mode(&mut d, &p, &c, Some(&big));
        assert_eq!(lens_and(&d, |a| a.size), [(5, 16.0), (6, 12.0)]);
        assert!(bind(&mut d, &body, "w", Some(&v)).is_err());
        assert!(bind(&mut d, &body, "dropLines", Some(&v)).is_err());

        format(&mut d, &t, Some([0, 2]), sized(30.0)).unwrap();
        let s = spans(&d);
        let got: Vec<_> = s
            .iter()
            .map(|s| {
                (
                    s.len,
                    s.attrs.size,
                    s.attrs.line_height,
                    s.attrs.text_style.clone(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (2, 30.0, 14.0, body.clone()),
                (3, 16.0, 14.0, body.clone()),
                (6, 12.0, 0.0, String::new())
            ]
        );

        d.apply(Command::DeleteTextStyle { id: body.clone() })
            .unwrap();
        let got: Vec<_> = spans(&d)
            .iter()
            .map(|s| (s.len, s.attrs.size, s.attrs.text_style.clone()))
            .collect();
        assert_eq!(
            got,
            [
                (2, 30.0, String::new()),
                (3, 16.0, String::new()),
                (6, 12.0, String::new())
            ]
        );
        assert!(d.snapshot().palette.text_styles.is_empty());
    }

    #[test]
    fn a_whole_text_style_holds_for_typed_text_and_keeps_overrides_until_reapplied() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi");
        let head = style(&mut d, "Head", 24.0);
        format(
            &mut d,
            &t,
            None,
            TextProps {
                text_style: Some(head.clone()),
                ..TextProps::default()
            },
        )
        .unwrap();
        d.apply(Command::SetText {
            id: t.clone(),
            text: String::new(),
        })
        .unwrap();
        d.apply(Command::SetText {
            id: t.clone(),
            text: "New".into(),
        })
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(3, 24.0)]);
        format(&mut d, &t, None, sized(9.0)).unwrap();
        let s = spans(&d);
        assert_eq!((s[0].attrs.size, s[0].attrs.line_height), (9.0, 14.0));
        assert_eq!(s[0].attrs.text_style, head);
        format(
            &mut d,
            &t,
            None,
            TextProps {
                text_style: Some(head.clone()),
                ..TextProps::default()
            },
        )
        .unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(3, 24.0)]);
    }

    #[test]
    fn text_size_binds_to_a_number_variable_in_pt() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi there");
        format(&mut d, &t, Some([0, 2]), sized(20.0)).unwrap();
        let (c, _) = collection(&mut d, "Type");
        let v = variable(&mut d, &c, "Size", Value::Number(18.0)).unwrap();
        bind(&mut d, &t, "size", Some(&v)).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(8, 18.0)]);
        format(&mut d, &t, None, sized(9.0)).unwrap();
        assert!(page(&d).children[0].bindings.is_empty());
    }

    #[test]
    fn a_font_variable_binds_to_a_story_and_follows_its_modes() {
        let (mut d, p) = empty();
        let mono = d.add_font(MONO).unwrap();
        let r = create(&mut d, &p, NewKind::Rect, [0.0, 0.0, 10.0, 10.0]);
        let t = text(&mut d, "Hi there");
        format(
            &mut d,
            &t,
            Some([0, 2]),
            TextProps {
                font: Some(mono.clone()),
                ..TextProps::default()
            },
        )
        .unwrap();
        let (c, _) = collection(&mut d, "Type");
        let alt = d
            .apply(Command::AddMode {
                collection: c.clone(),
                name: "Alt".into(),
            })
            .unwrap()
            .remove(0);
        let v = variable(&mut d, &c, "Face", Value::Font(None)).unwrap();
        set_value(&mut d, &v, &alt, Value::Font(Some(mono.clone()))).unwrap();
        let n = variable(&mut d, &c, "Size", Value::Number(9.0)).unwrap();
        assert!(bind(&mut d, &t, "font", Some(&n)).is_err());
        assert!(bind(&mut d, &t, "size", Some(&v)).is_err());
        assert!(bind(&mut d, &r, "font", Some(&v)).is_err());
        bind(&mut d, &t, "font", Some(&v)).unwrap();
        assert_eq!(lens_and(&d, |a| a.font.clone()), [(8, None)]);
        use_mode(&mut d, &p, &c, Some(&alt));
        assert_eq!(lens_and(&d, |a| a.font.clone()), [(8, Some(mono.clone()))]);
        format(
            &mut d,
            &t,
            None,
            TextProps {
                font: Some(mono),
                ..TextProps::default()
            },
        )
        .unwrap();
        assert!(page(&d).children[1].bindings.is_empty());
    }

    #[test]
    fn copies_keep_their_character_attributes() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi there");
        format(&mut d, &t, Some([0, 2]), sized(20.0)).unwrap();
        d.apply(Command::Duplicate { ids: vec![t] }).unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(2, 20.0), (6, 12.0)]);
    }

    #[test]
    fn a_text_layer_sets_its_text_inside_its_insets_and_rejects_bad_frames() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hi");
        let [x0, y0] = first_glyph(&d);
        set(
            &mut d,
            &t,
            Props {
                inset_left: Some(4.0),
                inset_top: Some(6.0),
                columns: Some(3),
                vertical_align: Some(VerticalAlign::Top),
                ..Props::default()
            },
        );
        let [x1, y1] = first_glyph(&d);
        assert!((x1 - x0 - 4.0).abs() < 1e-4 && (y1 - y0 - 6.0).abs() < 1e-4);
        let p = page(&d);
        let Kind::Text { frame, .. } = &p.children.last().unwrap().kind else {
            panic!("not text");
        };
        assert_eq!(frame.columns, 3);
        for bad in [
            Props {
                columns: Some(0),
                ..Props::default()
            },
            Props {
                inset_right: Some(-1.0),
                ..Props::default()
            },
            Props {
                baseline_grid: Some(-2.0),
                ..Props::default()
            },
        ] {
            assert!(
                d.apply(Command::Set {
                    id: t.clone(),
                    props: bad
                })
                .is_err()
            );
        }
    }

    #[test]
    fn a_new_text_layer_is_auto_width_and_grows_with_its_text_and_size() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [10.0, 20.0, 0.0, 0.0]);
        assert_eq!(node_sizing(&d, &t), sizing(Size::Hug, Size::Hug).unwrap());
        assert!(
            matches!(&page(&d).children[0].kind, Kind::Text { content, .. } if content.text.is_empty())
        );
        assert_eq!(page(&d).children[0].name, "Text");
        assert_eq!(frames(&d, &t)[0][..3], [10.0, 20.0, 0.0]);
        assert!(close(frames(&d, &t)[0][3], LEADING));
        set_text(&mut d, &t, "Text");
        let w1 = frames(&d, &t)[0][2];
        set_text(&mut d, &t, "Text\nText text");
        let [_, _, w2, h2] = frames(&d, &t)[0];
        assert!(w2 > w1 * 1.5 && close(h2, 2.0 * LEADING), "{w2} {h2}");
        set_text(
            &mut d,
            &t,
            "Satz sets type in the browser. The engine shapes it.",
        );
        assert_eq!(
            page(&d).children[0].name,
            "Satz sets type in the browser. The…"
        );
        set_text(&mut d, &t, "Text\nText text");
        let (c, m) = collection(&mut d, "Type");
        let v = variable(&mut d, &c, "Size", Value::Number(24.0)).unwrap();
        bind(&mut d, &t, "size", Some(&v)).unwrap();
        let [_, _, w3, h3] = frames(&d, &t)[0];
        assert!(close(w3, 2.0 * w2) && close(h3, 2.0 * h2), "{w3} {h3}");
        set_value(&mut d, &v, &m, Value::Number(6.0)).unwrap();
        let [_, _, w4, h4] = frames(&d, &t)[0];
        assert!(close(w4, w2 / 2.0) && close(h4, h2 / 2.0), "{w4} {h4}");
    }

    #[test]
    fn auto_height_text_is_at_least_its_min_height() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 100.0, 0.0]);
        set_text(&mut d, &t, "Hi");
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Fixed, Size::Hug),
                min_height: Some(80.0),
                ..Props::default()
            },
        );
        assert_eq!(frames(&d, &t)[0][3], 80.0);
    }

    #[test]
    fn resizing_the_width_makes_text_auto_height_and_the_height_makes_it_fixed() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        set_text(&mut d, &t, "Hi there Hi there");
        let auto_width = frames(&d, &t)[0];
        set_frame(&mut d, &t, [0.0, 0.0, auto_width[2] / 2.0, auto_width[3]]);
        assert_eq!(node_sizing(&d, &t), sizing(Size::Fixed, Size::Hug).unwrap());
        let [.., w, h] = frames(&d, &t)[0];
        assert!(
            close(w, auto_width[2] / 2.0) && close(h, 2.0 * LEADING),
            "{w} {h}"
        );
        set_frame(&mut d, &t, [0.0, 0.0, w, 100.0]);
        assert_eq!(
            node_sizing(&d, &t),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        assert_eq!(frames(&d, &t)[0], [0.0, 0.0, w, 100.0]);
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Hug, Size::Fixed),
                ..Props::default()
            },
        );
        assert_eq!(node_sizing(&d, &t), sizing(Size::Hug, Size::Hug).unwrap());
        assert_eq!(frames(&d, &t)[0], auto_width);
        set_frame(&mut d, &t, [0.0, 0.0, 50.0, 50.0]);
        assert_eq!(
            node_sizing(&d, &t),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Hug, Size::Hug),
                ..Props::default()
            },
        );
        set_frame(&mut d, &t, [0.0, 0.0, auto_width[2], 50.0]);
        assert_eq!(
            node_sizing(&d, &t),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
    }

    #[test]
    fn text_filling_an_auto_layout_frame_rewraps_and_the_frame_hugs_it() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 100.0, 10.0]);
        let t = create(&mut d, &f, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        set(
            &mut d,
            &f,
            Props {
                direction: Some(Direction::Vertical),
                padding_top: Some(5.0),
                padding_right: Some(5.0),
                padding_bottom: Some(5.0),
                padding_left: Some(5.0),
                sizing: sizing(Size::Fixed, Size::Hug),
                ..Props::default()
            },
        );
        set(
            &mut d,
            &t,
            Props {
                sizing: sizing(Size::Fill, Size::Hug),
                ..Props::default()
            },
        );
        set_text(&mut d, &t, "Hi there Hi there Hi there Hi there Hi there");
        let [outer, text] = frames(&d, &f)[..] else {
            panic!("one child");
        };
        assert_eq!(text[2], 90.0);
        assert!(text[3] > 2.5 * LEADING, "{text:?}");
        assert!(close(outer[3], text[3] + 10.0), "{outer:?} {text:?}");
        set_frame(&mut d, &f, [0.0, 0.0, 400.0, outer[3]]);
        let [outer, text] = frames(&d, &f)[..] else {
            panic!("one child");
        };
        assert_eq!(text[2], 390.0);
        assert!(close(text[3], LEADING), "{text:?}");
        assert!(close(outer[3], LEADING + 10.0), "{outer:?}");
    }

    #[test]
    fn edit_text_replaces_a_range_and_typed_text_takes_the_attributes_before_it() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        format(&mut d, &t, Some([0, 5]), sized(20.0)).unwrap();
        edit(&mut d, &t, [5, 5], "!").unwrap();
        assert_eq!(lens_and(&d, |a| a.size), [(6, 20.0), (6, 12.0)]);
        edit(&mut d, &t, [0, 1], "J").unwrap();
        edit(&mut d, &t, [6, 12], "").unwrap();
        assert!(
            matches!(&page(&d).children[0].kind, Kind::Text { content, .. } if content.text == "Jello!")
        );
        assert!(edit(&mut d, &t, [3, 9], "x").is_err());
        assert!(edit(&mut d, &t, [4, 3], "x").is_err());
    }

    #[test]
    fn the_engine_finds_and_draws_the_caret_and_the_selection_of_a_text() {
        let (mut d, p) = empty();
        let t = create(&mut d, &p, NewKind::Text, [10.0, 20.0, 0.0, 0.0]);
        set_text(&mut d, &t, "Hi\nHi");
        let [x0, top, bottom] = d.caret(&t, 0).unwrap();
        assert!(close(x0, 10.0) && close(top, 20.0) && close(bottom, 20.0 + LEADING));
        let [x2, ..] = d.caret(&t, 2).unwrap();
        assert!(x2 > x0);
        assert_eq!(d.text_index(&t, x2 + 0.5, 25.0).unwrap(), 2);
        assert_eq!(d.text_index(&t, 11.0, 20.0 + LEADING + 3.0).unwrap(), 3);
        assert_eq!(d.text_line(&t, 4).unwrap(), [3, 5]);
        let fills = |ops: Vec<Op>| -> Vec<Vec<f32>> {
            ops.into_iter()
                .map(|op| match op {
                    Op::FillPath { path, .. } => path,
                    op => panic!("unexpected {op:?}"),
                })
                .collect()
        };
        let caret = fills(d.text_overlay(&t, 2, 2, 0.5, &p).unwrap());
        assert_eq!(caret.len(), 1);
        assert!((caret[0][1] - x2 as f32 + 0.25).abs() < 0.01, "{caret:?}");
        assert_eq!(fills(d.text_overlay(&t, 1, 4, 0.5, &p).unwrap()).len(), 2);
        assert!(d.caret(&t, 9).is_err());
    }

    #[test]
    fn a_hugging_frame_whose_last_child_is_deleted_keeps_its_size_as_fixed() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 1.0, 1.0]);
        let r = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 30.0, 20.0]);
        auto(
            &mut d,
            &f,
            Props {
                padding_left: Some(5.0),
                sizing: sizing(Size::Hug, Size::Hug),
                ..Props::default()
            },
        );
        assert_eq!(frames(&d, &f)[0], [0.0, 0.0, 35.0, 20.0]);
        d.apply(Command::Delete { ids: vec![r] }).unwrap();
        assert_eq!(frames(&d, &f), [[0.0, 0.0, 35.0, 20.0]]);
        assert_eq!(
            node_sizing(&d, &f),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        d.apply(Command::Undo).unwrap();
        assert_eq!(node_sizing(&d, &f), sizing(Size::Hug, Size::Hug).unwrap());
    }

    #[test]
    fn a_hugging_frame_whose_layers_all_fill_it_turns_fixed_with_a_note() {
        let (mut d, p) = empty();
        let f = create(&mut d, &p, NewKind::Frame, [0.0, 0.0, 1.0, 1.0]);
        let r = create(&mut d, &f, NewKind::Rect, [0.0, 0.0, 30.0, 20.0]);
        auto(
            &mut d,
            &f,
            Props {
                sizing: sizing(Size::Hug, Size::Hug),
                ..Props::default()
            },
        );
        assert!(d.notes().is_empty());
        set(
            &mut d,
            &r,
            Props {
                sizing: sizing(Size::Fixed, Size::Fill),
                ..Props::default()
            },
        );
        assert_eq!(frames(&d, &f), [[0.0, 0.0, 30.0, 20.0]; 2]);
        assert_eq!(node_sizing(&d, &f), sizing(Size::Hug, Size::Fixed).unwrap());
        let name = d.snapshot().pages[0].children[0].name.clone();
        assert_eq!(
            d.notes(),
            [format!("Height of {name} set to fixed: its layers fill it")]
        );
        assert!(d.notes().is_empty());
    }

    #[test]
    fn a_frame_with_more_text_than_fits_is_overset() {
        let (mut d, p1) = empty();
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, 2.0 * LEADING + 1.0]);
        set_text(&mut d, &a, "Hi\nHi\nHi\nHi");
        assert!(flow(&d, &a).5);
    }

    #[test]
    fn a_frame_that_sets_none_of_its_text_is_named_after_it() {
        let (mut d, p1) = empty();
        let a = fixed_text(&mut d, &p1, [0.0, 0.0, 100.0, 1.0]);
        set_text(&mut d, &a, "Hi");
        assert_eq!(d.frame_range(&a).unwrap(), [0, 0]);
        assert_eq!(page(&d).children[0].name, "Hi");
    }

    #[test]
    fn text_flattens_into_the_outlines_of_its_glyphs_unless_threaded() {
        let (mut d, _, _, a, _) = two_pages();
        assert!(d.apply(Command::Flatten { id: a }).is_err());
        let t = text(&mut d, "Hi");
        d.apply(Command::Flatten { id: t.clone() }).unwrap();
        let p = page(&d);
        let n = p.children.iter().find(|n| n.id == t).unwrap();
        let Kind::Shape(Shape::Path { path, .. }) = &n.kind else {
            panic!("{:?}", n.kind);
        };
        assert!(geom::closed(path));
        assert!(
            n.w > 10.0 && n.w < 14.0 && n.h > 6.0 && n.h < 9.0,
            "{} {}",
            n.w,
            n.h
        );
    }

    #[test]
    fn threaded_frames_share_one_story_and_each_sets_its_part() {
        let (d, _, _, a, b) = two_pages();
        let story = "Hi\nHi\nHi\nHi".to_string();
        assert_eq!(
            flow(&d, &a),
            (story.clone(), 0, 6, None, Some(b.clone()), false)
        );
        assert_eq!(flow(&d, &b), (story, 6, 11, Some(a), None, false));
    }

    #[test]
    fn each_page_draws_the_lines_of_its_frames_of_a_thread() {
        let (d, p1, p2, _, _) = two_pages();
        let runs = |p: &str| {
            d.render(p)
                .into_iter()
                .filter_map(|o| match o {
                    Op::GlyphRun { positions, .. } => Some(positions[1]),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(runs(&p1).len(), 2);
        let on2 = runs(&p2);
        assert_eq!(on2.len(), 2);
        assert!(on2[0] > 10.0 && on2[0] < 10.0 + LEADING as f32, "{on2:?}");
    }

    #[test]
    fn text_typed_into_a_later_frame_goes_into_the_story_and_the_caret_finds_its_frame() {
        let (mut d, _, _, a, b) = two_pages();
        edit(&mut d, &b, [7, 7], "X").unwrap();
        assert_eq!(flow(&d, &a).0, "Hi\nHi\nHXi\nHi");
        assert_eq!(d.text_frame(&a, 8).unwrap(), b);
        assert_eq!(d.text_frame(&b, 2).unwrap(), a);
        assert_eq!(d.text_frame(&a, 6).unwrap(), b);
        let [_, top, _] = d.caret(&a, 8).unwrap();
        assert!(close(top, 10.0), "{top}");
        assert_eq!(d.text_index(&b, 11.0, 11.0).unwrap(), 6);
        assert_eq!(d.text_line(&a, 7).unwrap(), [6, 9]);
    }

    #[test]
    fn a_change_in_one_frame_moves_the_text_of_the_frames_after_it_on() {
        let (mut d, _, _, a, b) = two_pages();
        let at = |d: &Doc| (flow(d, &b).1, flow(d, &b).2);
        edit(&mut d, &a, [1, 1], "XY").unwrap();
        assert_eq!(at(&d), (8, 13));
        edit(&mut d, &a, [0, 1], "").unwrap();
        assert_eq!(at(&d), (7, 12));
        edit(&mut d, &a, [2, 4], "").unwrap();
        assert_eq!((at(&d), flow(&d, &a).0), ((8, 10), "XYHi\nHi\nHi".into()));
        format(&mut d, &a, Some([0, 1]), sized(20.0)).unwrap();
        assert_eq!(at(&d), (5, 10));
        edit(&mut d, &b, [9, 9], "Z").unwrap();
        assert_eq!((at(&d), flow(&d, &a).2), ((5, 11), 5));
    }

    #[test]
    fn a_layer_that_starts_to_wrap_a_frame_grows_the_auto_height_frame_its_story_ends_in() {
        let (mut d, p1, _, _, b) = two_pages();
        let auto = Props {
            sizing: sizing(Size::Fixed, Size::Hug),
            ..Props::default()
        };
        set(&mut d, &b, auto);
        let r = create(&mut d, &p1, NewKind::Rect, [0.0, 0.0, 100.0, LEADING]);
        let height = |d: &Doc| frame(&d.snapshot().pages[1].children[0])[3];
        let h = height(&d);
        set_frame(&mut d, &r, [0.0, 1.0, 100.0, LEADING]);
        assert_eq!((flow(&d, &b).1, height(&d)), (6, h));
        let jump = Props {
            wrap: Some(Wrap::Jump),
            ..Props::default()
        };
        set(&mut d, &r, jump);
        assert_eq!(flow(&d, &b).1, 3);
        assert!(height(&d) > h);
    }

    #[test]
    fn a_selection_across_threaded_frames_shows_on_each_page() {
        let (d, p1, p2, a, _) = two_pages();
        let overlay = |page: &str| d.text_overlay(&a, 1, 8, 0.5, page).unwrap().len();
        assert_eq!((overlay(&p1), overlay(&p2)), (2, 1));
    }

    #[test]
    fn the_last_frame_of_a_thread_is_overset_when_the_story_does_not_fit() {
        let (mut d, _, _, a, b) = two_pages();
        set_frame(&mut d, &b, [10.0, 10.0, 100.0, LEADING + 1.0]);
        assert!(flow(&d, &b).5);
        assert!(!flow(&d, &a).5);
    }

    #[test]
    fn unthreading_takes_the_story_back_into_the_first_frame_and_undo_threads_again() {
        let (mut d, _, _, a, b) = two_pages();
        d.apply(Command::Unthread { id: a.clone() }).unwrap();
        assert_eq!(flow(&d, &a).4, None);
        assert!(flow(&d, &a).5);
        assert_eq!(flow(&d, &b).0, "");
        assert_eq!(flow(&d, &b).3, None);
        d.apply(Command::Undo).unwrap();
        assert_eq!(flow(&d, &b).3, Some(a));
    }

    #[test]
    fn only_fixed_frames_thread_and_the_last_may_be_auto_height() {
        let (mut d, p) = empty();
        let a = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        set_text(&mut d, &a, "Hi Hi Hi Hi");
        set_frame(&mut d, &a, [0.0, 0.0, 30.0, LEADING]);
        set(
            &mut d,
            &a,
            Props {
                sizing: sizing(Size::Fixed, Size::Hug),
                ..Props::default()
            },
        );
        let b = create(&mut d, &p, NewKind::Text, [0.0, 100.0, 0.0, 0.0]);
        set_text(&mut d, &b, "Yo");
        let c = fixed_text(&mut d, &p, [0.0, 200.0, 30.0, 30.0]);
        set(
            &mut d,
            &c,
            Props {
                sizing: sizing(Size::Fixed, Size::Hug),
                ..Props::default()
            },
        );
        thread(&mut d, &a, &b).unwrap();
        assert_eq!(
            node_sizing(&d, &a),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        assert_eq!(
            node_sizing(&d, &b),
            sizing(Size::Fixed, Size::Fixed).unwrap()
        );
        assert_eq!(flow(&d, &a).0, "Hi Hi Hi Hi\nYo");
        assert!(close(frames(&d, &a)[0][3], 2.0 * LEADING));
        thread(&mut d, &b, &c).unwrap();
        assert_eq!(flow(&d, &c).1, 14);
        assert!(close(frames(&d, &c)[0][3], LEADING));
        assert_eq!(node_sizing(&d, &c), sizing(Size::Fixed, Size::Hug).unwrap());
        set_frame(&mut d, &b, [0.0, 100.0, 30.0, 1.0]);
        assert_eq!(flow(&d, &c).1, 12);
        assert!(close(frames(&d, &c)[0][3], LEADING));
        for (id, s) in [
            (&a, sizing(Size::Fixed, Size::Hug)),
            (&c, sizing(Size::Hug, Size::Hug)),
        ] {
            let bad = Command::Set {
                id: id.clone(),
                props: Props {
                    sizing: s,
                    ..Props::default()
                },
            };
            assert!(d.apply(bad).is_err());
        }
        assert!(thread(&mut d, &a, &a).is_err());
        assert!(thread(&mut d, &c, &a).is_err());
        let r = create(&mut d, &p, NewKind::Rect, [0.0; 4]);
        assert!(thread(&mut d, &c, &r).is_err());
    }

    #[test]
    fn a_frame_threaded_after_one_with_a_next_goes_in_between() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 50.0, 20.0]);
        let c = fixed_text(&mut d, &p, [0.0, 100.0, 50.0, 20.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 50.0, 20.0]);
        thread(&mut d, &a, &c).unwrap();
        thread(&mut d, &a, &b).unwrap();
        assert_eq!(flow(&d, &a).4, Some(b.clone()));
        assert_eq!(flow(&d, &b).4, Some(c.clone()));
        assert_eq!(flow(&d, &c).3, Some(b));
    }

    #[test]
    fn deleting_a_threaded_frame_keeps_its_story_in_the_rest_of_the_thread() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 100.0, LEADING + 1.0]);
        let c = fixed_text(&mut d, &p, [0.0, 100.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo\nHu");
        thread(&mut d, &a, &b).unwrap();
        thread(&mut d, &b, &c).unwrap();
        format(&mut d, &a, None, sized(10.0)).unwrap();
        d.apply(Command::Delete {
            ids: vec![b.clone()],
        })
        .unwrap();
        assert_eq!(flow(&d, &a).4, Some(c.clone()));
        assert_eq!(flow(&d, &c).1, 3);
        d.apply(Command::Delete {
            ids: vec![a.clone()],
        })
        .unwrap();
        let (text, start, _, prev, ..) = flow(&d, &c);
        assert_eq!((text.as_str(), start, prev), ("Hi\nHo\nHu", 0, None));
        assert!(spans(&d).iter().all(|s| s.attrs.size == 10.0));
        d.apply(Command::Undo).unwrap();
        d.apply(Command::Undo).unwrap();
        assert_eq!(flow(&d, &b).3, Some(a));
    }

    #[test]
    fn a_copy_of_a_threaded_frame_holds_its_whole_story_and_no_thread() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo");
        thread(&mut d, &a, &b).unwrap();
        let copy = d
            .apply(Command::Duplicate {
                ids: vec![b.clone()],
            })
            .unwrap()
            .remove(0);
        let (text, start, _, prev, next, _) = flow(&d, &copy);
        assert_eq!(
            (text.as_str(), start, prev, next),
            ("Hi\nHo", 0, None, None)
        );
        assert_eq!(flow(&d, &a).4, Some(b));
    }

    #[test]
    fn boxes_keep_a_least_size_and_paths_may_be_flat() {
        let (mut d, p) = empty();
        let least = 0.01 * MM;
        let size = |d: &Doc, id: &str| {
            let [.., w, h] = d.bounds(d.node(id).unwrap());
            [w, h]
        };
        for kind in [NewKind::Rect, NewKind::Ellipse, NewKind::Frame] {
            let r = create(&mut d, &p, kind, [0.0, 0.0, 40.0, 0.0]);
            assert_eq!(size(&d, &r), [40.0, least]);
            set_frame(&mut d, &r, [0.0, 0.0, 0.0, 20.0]);
            assert_eq!(size(&d, &r), [least, 20.0]);
        }
        let t = fixed_text(&mut d, &p, [0.0, 0.0, 40.0, 0.0]);
        assert_eq!(size(&d, &t), [40.0, least]);
        let hug = create(&mut d, &p, NewKind::Text, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(size(&d, &hug)[0], 0.0);
        let l = create(&mut d, &p, NewKind::Line, [0.0, 0.0, 40.0, 0.0]);
        assert_eq!(size(&d, &l), [40.0, 0.0]);
    }

    #[test]
    fn frames_thread_within_the_pages_or_within_one_master() {
        let (mut d, p) = empty();
        let (m1, m2) = (add_master(&mut d), add_master(&mut d));
        let on_page = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, 100.0]);
        let [a, b, c] = [&m1, &m1, &m2].map(|m| fixed_text(&mut d, m, [0.0, 0.0, 100.0, 100.0]));
        assert!(thread(&mut d, &a, &on_page).is_err());
        assert!(thread(&mut d, &on_page, &a).is_err());
        assert!(thread(&mut d, &a, &c).is_err());
        thread(&mut d, &a, &b).unwrap();
    }

    #[test]
    fn threaded_master_frames_thread_on_the_left_page_when_facing_pages_go_on() {
        let (mut d, _) = empty();
        facing(&mut d, false);
        let m = add_master(&mut d);
        let a = fixed_text(&mut d, &m, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &m, [0.0, 50.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo");
        thread(&mut d, &a, &b).unwrap();
        facing(&mut d, true);
        let copies: Vec<String> = d.snapshot().masters[0]
            .children
            .iter()
            .filter(|n| n.x < 0.0)
            .map(|n| n.id.clone())
            .collect();
        assert_eq!(flow(&d, &copies[0]).4, Some(copies[1].clone()));
        assert_eq!(flow(&d, &copies[1]).1, 3);
    }

    #[test]
    fn a_thread_from_the_left_master_page_keeps_its_story_when_facing_pages_go_off() {
        let (mut d, _) = empty();
        let m2 = add_master(&mut d);
        let left = fixed_text(&mut d, &m2, [-100.0, 0.0, 50.0, LEADING + 1.0]);
        let right = fixed_text(&mut d, &m2, [10.0, 0.0, 100.0, 100.0]);
        set_text(&mut d, &left, "Hi\nHo");
        thread(&mut d, &left, &right).unwrap();
        facing(&mut d, false);
        assert_eq!(flow(&d, &right).0, "Hi\nHo");
    }

    #[test]
    fn grouping_layers_across_the_spine_keeps_them_in_place() {
        let (mut d, _) = empty();
        let left = add_page(&mut d, None);
        let right = add_page(&mut d, None);
        let a = create(&mut d, &left, NewKind::Rect, [10.0, 0.0, 10.0, 10.0]);
        let b = create(&mut d, &right, NewKind::Rect, [10.0, 0.0, 10.0, 10.0]);
        let shift = d.snapshot().pages[1].x;
        for frame in [false, true] {
            let g = d
                .apply(Command::Group {
                    ids: vec![a.clone(), b.clone()],
                    frame,
                })
                .unwrap()
                .remove(0);
            let [x, ..] = d.bounds(d.node(&a).unwrap());
            assert_eq!(x, 10.0 + shift, "{frame}");
            assert_eq!(d.bounds(d.node(&g).unwrap())[0], 10.0 + shift);
            d.apply(Command::Undo).unwrap();
        }
    }

    #[test]
    fn copies_of_threaded_frames_thread_among_themselves() {
        let (mut d, p) = empty();
        let a = fixed_text(&mut d, &p, [0.0, 0.0, 100.0, LEADING + 1.0]);
        let b = fixed_text(&mut d, &p, [0.0, 50.0, 100.0, LEADING + 1.0]);
        let c = fixed_text(&mut d, &p, [0.0, 100.0, 100.0, 100.0]);
        set_text(&mut d, &a, "Hi\nHo\nHu");
        thread(&mut d, &a, &b).unwrap();
        thread(&mut d, &b, &c).unwrap();

        let page = d
            .apply(Command::DuplicatePage { id: p.clone() })
            .unwrap()
            .remove(0);
        let [a2, b2, c2] = d.snapshot().pages[1]
            .children
            .iter()
            .map(|n| n.id.clone())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        assert_eq!(flow(&d, &a2).4, Some(b2.clone()));
        assert_eq!(flow(&d, &b2).4, Some(c2.clone()));
        assert_eq!(flow(&d, &c2).1, 6);
        assert_eq!(flow(&d, &a).4, Some(b.clone()));
        d.apply(Command::DeletePage { id: page }).unwrap();

        let copies = d
            .apply(Command::Duplicate {
                ids: vec![b.clone(), c.clone()],
            })
            .unwrap();
        let (text, start, _, prev, next, _) = flow(&d, &copies[0]);
        assert_eq!(
            (text.as_str(), start, prev, next),
            ("Hi\nHo\nHu", 0, None, Some(copies[1].clone()))
        );
        assert_eq!(flow(&d, &copies[1]).1, 3);
        assert_eq!(flow(&d, &c).3, Some(b.clone()));

        d.apply(Command::Copy {
            ids: vec![a.clone(), b.clone()],
        })
        .unwrap();
        let pasted = d
            .apply(Command::Paste {
                above: vec![],
                page: None,
            })
            .unwrap();
        assert_eq!(flow(&d, &pasted[0]).4, Some(pasted[1].clone()));
        assert_eq!(flow(&d, &pasted[1]).1, 3);
    }

    #[test]
    fn a_font_is_added_once_by_its_name_and_hash() {
        let face = text::add_font(MONO).unwrap();
        assert_eq!(face.name, "DM Mono Regular");
        assert_eq!(face.hash.len(), 16);
        assert_eq!(text::add_font(MONO).unwrap(), face);
        assert!(text::add_font(b"not a font").is_err());
        assert!(text::add_font(b"wOF2 compressed").is_err());
        let woff2 = text::add_font(include_bytes!("../../fonts/DMMono-Regular.woff2")).unwrap();
        assert_eq!(woff2.name, face.name);
        let id = text::font_id(&Some(woff2));
        assert!(text::typeface(&text::font_bytes(id)).is_ok());
    }

    #[test]
    fn text_in_an_added_font_is_shaped_and_drawn_in_it() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        let plain = runs(&d);
        let face = text::add_font(MONO).unwrap();
        format(&mut d, &t, Some([0, 5]), in_font(face)).unwrap();
        let r = runs(&d);
        assert_eq!(r.len(), 2);
        let (font, glyphs, xy) = &r[0];
        assert!(*font > 0 && font & text::MISSING == 0);
        assert_ne!(glyphs, &plain[0].1[..5]);
        let steps: Vec<f32> = xy.chunks(2).map(|p| p[0]).collect();
        let steps: Vec<f32> = steps.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(
            steps.iter().all(|s| (s - steps[0]).abs() < 0.01),
            "{steps:?}"
        );
        assert_eq!(r[1].0, 0);
        assert!(d.snapshot().missing_fonts.is_empty());
        assert!(
            String::from_utf8_lossy(&pdf(&d))
                .matches("/FontFile")
                .count()
                >= 2
        );
    }

    #[test]
    fn a_removed_font_is_reported_missing_until_it_is_added_again() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        let face = text::add_font(MONO).unwrap();
        format(&mut d, &t, Some([0, 5]), in_font(face.clone())).unwrap();
        d.remove_font(&face.hash).unwrap();
        assert_eq!(d.snapshot().missing_fonts[0].font, face);
        assert_eq!(d.snapshot().fonts.len(), 1);
        d.add_font(MONO).unwrap();
        assert!(d.snapshot().missing_fonts.is_empty());
        assert_eq!(text::fonts_added(), 3);
    }

    #[test]
    fn a_missing_font_falls_back_to_the_bundled_one_and_is_reported() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        let plain = runs(&d);
        let gone = Typeface {
            name: "Gone Sans Bold".into(),
            family: "Gone Sans".into(),
            style: "Bold".into(),
            hash: "0123456789abcdef".into(),
        };
        format(&mut d, &t, None, in_font(gone.clone())).unwrap();
        let r = runs(&d);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, text::MISSING);
        assert_eq!((&r[0].1, &r[0].2), (&plain[0].1, &plain[0].2));
        assert_eq!(
            d.snapshot().missing_fonts,
            [MissingFont {
                font: gone,
                stories: vec![t]
            }]
        );
        assert!(pdf(&d).starts_with(b"%PDF"));
    }

    #[test]
    fn a_text_style_sets_its_font_but_not_what_it_does_not_style() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        let mono = d.add_font(MONO).unwrap();
        let mut style = |props| {
            d.apply(Command::AddTextStyle {
                name: "Mono".into(),
                props,
            })
        };
        let centred = TextProps {
            text_align: Some(TextAlign::Center),
            ..TextProps::default()
        };
        assert!(style(centred).is_err());
        let id = style(in_font(mono.clone())).unwrap().remove(0);
        let styled = TextProps {
            text_style: Some(id),
            ..TextProps::default()
        };
        format(&mut d, &t, None, styled).unwrap();
        assert_eq!(spans(&d)[0].attrs.font, Some(mono));
    }

    #[test]
    fn a_missing_font_once_added_sets_its_text() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        let mono = Typeface {
            name: "DM Mono Regular".into(),
            family: "DM Mono".into(),
            style: "Regular".into(),
            hash: "c7ad9b42c84d5685".into(),
        };
        format(&mut d, &t, None, in_font(mono.clone())).unwrap();
        assert_eq!(runs(&d)[0].0, text::MISSING);
        assert_eq!(d.add_font(MONO).unwrap(), mono);
        assert!(runs(&d)[0].0 & text::MISSING == 0);
        assert!(d.snapshot().missing_fonts.is_empty());
    }

    #[test]
    fn text_set_again_in_an_added_font_is_no_change_and_no_undo_step() {
        let (mut d, _) = empty();
        let t = text(&mut d, "Hello world");
        let mono = Typeface {
            name: "DM Mono Regular".into(),
            family: "DM Mono".into(),
            style: "Regular".into(),
            hash: "c7ad9b42c84d5685".into(),
        };
        format(&mut d, &t, None, in_font(mono)).unwrap();
        let fallback = page(&d).children[0].w;
        let version = d.version();
        d.add_font(MONO).unwrap();
        assert!(page(&d).children[0].w != fallback);
        assert_eq!(d.version(), version);
        d.apply(Command::Undo).unwrap();
        assert_eq!(spans(&d)[0].attrs.font, None);
    }
}
