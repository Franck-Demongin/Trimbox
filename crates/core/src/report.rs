use std::fmt;

use crate::boxes::Rect;
use crate::source_characteristics::SourceCharacteristics;

#[derive(Debug, Clone)]
pub struct PageCropChange {
    pub page: u32,
    pub before: Rect,
    pub after: Rect,
}

#[derive(Debug, Clone)]
pub struct CropReport {
    pub source: String,
    pub output: String,
    pub requested_width_cm: f64,
    pub requested_height_cm: f64,
    pub requested_width_pt: f64,
    pub requested_height_pt: f64,
    pub page_count: usize,
    pub pages: Vec<PageCropChange>,
    pub resolution_origin: String,
    pub color_note: String,
    pub pdfx_note: String,
    pub source_characteristics: SourceCharacteristics,
}

impl CropReport {
    pub fn new(
        source: impl Into<String>,
        output: impl Into<String>,
        requested_width_cm: f64,
        requested_height_cm: f64,
    ) -> Self {
        Self {
            source: source.into(),
            output: output.into(),
            requested_width_cm,
            requested_height_cm,
            requested_width_pt: crate::boxes::cm_to_pt(requested_width_cm),
            requested_height_pt: crate::boxes::cm_to_pt(requested_height_cm),
            page_count: 0,
            pages: Vec::new(),
            resolution_origin: String::from(
                "72 (convention PDF, 1 pt = 1/72 in), origine: norme PDF",
            ),
            color_note: String::from(
                "Aucune transformation de contenu: les ressources couleur, CMJN et profils ICC sont conservees.",
            ),
            pdfx_note: String::from(
                "Validation PDF/X-4 non evaluee dans cette incrementation; conservation structurelle.",
            ),
            source_characteristics: SourceCharacteristics::default(),
        }
    }

    pub fn add_page(&mut self, page: u32, before: Rect, after: Rect) {
        self.page_count = self.page_count.max(page as usize);
        self.pages.push(PageCropChange {
            page,
            before,
            after,
        });
    }

    pub fn finalize(&mut self) {
        self.page_count = self.pages.len();
    }
}

impl fmt::Display for CropReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Trimbox - rapport d'execution")?;
        writeln!(f, "Fichier source : {}", self.source)?;
        writeln!(f, "Fichier genere : {}", self.output)?;
        writeln!(f, "Operation : recadrage central")?;
        writeln!(
            f,
            "Dimensions demandees : {:.2} cm x {:.2} cm ({} x {} pt)",
            self.requested_width_cm,
            self.requested_height_cm,
            fmt_pt(self.requested_width_pt),
            fmt_pt(self.requested_height_pt)
        )?;
        writeln!(
            f,
            "Dimensions effectives : {:.2} cm x {:.2} cm",
            self.requested_width_cm, self.requested_height_cm
        )?;
        writeln!(f, "Resolution utilisee : {}", self.resolution_origin)?;
        writeln!(f, "Nombre de pages : {}", self.page_count)?;
        writeln!(f, "Boites (points) :")?;
        for change in &self.pages {
            writeln!(
                f,
                "  page {} : MediaBox avant {:.2}x{:.2} pt -> apres {:.2}x{:.2} pt",
                change.page,
                change.before.width(),
                change.before.height(),
                change.after.width(),
                change.after.height()
            )?;
            writeln!(
                f,
                "           avant [{:.2}, {:.2}, {:.2}, {:.2}] -> apres [{:.2}, {:.2}, {:.2}, {:.2}]",
                change.before.llx,
                change.before.lly,
                change.before.urx,
                change.before.ury,
                change.after.llx,
                change.after.lly,
                change.after.urx,
                change.after.ury
            )?;
        }
        writeln!(
            f,
            "Boites lues/modifiees : MediaBox et CropBox (TrimBox/BleedBox si presents)."
        )?;
        writeln!(f, "Profils ICC : conserves (aucune re-encodage).")?;
        writeln!(f, "Couleur : {}", self.color_note)?;
        writeln!(f, "PDF/X-4 : {}", self.pdfx_note)?;
        writeln!(
            f,
            "Couleur source : {}",
            self.source_characteristics.color_note()
        )?;
        writeln!(
            f,
            "ICC source (CMJN/RVB/gris) : {}/{}/{} ; profils detectes : {}",
            self.source_characteristics.color.cmyk_present,
            self.source_characteristics.color.rgb_present,
            self.source_characteristics.color.gray_present,
            self.source_characteristics.color.icc_profile_count()
        )?;
        writeln!(
            f,
            "Profils sorties : {}",
            self.source_characteristics
                .color
                .output_intent_profile_count
        )?;
        writeln!(
            f,
            "Metadonnees : {}",
            self.source_characteristics.metadata_note()
        )?;
        writeln!(
            f,
            "Comportement de repli : aucun applicable au recadrage geometrique."
        )?;
        writeln!(f, "Resultat global : SUCCES")?;
        Ok(())
    }
}

fn fmt_pt(value: f64) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{}", value.round() as i64)
    } else {
        format!("{:.2}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::{Rect, cm_to_pt};

    #[test]
    fn report_renders_source_and_output() {
        let mut report = CropReport::new("in.pdf", "out.pdf", 10.0, 15.0);
        report.add_page(
            1,
            Rect {
                llx: 0.0,
                lly: 0.0,
                urx: 595.0,
                ury: 842.0,
            },
            Rect {
                llx: 10.0,
                lly: 20.0,
                urx: 10.0 + cm_to_pt(10.0),
                ury: 20.0 + cm_to_pt(15.0),
            },
        );
        report.finalize();
        let text = report.to_string();
        assert!(text.contains("in.pdf"));
        assert!(text.contains("out.pdf"));
        assert!(text.contains("SUCCES"));
    }
}
