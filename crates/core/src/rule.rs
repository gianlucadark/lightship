use crate::finding::Finding;
use crate::meta::RuleMeta;
use tl::VDom;

/// Una regola di lint statica sull'HTML già parsato.
///
/// Riceve il `VDom` (parser `tl`, che espone gli offset di sorgente) e il
/// sorgente originale `src`, da cui le regole ricavano lo span esatto del tag.
///
/// È `Sync` perché l'insieme delle regole viene condiviso (per reference) tra i
/// thread di rayon mentre processiamo le pagine in parallelo.
pub trait Rule: Sync {
    fn id(&self) -> &'static str;

    /// Metadati statici della regola (gravità di default, descrizione,
    /// suggerimento di fix, esempi). Usati da report, `rules` ed `explain`.
    fn meta(&self) -> RuleMeta;

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding>;
}
