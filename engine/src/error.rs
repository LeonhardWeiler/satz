use serde::Serialize;

pub type Res<T> = Result<T, Error>;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename = "EngineError"))]
#[derive(Debug, PartialEq, Serialize, thiserror::Error)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Error {
    #[error("{message}")]
    Internal { message: String },
    #[error("no {what} {id}")]
    NotFound { what: &'static str, id: String },
    #[error("{what} must be from {} to {}", round(*lo), round(*hi))]
    OutOfRange {
        what: &'static str,
        lo: f64,
        hi: f64,
    },
    #[error("no parent")]
    NoParent,
    #[error("no page")]
    NoPage,
    #[error("no entry")]
    NoEntry,
    #[error("no story")]
    NoStory,
    #[error("no such version")]
    NoVersion,
    #[error("no such layer")]
    NoLayer,
    #[error("{id} is not a layer")]
    NotALayer { id: String },
    #[error("not a container")]
    NotAContainer,
    #[error("cannot move a layer into itself")]
    IntoItself,
    #[error("nothing to group")]
    NothingToGroup,
    #[error("nothing to mask")]
    NothingToMask,
    #[error("not a Satz document")]
    NotSatz,
    #[error("a version is previewed")]
    Previewing,
    #[error("props are not a map")]
    PropsNotAMap,
    #[error("numbers must be finite")]
    NotFinite,
    #[error("stroke sides take 4 weights or none")]
    StrokeSides,
    #[error("corners take 4 radii or none")]
    Corners,
    #[error("paint transforms must be finite")]
    PaintTransform,
    #[error("a gradient needs 2 stops or more")]
    GradientStops,
    #[error("strokes cannot be images")]
    ImageStroke,
    #[error("text cannot be filled with an image")]
    ImageTextFill,
    #[error("no image fill")]
    NoImageFill,
    #[error("an image fits only into a box")]
    FitIntoBox,
    #[error("an image cannot be cropped to nothing")]
    CropToNothing,
    #[error("not a path")]
    NotAPath,
    #[error("only shapes and text flatten")]
    NotFlattenable,
    #[error("threaded text does not flatten")]
    FlattenThreaded,
    #[error("a drop cap takes up to 20 lines and 1 to 20 characters")]
    DropCap,
    #[error("a tab stop sits at 0 pt or more with a leader of up to 8 characters")]
    TabStop,
    #[error("keep 1 to 20 lines together")]
    KeepLines,
    #[error("{feature} is not an OpenType feature")]
    NotAFeature { feature: String },
    #[error("an axis has a 4 letter tag and a finite value")]
    Axis,
    #[error("a threaded text frame cannot be auto width")]
    ThreadedAutoWidth,
    #[error("only the last frame of a thread can be auto height")]
    ThreadedAutoHeight,
    #[error("not a text node")]
    NotText,
    #[error("index outside the text")]
    IndexOutsideText,
    #[error("range outside the text")]
    RangeOutsideText,
    #[error("the scale must be positive")]
    ScaleNotPositive,
    #[error("no date {y}-{m}-{d}")]
    NoDate { y: u32, m: u32, d: u32 },
    #[error("a condition is a boolean variable")]
    ConditionNotBoolean,
    #[error("a text style named {name} exists")]
    TextStyleExists { name: String },
    #[error("a text style does not set {key}")]
    NotInTextStyle { key: String },
    #[error("{pattern} is no regular expression")]
    NotARegex { pattern: String },
    #[error("a GREP style sets another text style")]
    GrepSetsStyle,
    #[error("only text frames thread")]
    ThreadNotText,
    #[error("the frame is threaded already")]
    Threaded,
    #[error("frames thread within the pages or within one master")]
    ThreadAcross,
    #[error("{id} is anchored")]
    Anchored { id: String },
    #[error("{id} is not anchored")]
    NotAnchored { id: String },
    #[error("a layer cannot be anchored in its own text")]
    AnchorInOwnText,
    #[error("no table")]
    NoTable,
    #[error("no cells")]
    NoCells,
    #[error("{id} is not in a table")]
    NotInATable { id: String },
    #[error("{id} is not a cell")]
    NotACell { id: String },
    #[error("{id} is not a cell of the table")]
    NotACellOfTable { id: String },
    #[error("index outside the table")]
    IndexOutsideTable,
    #[error("a table has at most 1000 rows and columns")]
    TableTooBig,
    #[error("a table keeps 1 row and column")]
    TableKeepsOne,
    #[error("merging takes 2 cells or more")]
    MergeOne,
    #[error("the cells do not make a rectangle")]
    NotARectangle,
    #[error("{id} is not merged")]
    NotMerged { id: String },
    #[error("as many columns and rows as the table has")]
    TableSizeCount,
    #[error("column widths are above 0 and row heights not below")]
    TableSizeValue,
    #[error("a section starts at 1 to {max}")]
    SectionStart { max: u32 },
    #[error("raster ppi must be in 72..=1200")]
    RasterPpi,
    #[error("image ppi must be 0 or in 72..=2400")]
    ImagePpi,
    #[error("JPEG quality must be in 0..=100")]
    JpegQuality,
    #[error("ink limit must be in 200..=400")]
    InkLimit,
    #[error("PDF/X-1a needs a CMYK document")]
    PdfX1aNeedsCmyk,
    #[error("guides need finite positions")]
    Guides,
    #[error("a document keeps one page")]
    LastPage,
    #[error("a page twice")]
    PageTwice,
    #[error("no free master name")]
    NoMasterName,
    #[error("a master named {name} exists")]
    MasterExists { name: String },
    #[error("the page has no master")]
    NoMaster,
    #[error("not a layer of the page's master")]
    NotOnMaster,
    #[error("already overridden")]
    Overridden,
    #[error("{id} overrides no master layer")]
    NotAnOverride { id: String },
    #[error("no area there")]
    NoArea,
    #[error("only paths have areas")]
    AreaNotPath,
    #[error("not a point")]
    NotAPoint,
    #[error("nothing to combine")]
    NothingToCombine,
    #[error("a boolean needs two shapes or more")]
    BooleanOne,
    #[error("only shapes combine")]
    BooleanNotShape,
    #[error("the shapes are on different pages")]
    BooleanPages,
    #[error("the shapes leave nothing")]
    BooleanEmpty,
    #[error("a variable cannot hold a variable")]
    VariableInVariable,
    #[error("a variable named {name} exists")]
    VariableExists { name: String },
    #[error("a variable keeps its type")]
    VariableType,
    #[error("a variable binds to a property of its type")]
    BindType,
    #[error("{prop} cannot be bound")]
    NotBindable { prop: String },
    #[error("a collection keeps one mode")]
    LastMode,
    #[error("a swatch named {name} exists")]
    SwatchExists { name: String },
    #[error("a swatch cannot use a swatch or variable")]
    SwatchOfSwatch,
    #[error("a spot color needs a CMYK alternate")]
    SpotNeedsCmyk,
    #[error("color values must be in 0..=1")]
    ColorRange,
    #[error("not an ICC profile")]
    NotIcc,
    #[error("not a CMYK profile")]
    NotCmykProfile,
    #[error("the profile does not convert colors")]
    ProfileConversion,
    #[error("not a TrueType or OpenType font")]
    NotAFont,
    #[error("the font is damaged")]
    FontDamaged,
    #[error("not a PNG or JPEG image")]
    NotAnImage,
    #[error("the image is damaged")]
    ImageDamaged,
    #[error("the image has more than {} megapixels", crate::image::MAX_PIXELS / 1_000_000)]
    TooManyPixels,
    #[error("no image {hash}")]
    NoImage { hash: String },
    #[error("image {hash} does not decode")]
    ImageNotDecoded { hash: String },
    #[error("image {hash} is not read")]
    ImageNotRead { hash: String },
    #[error("{hash} is not a linked image")]
    NotLinked { hash: String },
    #[error("a link needs a file name")]
    LinkName,
    #[error("the file of {name} is missing")]
    FileMissing { name: String },
    #[error("only SVG, PDF and Figma files import")]
    NotImportable,
    #[error("it has no pages")]
    NoPages,
    #[error("{message}")]
    NotAnSvg { message: String },
    #[error("not a PDF ({message})")]
    NotAPdf { message: String },
    #[error("not a Figma file")]
    NotFigma,
}

fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn err(e: impl std::fmt::Display) -> Error {
    Error::Internal {
        message: e.to_string(),
    }
}
