use crate::{Student, pdf};
use std::{
    collections::HashMap,
    io::{Cursor, Write},
};
use zip::{ZipWriter, write::SimpleFileOptions};

// Keep Unicode names, but never allow path separators, control characters,
// Windows special characters or dot-only components inside the archive.
fn filename_part(name: &str) -> String {
    let mut cleaned = String::new();
    for c in name.chars() {
        let c = if c.is_control() || "/\\:*?\"<>|".contains(c) {
            '_'
        } else {
            c
        };
        // Leave room for the period and prefix within a 255-byte filename,
        // including names written with multi-byte Unicode characters.
        if cleaned.len() + c.len_utf8() > 80 {
            break;
        }
        cleaned.push(c);
    }
    let cleaned = cleaned.trim_matches([' ', '.']);
    if cleaned.is_empty() {
        "Kind".into()
    } else {
        cleaned.into()
    }
}

pub fn render(
    students: Vec<Student>,
    year: String,
    period: String,
    mut marks: HashMap<String, Vec<String>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    // PDFs already contain compressed streams; storing them avoids redundant
    // compression. A sequence number keeps even identical names distinct.
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (index, student) in students.into_iter().enumerate() {
        let pdf = pdf::render(
            &student.name,
            &year,
            &period,
            &marks.remove(&student.id).unwrap_or_default(),
        )?;
        archive.start_file(
            format!(
                "{:03}-Mathematik-{}-{}.pdf",
                index + 1,
                filename_part(&student.name),
                filename_part(&period)
            ),
            options,
        )?;
        archive.write_all(&pdf)?;
    }
    Ok(archive.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_are_safe_and_readable() {
        assert_eq!(filename_part("Mia Müller"), "Mia Müller");
        assert_eq!(
            filename_part("../Mia\\Zeugnis:*?\"<>|"),
            "_Mia_Zeugnis_______"
        );
        assert_eq!(filename_part(" . "), "Kind");
        assert_eq!(filename_part("Mia\n"), "Mia_");
        assert!(filename_part(&"漢".repeat(120)).len() <= 80);
    }
}
