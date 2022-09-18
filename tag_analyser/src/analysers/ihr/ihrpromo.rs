use model::ContentKind;

use crate::analyser::TagAnalyser;

pub struct IhrPromo;

impl TagAnalyser for IhrPromo {
    fn analyse(&self, tags: &model::Tags) -> model::ContentKind {
        const PROMO_PREFIXES: &[&str] = &[
            "Iheart Promo Project",
            "Ihm Promo Product",
            "iHR ",
            "IHTU ",
            "ISWI ",
            "ISWI_",
            "OTPC ",
            "STFB ",
            "INSW-",
            "Podcast Promo ",
        ];

        if tags.get("TrackTitle").map_or(false, |title| {
            PROMO_PREFIXES.iter().any(|p| title.starts_with(p))
        }) {
            ContentKind::Advertisement
        } else {
            ContentKind::Unknown
        }
    }
}
