//! PDF/X on top of krilla, which has no PDF/X support: an incremental update
//! adds the FOGRA51 OutputIntent, the XMP metadata and the Info dictionary, and
//! for `rgb` gives every page an sRGB transparency group, so that the page
//! blends in RGB as the canvas does.

use crate::pdf::Preset;

const PAGE: &[u8] = b" 0 obj\n<</Type/Page/";
const FOGRA51: &[u8] = include_bytes!("../icc/FOGRA51.icc");

/// `pdf` as `preset` titled `title`, made at `date` in ISO 8601 UTC, as
/// `2026-09-28T12:00:00Z`.
pub fn pdfx(mut pdf: Vec<u8>, preset: Preset, rgb: bool, title: &str, date: &str) -> Vec<u8> {
    let tail = String::from_utf8_lossy(&pdf[pdf.len().saturating_sub(1024)..]).into_owned();
    let trailer = &tail[tail.rfind("trailer").expect("krilla writes a trailer")..];
    let number = |key: &str| -> usize {
        let at = trailer.find(key).expect("krilla's trailer") + key.len();
        let digits = trailer[at..].trim_start();
        let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap();
        digits[..end].parse().unwrap()
    };
    let (size, root, prev) = (number("/Size"), number("/Root"), number("startxref"));
    let id = &trailer[trailer.find("/ID[").unwrap()..];
    let id = &id[..=id.find(']').unwrap()];
    let did = id[5..].split(')').next().unwrap();

    let catalog = dict(&pdf, root);
    let pages: Vec<usize> = pdf
        .windows(PAGE.len())
        .enumerate()
        .filter(|(_, w)| *w == PAGE)
        .map(|(at, _)| {
            let start = pdf[..at].iter().rposition(|&b| b == b'\n').unwrap() + 1;
            std::str::from_utf8(&pdf[start..at])
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect();

    let pdf_date = format!(
        "D:{}Z00'00'",
        date.chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
    );
    let (version, xmp_version) = match preset {
        Preset::X1a => (
            "/GTS_PDFXVersion(PDF/X-1:2003)/GTS_PDFXConformance(PDF/X-1a:2003)",
            "<pdfx:GTS_PDFXVersion>PDF/X-1:2003</pdfx:GTS_PDFXVersion>\
             <pdfx:GTS_PDFXConformance>PDF/X-1a:2003</pdfx:GTS_PDFXConformance>",
        ),
        _ => (
            "/GTS_PDFXVersion(PDF/X-4)",
            "<pdfxid:GTS_PDFXVersion>PDF/X-4</pdfxid:GTS_PDFXVersion>",
        ),
    };
    let utf16: String = title.encode_utf16().map(|u| format!("{u:04X}")).collect();
    let xml_title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xmp = format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\" xmlns:xmpMM=\"http://ns.adobe.com/xap/1.0/mm/\" \
xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" xmlns:pdfx=\"http://ns.adobe.com/pdfx/1.3/\" \
xmlns:pdfxid=\"http://www.npes.org/pdfx/ns/id/\">\
<dc:format>application/pdf</dc:format>\
<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{xml_title}</rdf:li></rdf:Alt></dc:title>\
<xmp:CreateDate>{date}</xmp:CreateDate><xmp:ModifyDate>{date}</xmp:ModifyDate>\
<xmp:MetadataDate>{date}</xmp:MetadataDate><xmp:CreatorTool>Satz</xmp:CreatorTool>\
<xmpMM:DocumentID>xmp.did:{did}</xmpMM:DocumentID><xmpMM:InstanceID>xmp.iid:{did}</xmpMM:InstanceID>\
<xmpMM:VersionID>1</xmpMM:VersionID><xmpMM:RenditionClass>default</xmpMM:RenditionClass>\
<pdf:Producer>Satz</pdf:Producer><pdf:Trapped>False</pdf:Trapped>{xmp_version}\
</rdf:Description></rdf:RDF></x:xmpmeta>\n<?xpacket end=\"w\"?>"
    );
    let icc = miniz_oxide::deflate::compress_to_vec_zlib(FOGRA51, 6);
    let stream = |dict: String, data: &[u8]| {
        let mut o = format!("{dict}\nstream\n").into_bytes();
        o.extend_from_slice(data);
        o.extend_from_slice(b"\nendstream");
        o
    };
    let mut objects = vec![
        (
            size,
            stream(
                format!("<</N 4/Length {}/Filter/FlateDecode>>", icc.len()),
                &icc,
            ),
        ),
        (
            size + 1,
            format!(
                "<</Type/OutputIntent/S/GTS_PDFX/OutputConditionIdentifier(FOGRA51)\
                 /OutputCondition(Offset printing, coated paper, ISO 12647-2:2013)\
                 /RegistryName(http://www.color.org)/Info(FOGRA51)/DestOutputProfile {size} 0 R>>"
            )
            .into_bytes(),
        ),
        (
            size + 2,
            stream(
                format!("<</Type/Metadata/Subtype/XML/Length {}>>", xmp.len()),
                xmp.as_bytes(),
            ),
        ),
        (
            size + 3,
            format!(
                "<</Title<FEFF{utf16}>/Creator(Satz)/Producer(Satz)/CreationDate({pdf_date})\
                 /ModDate({pdf_date})/Trapped/False{version}>>"
            )
            .into_bytes(),
        ),
    ];
    if rgb {
        let srgb = moxcms::ColorProfile::new_srgb().encode().unwrap();
        let srgb = miniz_oxide::deflate::compress_to_vec_zlib(&srgb, 6);
        objects.push((
            size + 4,
            stream(
                format!("<</N 3/Length {}/Filter/FlateDecode>>", srgb.len()),
                &srgb,
            ),
        ));
        for &n in &pages {
            let group = format!(
                "{}/Group<</Type/Group/S/Transparency/CS[/ICCBased {} 0 R]>>>>",
                dict(&pdf, n),
                size + 4
            );
            objects.push((n, group.into_bytes()));
        }
    }
    let catalog = format!(
        "{catalog}/OutputIntents[{} 0 R]/Metadata {} 0 R>>",
        size + 1,
        size + 2
    );
    objects.push((root, catalog.into_bytes()));
    if pdf.last() != Some(&b'\n') {
        pdf.push(b'\n');
    }
    let fresh = objects.iter().filter(|(n, _)| *n >= size).count();
    let mut xref = String::from("xref\n");
    for (n, body) in &objects {
        if *n == size {
            xref += &format!("{size} {fresh}\n");
        }
        if *n < size {
            xref += &format!("{n} 1\n");
        }
        xref += &format!("{:010} 00000 n \n", pdf.len());
        pdf.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let at = pdf.len();
    xref += &format!(
        "trailer\n<</Size {}/Root {root} 0 R/Info {} 0 R/Prev {prev}{id}>>\nstartxref\n{at}\n%%EOF\n",
        size + fresh,
        size + 3
    );
    pdf.extend_from_slice(xref.as_bytes());
    pdf
}

/// The dictionary of the object `n` in `pdf` without its closing `>>`.
fn dict(pdf: &[u8], n: usize) -> String {
    let start = format!("\n{n} 0 obj\n");
    let at = rfind(pdf, start.as_bytes()).expect("krilla writes the object") + start.len();
    let body = &pdf[at..];
    let body = &body[..find(body, b"endobj").unwrap()];
    String::from_utf8_lossy(&body[..rfind(body, b">>").unwrap()]).into_owned()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}
