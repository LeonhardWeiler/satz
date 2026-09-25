use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    #[default]
    None,
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MainAlign {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Align3 {
    #[default]
    Start,
    Center,
    End,
}

/// Hug only applies to auto layout frames and text, fill only inside auto layout
/// frames. On text, hugging both sides is Figma's auto width, hugging the height
/// auto height.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Size {
    #[default]
    Fixed,
    Hug,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sizing {
    pub horizontal: Size,
    pub vertical: Size,
}

/// Figma auto layout of a frame and how a layer sits in one.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Layout {
    pub direction: Direction,
    pub gap: f64,
    pub padding_top: f64,
    pub padding_right: f64,
    pub padding_bottom: f64,
    pub padding_left: f64,
    pub align_main: MainAlign,
    pub align_cross: Align3,
    pub sizing: Sizing,
    /// Left out of its parent's auto layout.
    pub absolute: bool,
}

impl Layout {
    /// Main axis first: (horizontal, [padding before, after] per axis).
    pub fn axes(&self) -> Option<(bool, [[f64; 2]; 2])> {
        let h = [self.padding_left, self.padding_right];
        let v = [self.padding_top, self.padding_bottom];
        match self.direction {
            Direction::None => None,
            Direction::Horizontal => Some((true, [h, v])),
            Direction::Vertical => Some((false, [v, h])),
        }
    }

    pub fn size(&self, horizontal: bool) -> Size {
        if horizontal {
            self.sizing.horizontal
        } else {
            self.sizing.vertical
        }
    }
}

/// Places `children`, each [start, size] along the main and cross axis, in a frame
/// of `size` along both axes; returns the frame's size and the children's boxes.
/// `fill` and `hug` tell per axis which children fill and whether the frame hugs.
pub fn arrange(
    l: &Layout,
    pad: [[f64; 2]; 2],
    mut size: [f64; 2],
    hug: [bool; 2],
    children: &[([f64; 2], [bool; 2])],
) -> ([f64; 2], Vec<[[f64; 2]; 2]>) {
    let n = children.len() as f64;
    let fixed: f64 = children
        .iter()
        .filter(|(_, fill)| !fill[0] || hug[0])
        .map(|(s, _)| s[0])
        .sum();
    let gaps = l.gap * (n - 1.0).max(0.0);
    if hug[0] {
        size[0] = pad[0][0] + pad[0][1] + fixed + gaps;
    }
    if hug[1] {
        let widest = children
            .iter()
            .filter(|(_, fill)| !fill[1])
            .map(|(s, _)| s[1])
            .fold(0.0, f64::max);
        size[1] = pad[1][0] + pad[1][1] + widest;
    }
    let inner = [0, 1].map(|a| (size[a] - pad[a][0] - pad[a][1]).max(0.0));
    let fills = children.iter().filter(|(_, f)| f[0] && !hug[0]).count() as f64;
    let share = if fills > 0.0 {
        ((inner[0] - fixed - gaps) / fills).max(0.0)
    } else {
        0.0
    };
    let mains: Vec<f64> = children
        .iter()
        .map(|(s, f)| if f[0] && !hug[0] { share } else { s[0] })
        .collect();
    let used: f64 = mains.iter().sum();
    let free = inner[0] - used - gaps;
    let (mut at, gap) = match l.align_main {
        MainAlign::Start => (0.0, l.gap),
        MainAlign::Center => (free / 2.0, l.gap),
        MainAlign::End => (free, l.gap),
        MainAlign::SpaceBetween if n > 1.0 => (0.0, (inner[0] - used) / (n - 1.0)),
        MainAlign::SpaceBetween => (0.0, l.gap),
    };
    let boxes = children
        .iter()
        .zip(mains)
        .map(|((s, f), main)| {
            let cross = if f[1] && !hug[1] { inner[1] } else { s[1] };
            let offset = match l.align_cross {
                Align3::Start => 0.0,
                Align3::Center => (inner[1] - cross) / 2.0,
                Align3::End => inner[1] - cross,
            };
            let b = [[pad[0][0] + at, main], [pad[1][0] + offset, cross]];
            at += main + gap;
            b
        })
        .collect();
    (size, boxes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(align_main: MainAlign, align_cross: Align3) -> Layout {
        Layout {
            direction: Direction::Horizontal,
            gap: 10.0,
            align_main,
            align_cross,
            ..Layout::default()
        }
    }

    const PAD: [[f64; 2]; 2] = [[5.0, 5.0], [2.0, 2.0]];

    #[test]
    fn a_hugging_frame_fits_its_children_gaps_and_padding() {
        let l = row(MainAlign::Start, Align3::Start);
        let kids = [([20.0, 8.0], [false; 2]), ([30.0, 12.0], [false; 2])];
        let (size, boxes) = arrange(&l, PAD, [0.0; 2], [true; 2], &kids);
        assert_eq!(size, [70.0, 16.0]);
        assert_eq!(
            boxes,
            [[[5.0, 20.0], [2.0, 8.0]], [[35.0, 30.0], [2.0, 12.0]]]
        );
    }

    #[test]
    fn filling_children_share_the_free_space_and_fill_the_cross_axis() {
        let l = row(MainAlign::Start, Align3::Start);
        let kids = [
            ([20.0, 8.0], [false; 2]),
            ([0.0, 0.0], [true; 2]),
            ([0.0, 0.0], [true, false]),
        ];
        let (_, boxes) = arrange(&l, PAD, [110.0, 30.0], [false; 2], &kids);
        assert_eq!(boxes[1], [[35.0, 30.0], [2.0, 26.0]]);
        assert_eq!(boxes[2], [[75.0, 30.0], [2.0, 0.0]]);
    }

    #[test]
    fn free_space_goes_by_the_alignment_on_both_axes() {
        let kids = [([20.0, 8.0], [false; 2]), ([30.0, 12.0], [false; 2])];
        let at = |main, cross| {
            let (_, b) = arrange(&row(main, cross), PAD, [110.0, 30.0], [false; 2], &kids);
            b.iter().map(|b| [b[0][0], b[1][0]]).collect::<Vec<_>>()
        };
        assert_eq!(
            at(MainAlign::Center, Align3::Center),
            [[25.0, 11.0], [55.0, 9.0]]
        );
        assert_eq!(
            at(MainAlign::End, Align3::End),
            [[45.0, 20.0], [75.0, 16.0]]
        );
        assert_eq!(
            at(MainAlign::SpaceBetween, Align3::Start),
            [[5.0, 2.0], [75.0, 2.0]]
        );
    }

    #[test]
    fn the_main_axis_comes_first_with_its_padding() {
        let l = Layout {
            direction: Direction::Vertical,
            padding_top: 1.0,
            padding_bottom: 2.0,
            padding_left: 3.0,
            padding_right: 4.0,
            ..Layout::default()
        };
        assert_eq!(l.axes(), Some((false, [[1.0, 2.0], [3.0, 4.0]])));
        assert_eq!(Layout::default().axes(), None);
    }
}
