use crate::finding::Finding;
use crate::fix::Fix;
use crate::meta::RuleMeta;
use tl::VDom;

/// A cosa si applica una regola: alcune verificano proprietà **dell'intero
/// documento** (un `<title>`, il `<meta charset>`, un solo `<h1>`…) e hanno
/// senso solo su una **pagina completa**; altre esaminano singoli elementi
/// (`<img>`, `<a>`…) e valgono ovunque, anche nei frammenti/partial.
///
/// L'orchestratore salta le regole `Document` sui file che non sono pagine
/// complete (niente `<html>`/`<head>`/doctype), così un componente o un partial
/// htmx non genera falsi "missing title/charset/viewport".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleScope {
    /// Vale solo su una pagina HTML completa.
    Document,
    /// Vale su qualunque frammento HTML (default).
    Fragment,
}

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

    /// Ambito della regola. Default `Fragment` (vale ovunque); le regole che
    /// controllano proprietà dell'intera pagina la sovrascrivono con `Document`.
    fn scope(&self) -> RuleScope {
        RuleScope::Fragment
    }

    fn check(&self, dom: &VDom<'_>, src: &str) -> Vec<Finding>;

    /// Fix **sicuri e deterministici** per i problemi trovati dalla regola.
    /// Default: nessuno. Le regole lo sovrascrivono solo quando esiste una
    /// correzione non ambigua (aggiungere un attributo, inserire un tag noto);
    /// per i problemi che richiedono contenuto umano non si propone nulla.
    fn fixes(&self, _dom: &VDom<'_>, _src: &str) -> Vec<Fix> {
        Vec::new()
    }
}
