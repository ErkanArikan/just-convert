use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};

const FONT_NAME: &str = "f-0-0";
// The glyphless font's advance is 1/CHAR_WIDTH em, so text stretches to the recognized word width.
const CHAR_WIDTH: f64 = 2.0;

const TO_UNICODE: &str = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo\n<<\n  /Registry (Adobe)\n  /Ordering (UCS)\n  /Supplement 0\n>> def\n/CMapName /Adobe-Identify-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n1 beginbfrange\n<0000> <FFFF> <0000>\nendbfrange\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";

/// A recognized word; coordinates are pixels in the rendered page image, origin top-left.
pub struct Word {
    pub text: String,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// The words of one page and the size of the image they were recognized in.
pub struct PageWords {
    pub words: Vec<Word>,
    pub image_width: u32,
    pub image_height: u32,
    /// The page as displayed (after rotation), in points.
    pub width_pt: f64,
    pub height_pt: f64,
}

/// Parses Tesseract TSV output into words (level 5 rows with text).
pub fn words_from_tsv(tsv: &str) -> Vec<Word> {
    tsv.lines()
        .filter_map(|line| {
            let mut columns = line.splitn(12, '\t');
            let level: u32 = columns.next()?.parse().ok()?;
            if level != 5 {
                return None;
            }
            let mut numbers = columns.by_ref().skip(5).take(5);
            let left: f64 = numbers.next()?.parse().ok()?;
            let top: f64 = numbers.next()?.parse().ok()?;
            let width: f64 = numbers.next()?.parse().ok()?;
            let height: f64 = numbers.next()?.parse().ok()?;
            let confidence: f64 = numbers.next()?.parse().ok()?;
            let text = columns.next()?.trim();
            (confidence >= 0.0 && !text.is_empty() && width > 0.0 && height > 0.0).then(|| Word {
                text: text.to_string(),
                left,
                top,
                width,
                height,
            })
        })
        .collect()
}

/// A one-page PDF showing `rgb` at `dpi`, ready for a text layer.
pub fn image_document(rgb: &[u8], width: u32, height: u32, dpi: f64) -> Result<Document, String> {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode(rgb, width, height, image::ExtendedColorType::Rgb8)
        .map_err(|error| format!("The page image could not be encoded: {error}"))?;
    let width_pt = f64::from(width) * 72.0 / dpi;
    let height_pt = f64::from(height) * 72.0 / dpi;

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let image_id = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => i64::from(width),
            "Height" => i64::from(height),
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
            "Filter" => "DCTDecode",
        },
        jpeg,
    ));
    let content_id = document.add_object(Stream::new(
        dictionary! {},
        format!("q\n{width_pt:.2} 0 0 {height_pt:.2} 0 0 cm\n/Im0 Do\nQ\n").into_bytes(),
    ));
    let page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), width_pt.into(), height_pt.into()],
        "Contents" => content_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);
    Ok(document)
}

/// Overlays an invisible text layer on each page; `pages[i]` belongs to the i-th page.
pub fn add_text_layer(
    document: &mut Document,
    pages: &[PageWords],
    font_file: &[u8],
) -> Result<(), String> {
    let font_id = add_glyphless_font(document, font_file);
    let page_ids: Vec<ObjectId> = document.get_pages().into_values().collect();
    if page_ids.len() != pages.len() {
        return Err("The PDF page count changed during recognition".into());
    }
    let wrap_open = document.add_object(Stream::new(dictionary! {}, b"q\n".to_vec()));
    let wrap_close = document.add_object(Stream::new(dictionary! {}, b"Q\n".to_vec()));

    for (page_id, layer) in page_ids.into_iter().zip(pages) {
        if layer.words.is_empty() {
            continue;
        }
        let geometry = PageGeometry::of(document, page_id)?;
        let mut content = String::from("q\n");
        for word in &layer.words {
            content.push_str(&geometry.word_operators(word, layer));
        }
        content.push_str("Q\n");
        let mut stream = Stream::new(dictionary! {}, content.into_bytes());
        let _ = stream.compress();
        let text_id = document.add_object(stream);

        let mut contents: Vec<Object> = vec![wrap_open.into()];
        contents.extend(existing_contents(document, page_id));
        contents.push(wrap_close.into());
        contents.push(text_id.into());

        let mut resources = match inherited(document, page_id, b"Resources") {
            Some(Object::Dictionary(dictionary)) => dictionary,
            _ => Dictionary::new(),
        };
        let mut fonts = match resources.get(b"Font") {
            Ok(Object::Dictionary(fonts)) => fonts.clone(),
            Ok(Object::Reference(id)) => document.get_dictionary(*id).cloned().unwrap_or_default(),
            _ => Dictionary::new(),
        };
        fonts.set(FONT_NAME, font_id);
        resources.set("Font", Object::Dictionary(fonts));

        let page = document
            .get_dictionary_mut(page_id)
            .map_err(|error| format!("A PDF page could not be edited: {error}"))?;
        page.set("Contents", Object::Array(contents));
        page.set("Resources", Object::Dictionary(resources));
    }
    Ok(())
}

fn add_glyphless_font(document: &mut Document, font_file: &[u8]) -> ObjectId {
    let mut map = Vec::with_capacity(2 * (1 << 16));
    for _ in 0..(1 << 16) {
        map.extend([0u8, 1u8]);
    }
    let mut map_stream = Stream::new(dictionary! {}, map);
    let _ = map_stream.compress();
    let map_id = document.add_object(map_stream);
    let to_unicode_id =
        document.add_object(Stream::new(dictionary! {}, TO_UNICODE.as_bytes().to_vec()));
    let file_id = document.add_object(Stream::new(
        dictionary! { "Length1" => font_file.len() as i64 },
        font_file.to_vec(),
    ));
    let width = (1000.0 / CHAR_WIDTH) as i64;
    let descriptor_id = document.add_object(dictionary! {
        "Type" => "FontDescriptor",
        "FontName" => "GlyphLessFont",
        "Flags" => 5,
        "FontBBox" => vec![0.into(), 0.into(), width.into(), 1000.into()],
        "ItalicAngle" => 0,
        "Ascent" => 1000,
        "Descent" => -1,
        "CapHeight" => 1000,
        "StemV" => 80,
        "FontFile2" => file_id,
    });
    let cid_font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "CIDFontType2",
        "BaseFont" => "GlyphLessFont",
        "CIDToGIDMap" => map_id,
        "CIDSystemInfo" => dictionary! {
            "Registry" => Object::string_literal("Adobe"),
            "Ordering" => Object::string_literal("Identity"),
            "Supplement" => 0,
        },
        "FontDescriptor" => descriptor_id,
        "DW" => width,
    });
    document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type0",
        "BaseFont" => "GlyphLessFont",
        "Encoding" => "Identity-H",
        "DescendantFonts" => vec![cid_font_id.into()],
        "ToUnicode" => to_unicode_id,
    })
}

struct PageGeometry {
    origin_x: f64,
    origin_y: f64,
    width: f64,
    height: f64,
    rotation: i64,
}

impl PageGeometry {
    fn of(document: &Document, page_id: ObjectId) -> Result<Self, String> {
        let boxed = inherited(document, page_id, b"CropBox")
            .or_else(|| inherited(document, page_id, b"MediaBox"))
            .and_then(|object| match object {
                Object::Array(values) => Some(values),
                _ => None,
            })
            .filter(|values| values.len() == 4)
            .ok_or("A PDF page has no size")?;
        let numbers: Vec<f64> = boxed
            .iter()
            .map(number)
            .collect::<Option<_>>()
            .ok_or("A PDF page has an invalid size")?;
        let (left, right) = (numbers[0].min(numbers[2]), numbers[0].max(numbers[2]));
        let (bottom, top) = (numbers[1].min(numbers[3]), numbers[1].max(numbers[3]));
        let rotation = inherited(document, page_id, b"Rotate")
            .as_ref()
            .and_then(number)
            .map_or(0, |value| (value as i64).rem_euclid(360));
        Ok(Self {
            origin_x: left,
            origin_y: bottom,
            width: right - left,
            height: top - bottom,
            rotation,
        })
    }

    /// Text operators placing one invisible word; `u`/`v` are displayed-page points, origin top-left.
    fn word_operators(&self, word: &Word, layer: &PageWords) -> String {
        let scale_x = layer.width_pt / f64::from(layer.image_width.max(1));
        let scale_y = layer.height_pt / f64::from(layer.image_height.max(1));
        let u = word.left * scale_x;
        let v = (word.top + word.height) * scale_y;
        let size = (word.height * scale_y).max(1.0);

        let (x, y, matrix) = match self.rotation {
            90 => (v, u, [0.0, 1.0, -1.0, 0.0]),
            180 => (self.width - u, v, [-1.0, 0.0, 0.0, -1.0]),
            270 => (self.width - v, self.height - u, [0.0, -1.0, 1.0, 0.0]),
            _ => (u, self.height - v, [1.0, 0.0, 0.0, 1.0]),
        };

        let characters: Vec<u16> = word.text.encode_utf16().collect();
        let stretch = (CHAR_WIDTH * 100.0 * word.width * scale_x
            / (size * characters.len().max(1) as f64))
            .clamp(1.0, 2000.0);
        let mut hex: String = characters
            .iter()
            .map(|unit| format!("{unit:04X}"))
            .collect();
        hex.push_str("0020");
        format!(
            "BT /{FONT_NAME} {size:.2} Tf 3 Tr {:.0} {:.0} {:.0} {:.0} {:.2} {:.2} Tm {stretch:.2} Tz [ <{hex}> ] TJ ET\n",
            matrix[0],
            matrix[1],
            matrix[2],
            matrix[3],
            self.origin_x + x,
            self.origin_y + y,
        )
    }
}

fn number(object: &Object) -> Option<f64> {
    match object {
        Object::Integer(value) => Some(*value as f64),
        Object::Real(value) => Some(f64::from(*value)),
        _ => None,
    }
}

/// Looks a page attribute up through the `Parent` chain, resolving references.
fn inherited(document: &Document, page_id: ObjectId, key: &[u8]) -> Option<Object> {
    let mut current = page_id;
    for _ in 0..64 {
        let dictionary = document.get_dictionary(current).ok()?;
        if let Ok(value) = dictionary.get(key) {
            return document
                .dereference(value)
                .ok()
                .map(|(_, object)| object.clone());
        }
        current = dictionary.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

fn existing_contents(document: &Document, page_id: ObjectId) -> Vec<Object> {
    let Ok(page) = document.get_dictionary(page_id) else {
        return Vec::new();
    };
    match page.get(b"Contents") {
        Ok(Object::Reference(id)) => match document.get_object(*id) {
            Ok(Object::Array(items)) => items.clone(),
            _ => vec![Object::Reference(*id)],
        },
        Ok(Object::Array(items)) => items.clone(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_words_are_extracted_with_their_boxes() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
                   4\t1\t1\t1\t1\t0\t10\t20\t300\t40\t-1\t\n\
                   5\t1\t1\t1\t1\t1\t10\t20\t120\t40\t96.5\tInvoice\n\
                   5\t1\t1\t1\t1\t2\t140\t20\t60\t40\t91\tçğ\n\
                   5\t1\t1\t1\t1\t3\t210\t20\t60\t40\t-1\t \n";
        let words = words_from_tsv(tsv);

        assert_eq!(words.len(), 2);
        assert_eq!(words[0].text, "Invoice");
        assert_eq!(
            (words[0].left, words[0].top, words[0].width),
            (10.0, 20.0, 120.0)
        );
        assert_eq!(words[1].text, "çğ");
    }

    #[test]
    fn words_are_placed_in_unrotated_page_space() {
        let layer = PageWords {
            words: Vec::new(),
            image_width: 1000,
            image_height: 2000,
            width_pt: 100.0,
            height_pt: 200.0,
        };
        let word = Word {
            text: "ab".into(),
            left: 100.0,
            top: 400.0,
            width: 200.0,
            height: 100.0,
        };
        let upright = PageGeometry {
            origin_x: 0.0,
            origin_y: 0.0,
            width: 100.0,
            height: 200.0,
            rotation: 0,
        };
        let operators = upright.word_operators(&word, &layer);

        // u = 10pt, baseline v = 50pt from the top, so y = 150pt; size = 10pt.
        assert!(operators.contains("10.00 Tf"), "{operators}");
        assert!(operators.contains("1 0 0 1 10.00 150.00 Tm"), "{operators}");
        assert!(operators.contains("<00610062"), "{operators}");

        let turned = PageGeometry {
            origin_x: 5.0,
            origin_y: 0.0,
            width: 200.0,
            height: 100.0,
            rotation: 90,
        };
        let operators = turned.word_operators(&word, &layer);
        assert!(operators.contains("0 1 -1 0 55.00 10.00 Tm"), "{operators}");
    }

    #[test]
    fn a_text_layer_makes_an_image_page_searchable() {
        let rgb = vec![255u8; 20 * 10 * 3];
        let mut document = image_document(&rgb, 20, 10, 72.0).unwrap();
        let layer = PageWords {
            words: vec![Word {
                text: "Hello".into(),
                left: 2.0,
                top: 2.0,
                width: 10.0,
                height: 5.0,
            }],
            image_width: 20,
            image_height: 10,
            width_pt: 20.0,
            height_pt: 10.0,
        };
        add_text_layer(&mut document, &[layer], b"font").unwrap();

        let page_id = *document.get_pages().values().next().unwrap();
        let content = document.get_page_content(page_id);
        let content = String::from_utf8_lossy(&content);
        assert!(content.contains("/Im0 Do"));
        assert!(content.contains("3 Tr"));
        assert!(content.contains("<00480065006C006C006F0020>"));
    }
}
