use lopdf::{
    Document, Object,
    content::{Content, Operation},
    dictionary,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::OnceLock};

const TEMPLATE: &[u8] =
    include_bytes!("../../entwicklungsuebersicht_mathe_DRUCKVERSION_270925.pdf");
#[derive(Serialize, Deserialize)]
pub struct Catalog {
    pub template: String,
    pub source: String,
    pub sha256: String,
    pub coordinate_system: String,
    pub criteria: Vec<Criterion>,
}
#[derive(Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub domain: String,
    pub section: String,
    pub subtitle: String,
    pub level: String,
    pub label: String,
    pub symbol: String,
    pub band: String,
    pub page: u32,
    pub x: f32,
    pub y: f32,
    pub page_height: f32,
}
pub fn catalog() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../criteria.json"))
            .expect("checked-in criteria must be valid")
    })
}
fn op(name: &str, args: Vec<Object>) -> Operation {
    Operation::new(name, args)
}

// The original is immutable and embedded. Each export starts afresh. Only
// affirmative assessments get crosses; no/unassessed remain blank on paper.
pub fn render(
    name: &str,
    year: &str,
    period: &str,
    yes: &[String],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut doc = Document::load_mem(TEMPLATE)?;
    let pages = doc.get_pages();
    let selected: HashSet<&str> = yes.iter().map(String::as_str).collect();
    for (number, page_id) in &pages {
        let mut operations = vec![
            op("q", vec![]),
            op("RG", vec![0.into(), 0.into(), 0.into()]),
            op("w", vec![1.1.into()]),
        ];
        for c in catalog()
            .criteria
            .iter()
            .filter(|c| c.page == *number && selected.contains(c.id.as_str()))
        {
            let (x, y) = (c.x, c.page_height - c.y);
            let d = 2.6;
            operations.extend([
                op("m", vec![(x - d).into(), (y - d).into()]),
                op("l", vec![(x + d).into(), (y + d).into()]),
                op("m", vec![(x - d).into(), (y + d).into()]),
                op("l", vec![(x + d).into(), (y - d).into()]),
                op("S", vec![]),
            ]);
        }
        operations.push(op("Q", vec![]));
        if operations.len() > 4 {
            doc.add_page_contents(*page_id, Content { operations }.encode()?)?;
        }
    }
    // Embed a Unicode font for student names, including non-Latin scripts.
    // Type0 font uses glyph IDs and a ToUnicode CMap from a bundled TTF.
    add_cover_text(&mut doc, pages[&1], name, year, period)?;
    let mut bytes = vec![];
    doc.save_to(&mut bytes)?;
    Ok(bytes)
}

fn add_cover_text(
    doc: &mut Document,
    page: lopdf::ObjectId,
    name: &str,
    year: &str,
    period: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use lopdf::Stream;
    use std::collections::BTreeMap;
    const FONT: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");
    let face = ttf_parser::Face::parse(FONT, 0)?;
    let scale = 1000.0 / f32::from(face.units_per_em());
    let mut glyphs = BTreeMap::new();
    for ch in name.chars().chain(year.chars()).chain(period.chars()) {
        let gid = face.glyph_index(ch).unwrap_or(ttf_parser::GlyphId(0));
        glyphs.insert(gid.0, ch);
    }
    let fontfile = doc.add_object(Stream::new(
        dictionary! {"Length1"=>FONT.len() as i64},
        FONT.to_vec(),
    ));
    let bbox = face.global_bounding_box();
    let descriptor=doc.add_object(dictionary!{
        "Type"=>"FontDescriptor","FontName"=>"DejaVuSans","Flags"=>32,
        "FontBBox"=>vec![(f32::from(bbox.x_min)*scale).into(),(f32::from(bbox.y_min)*scale).into(),(f32::from(bbox.x_max)*scale).into(),(f32::from(bbox.y_max)*scale).into()],
        "ItalicAngle"=>0,"Ascent"=>f32::from(face.ascender())*scale,"Descent"=>f32::from(face.descender())*scale,
        "CapHeight"=>750,"StemV"=>80,"FontFile2"=>fontfile
    });
    let mut widths: Vec<Object> = vec![];
    for gid in glyphs.keys() {
        widths.push(i64::from(*gid).into());
        widths.push(Object::Array(vec![
            (f32::from(
                face.glyph_hor_advance(ttf_parser::GlyphId(*gid))
                    .unwrap_or(600),
            ) * scale)
                .into(),
        ]));
    }
    let cid=doc.add_object(dictionary!{
        "Type"=>"Font","Subtype"=>"CIDFontType2","BaseFont"=>"DejaVuSans",
        "CIDSystemInfo"=>dictionary!{"Registry"=>Object::string_literal("Adobe"),"Ordering"=>Object::string_literal("Identity"),"Supplement"=>0},
        "FontDescriptor"=>descriptor,"CIDToGIDMap"=>"Identity","DW"=>600,"W"=>widths
    });
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /TamaraUnicode def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n",
    );
    for chunk in glyphs.iter().collect::<Vec<_>>().chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (gid, ch) in chunk {
            let mut buf = [0u16; 2];
            let utf = ch
                .encode_utf16(&mut buf)
                .iter()
                .map(|u| format!("{u:04X}"))
                .collect::<String>();
            cmap.push_str(&format!("<{gid:04X}> <{utf}>\n"));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap CMapName currentdict /CMap defineresource pop end end");
    let unicode = doc.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    let font=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type0","BaseFont"=>"DejaVuSans","Encoding"=>"Identity-H","DescendantFonts"=>vec![Object::Reference(cid)],"ToUnicode"=>unicode});
    let mut resources = doc.get_or_create_resources(page)?.as_dict()?.clone();
    let mut fonts = match resources.get(b"Font") {
        Ok(Object::Reference(r)) => doc.get_object(*r)?.as_dict()?.clone(),
        Ok(o) => o.as_dict()?.clone(),
        Err(_) => lopdf::Dictionary::new(),
    };
    fonts.set("TamaraFont", font);
    resources.set("Font", fonts);
    doc.get_object_mut(page)?
        .as_dict_mut()?
        .set("Resources", resources);
    let mut ops = vec![
        op("q", vec![]),
        op("rg", vec![0.12.into(), 0.18.into(), 0.17.into()]),
    ];
    for (label, y, size) in [
        (name, 217.0, 14.0),
        (year, 116.0, 14.0),
        (period, 174.0, 9.0),
    ] {
        let units: f32 = label
            .chars()
            .map(|ch| {
                f32::from(
                    face.glyph_hor_advance(face.glyph_index(ch).unwrap_or(ttf_parser::GlyphId(0)))
                        .unwrap_or(600),
                )
            })
            .sum();
        let fitted = (205.0 * f32::from(face.units_per_em()) / units.max(1.0)).min(size);
        let bytes: Vec<u8> = label
            .chars()
            .flat_map(|ch| {
                face.glyph_index(ch)
                    .unwrap_or(ttf_parser::GlyphId(0))
                    .0
                    .to_be_bytes()
            })
            .collect();
        ops.extend([
            op("BT", vec![]),
            op(
                "Tf",
                vec![Object::Name(b"TamaraFont".to_vec()), fitted.into()],
            ),
            op(
                "Tm",
                vec![1.into(), 0.into(), 0.into(), 1.into(), 332.into(), y.into()],
            ),
            op("Tj", vec![Object::string_literal(bytes)]),
            op("ET", vec![]),
        ]);
    }
    ops.push(op("Q", vec![]));
    doc.add_page_contents(page, Content { operations: ops }.encode()?)?;
    doc.compress();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn mapping_matches_template() {
        assert_eq!(format!("{:x}", Sha256::digest(TEMPLATE)), catalog().sha256);
        assert!(catalog().criteria.len() > 350);
        let mut ids = HashSet::new();
        for c in &catalog().criteria {
            assert!(ids.insert(&c.id));
            assert_eq!(
                c.band,
                match c.level.as_str() {
                    "03" | "07" => "light",
                    "05" | "09" => "dark",
                    _ => "white",
                }
            );
            assert!((5..=16).contains(&c.page));
            assert!(c.x > 78.0 && c.x < 90.0 && c.y > 128.0 && c.y < 790.0);
            assert!(
                c.label.starts_with("kann ")
                    || c.label.starts_with("kennt ")
                    || c.label.starts_with("versteht ")
            );
        }
    }
    #[test]
    fn exports_preserve_pages_and_add_only_selected_marks() {
        let selected = vec![
            catalog().criteria[0].id.clone(),
            catalog().criteria.last().unwrap().id.clone(),
        ];
        let bytes = render("Mia Müller", "2026/27", "Herbst", &selected).unwrap();
        let output = Document::load_mem(&bytes).unwrap();
        let source = Document::load_mem(TEMPLATE).unwrap();
        assert_eq!(output.get_pages().len(), 20);
        for (number, page) in output.get_pages() {
            let count = output.get_page_contents(page).len();
            let original = source.get_page_contents(source.get_pages()[&number]).len();
            assert_eq!(
                count,
                original + usize::from([1, 5, 16].contains(&number)),
                "page {number}"
            );
        }
    }
}
