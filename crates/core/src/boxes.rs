use lopdf::Document;
use std::collections::BTreeMap;

use crate::error::Error;

/// Facteur de conversion canonique : 1 pouce = 25,4 mm = 72 points PDF.
pub const MM_TO_PT: f64 = 72.0 / 25.4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub llx: f64,
    pub lly: f64,
    pub urx: f64,
    pub ury: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.urx - self.llx
    }

    pub fn height(&self) -> f64 {
        self.ury - self.lly
    }

    pub fn from_array(values: &[f32]) -> Option<Self> {
        if values.len() != 4 {
            return None;
        }
        Some(Self {
            llx: f64::from(values[0]),
            lly: f64::from(values[1]),
            urx: f64::from(values[2]),
            ury: f64::from(values[3]),
        })
    }

    pub fn to_array(&self) -> [f32; 4] {
        [
            self.llx as f32,
            self.lly as f32,
            self.urx as f32,
            self.ury as f32,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct PageBoxes {
    pub page: u32,
    pub media: Option<Rect>,
    pub crop: Option<Rect>,
    pub trim: Option<Rect>,
    pub bleed: Option<Rect>,
    pub art: Option<Rect>,
}

pub fn mm_to_pt(mm: f64) -> f64 {
    mm * MM_TO_PT
}

pub fn pt_to_mm(pt: f64) -> f64 {
    pt / MM_TO_PT
}

/// Lit les boîtes d'un document PDF, avec héritage via le nœud Pages parent.
pub fn read_page_boxes(document: &Document) -> Result<Vec<PageBoxes>, Error> {
    let pages = document.get_pages();
    let mut result = Vec::with_capacity(pages.len());
    let mut ordered: BTreeMap<u32, lopdf::ObjectId> = pages.into_iter().collect();

    for (page_number, page_id) in &ordered {
        let boxes = read_single_page_boxes(document, *page_number, *page_id)?;
        result.push(boxes);
    }
    // Garde l'ordre numérique déjà fourni par BTreeMap.
    let _ = &mut ordered;
    Ok(result)
}

fn read_single_page_boxes(
    document: &Document,
    page_number: u32,
    page_id: lopdf::ObjectId,
) -> Result<PageBoxes, Error> {
    let page_dict = document
        .get_object(page_id)
        .map_err(|e| Error::Pdf(e.to_string()))?
        .as_dict()
        .map_err(|e| Error::Pdf(e.to_string()))?;

    let media = resolve_box(document, page_dict, b"MediaBox")?;
    let crop = resolve_box(document, page_dict, b"CropBox")?;
    let trim = resolve_box(document, page_dict, b"TrimBox")?;
    let bleed = resolve_box(document, page_dict, b"BleedBox")?;
    let art = resolve_box(document, page_dict, b"ArtBox")?;

    Ok(PageBoxes {
        page: page_number,
        media,
        crop,
        trim,
        bleed,
        art,
    })
}

fn resolve_box(
    document: &Document,
    page_dict: &lopdf::Dictionary,
    key: &[u8],
) -> Result<Option<Rect>, Error> {
    if let Ok(obj) = page_dict.get(key) {
        if let Some(rect) = object_to_rect(obj) {
            return Ok(Some(rect));
        }
        if let Ok(id) = obj.as_reference()
            && let Ok(resolved) = document.get_object(id)
            && let Some(rect) = object_to_rect(resolved)
        {
            return Ok(Some(rect));
        }
        return Ok(None);
    }

    if let Ok(parent) = page_dict.get(b"Parent")
        && let Ok(parent_id) = parent.as_reference()
        && let Ok(parent_obj) = document.get_object(parent_id)
        && let Ok(parent_dict) = parent_obj.as_dict()
    {
        return resolve_box(document, parent_dict, key);
    }

    Ok(None)
}

fn object_to_rect(obj: &lopdf::Object) -> Option<Rect> {
    match obj {
        lopdf::Object::Array(items) if items.len() == 4 => {
            let mut values = [0.0f32; 4];
            for (index, item) in items.iter().enumerate() {
                match item {
                    lopdf::Object::Integer(v) => values[index] = *v as f32,
                    lopdf::Object::Real(v) => values[index] = *v,
                    _ => return None,
                }
            }
            Rect::from_array(&values)
        }
        _ => None,
    }
}

pub fn page_media_box(
    document: &Document,
    page_id: lopdf::ObjectId,
) -> Result<Option<Rect>, Error> {
    let page_dict = document
        .get_object(page_id)
        .map_err(|e| Error::Pdf(e.to_string()))?
        .as_dict()
        .map_err(|e| Error::Pdf(e.to_string()))?;
    resolve_box(document, page_dict, b"MediaBox")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_aller_retour() {
        let pt = mm_to_pt(25.4);
        assert!((pt - 72.0).abs() < f64::EPSILON);
        assert!((pt_to_mm(pt) - 25.4).abs() < f64::EPSILON);
    }

    #[test]
    fn rect_dimensions() {
        let rect = Rect {
            llx: 10.0,
            lly: 20.0,
            urx: 40.0,
            ury: 80.0,
        };
        assert!((rect.width() - 30.0).abs() < f64::EPSILON);
        assert!((rect.height() - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn object_to_rect_accepts_integers_and_reals() {
        let obj = lopdf::Object::Array(vec![
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(0),
            lopdf::Object::Real(595.0),
            lopdf::Object::Real(842.0),
        ]);
        let rect = object_to_rect(&obj).expect("rect");
        assert!((rect.width() - 595.0).abs() < f64::EPSILON);
    }
}
